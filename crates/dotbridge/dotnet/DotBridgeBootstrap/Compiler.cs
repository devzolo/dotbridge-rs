using System.Reflection;
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;

namespace DotBridgeBootstrap;

/// <summary>
/// Compiles inline C# source code into callable delegates.
/// Called from Rust via hostfxr function pointer mechanism.
/// </summary>
public static class Compiler
{
    private static readonly string[] DefaultUsings =
    [
        "System",
        "System.Collections.Generic",
        "System.Linq",
        "System.Text",
        "System.Threading.Tasks",
        "System.Dynamic",
    ];

    private static readonly List<MetadataReference> DefaultReferences;

    static Compiler()
    {
        DefaultReferences = new List<MetadataReference>();

        // Add references to core assemblies
        var trustedAssemblies = AppContext.GetData("TRUSTED_PLATFORM_ASSEMBLIES") as string;
        if (trustedAssemblies != null)
        {
            foreach (var path in trustedAssemblies.Split(Path.PathSeparator))
            {
                var name = Path.GetFileNameWithoutExtension(path);
                if (name.StartsWith("System.") || name == "mscorlib" || name == "netstandard")
                {
                    DefaultReferences.Add(MetadataReference.CreateFromFile(path));
                }
            }
        }
    }

    /// <summary>
    /// Compile C# source and return a function pointer.
    /// Entry point called from Rust via UnmanagedCallersOnly convention.
    /// </summary>
    [UnmanagedCallersOnly]
    public static unsafe int CompileFunc(
        byte* source, int sourceLen,
        byte* references, int referencesLen,
        void** resultPtr,
        byte** errorPtr, int* errorLen)
    {
        try
        {
            var sourceStr = Encoding.UTF8.GetString(source, sourceLen);
            var wrappedSource = WrapSource(sourceStr);

            var syntaxTree = CSharpSyntaxTree.ParseText(wrappedSource);

            // Build reference list: defaults + any additional paths from Rust
            var allReferences = new List<MetadataReference>(DefaultReferences);
            if (references != null && referencesLen > 0)
            {
                var refStr = Encoding.UTF8.GetString(references, referencesLen);
                foreach (var refPath in refStr.Split(';', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries))
                {
                    if (File.Exists(refPath))
                        allReferences.Add(MetadataReference.CreateFromFile(refPath));
                }
            }

            var compilation = CSharpCompilation.Create(
                assemblyName: $"DotBridgeDynamic_{Guid.NewGuid():N}",
                syntaxTrees: [syntaxTree],
                references: allReferences,
                options: new CSharpCompilationOptions(OutputKind.DynamicallyLinkedLibrary)
                    .WithOptimizationLevel(OptimizationLevel.Release));

            using var ms = new MemoryStream();
            var result = compilation.Emit(ms);

            if (!result.Success)
            {
                var errors = string.Join("\n", result.Diagnostics
                    .Where(d => d.Severity == DiagnosticSeverity.Error)
                    .Select(d => d.ToString()));

                var errorBytes = Encoding.UTF8.GetBytes(errors);
                var errBuf = (byte*)Marshal.AllocHGlobal(errorBytes.Length);
                Marshal.Copy(errorBytes, 0, (IntPtr)errBuf, errorBytes.Length);
                *errorPtr = errBuf;
                *errorLen = errorBytes.Length;
                return -1;
            }

            ms.Seek(0, SeekOrigin.Begin);
            var alc = new DotBridgeAssemblyLoadContext(name: $"DotBridgeDynamic_{Guid.NewGuid():N}");
            var assembly = alc.LoadFromStream(ms);

            var type = assembly.GetType("Startup")
                ?? throw new Exception("Compiled assembly does not contain a 'Startup' type");

            var method = type.GetMethod("Invoke", BindingFlags.Public | BindingFlags.Instance)
                ?? type.GetMethod("Invoke", BindingFlags.Public | BindingFlags.Static)
                ?? throw new Exception("'Startup' type does not contain an 'Invoke' method");

            // Create a delegate and get its function pointer
            var instance = method.IsStatic ? null : Activator.CreateInstance(type);
            var invoker = new ManagedInvoker(instance, method);
            var handle = GCHandle.Alloc(invoker);
            *resultPtr = (void*)GCHandle.ToIntPtr(handle);

            return 0;
        }
        catch (Exception ex)
        {
            var unwrapped = WireProtocol.UnwrapException(ex);
            var errorBytes = Encoding.UTF8.GetBytes(unwrapped.ToString());
            var errBuf = (byte*)Marshal.AllocHGlobal(errorBytes.Length);
            Marshal.Copy(errorBytes, 0, (IntPtr)errBuf, errorBytes.Length);
            *errorPtr = errBuf;
            *errorLen = errorBytes.Length;
            return -2;
        }
    }

    private static string WrapSource(string source)
    {
        var trimmed = source.Trim();

        // If it's already a complete class, use as-is
        if (trimmed.Contains("class ") && trimmed.Contains("Invoke"))
        {
            return trimmed;
        }

        // If it's a lambda expression, wrap it in a Startup class
        var usings = string.Join("\n", DefaultUsings.Select(u => $"using {u};"));

        return $$"""
            {{usings}}

            public class Startup
            {
                public async Task<object?> Invoke(object? input)
                {
                    var func = new Func<object?, Task<object?>>({{trimmed}});
                    return await func(input);
                }
            }
            """;
    }
}

/// <summary>
/// Wraps a managed method for invocation from Rust.
/// </summary>
internal class ManagedInvoker
{
    private readonly object? _instance;
    private readonly MethodInfo _method;

    public ManagedInvoker(object? instance, MethodInfo method)
    {
        _instance = instance;
        _method = method;
    }

    public async Task<object?> InvokeAsync(object? input)
    {
        var result = _method.Invoke(_instance, [input]);
        if (result is Task<object?> task)
            return await task;
        if (result is Task voidTask)
        {
            await voidTask;
            return null;
        }
        return result;
    }
}
