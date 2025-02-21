using System.Reflection;
using System.Runtime.Loader;

namespace DotBridgeBootstrap;

/// <summary>
/// Custom AssemblyLoadContext for isolating dynamically loaded assemblies.
/// Provides assembly resolution with fallback to deps.json, NuGet cache, and custom paths.
/// </summary>
internal class DotBridgeAssemblyLoadContext : AssemblyLoadContext
{
    private readonly string? _basePath;
    private readonly AssemblyDependencyResolver? _resolver;
    private readonly List<string> _additionalProbePaths = new();

    public DotBridgeAssemblyLoadContext(string? basePath = null, string? name = null)
        : base(name ?? $"DotBridge_{Guid.NewGuid():N}", isCollectible: true)
    {
        _basePath = basePath;

        // If we have a base path with a .deps.json, use the built-in resolver
        if (basePath != null)
        {
            var depsFile = FindDepsJson(basePath);
            if (depsFile != null)
            {
                _resolver = new AssemblyDependencyResolver(depsFile);
            }

            _additionalProbePaths.Add(basePath);
        }

        // Add NuGet global packages folder as a probe path
        var nugetPath = GetNuGetPackagesPath();
        if (nugetPath != null)
            _additionalProbePaths.Add(nugetPath);
    }

    /// <summary>
    /// Add an additional directory to search for assemblies.
    /// </summary>
    public void AddProbePath(string path)
    {
        if (Directory.Exists(path) && !_additionalProbePaths.Contains(path))
            _additionalProbePaths.Add(path);
    }

    protected override Assembly? Load(AssemblyName assemblyName)
    {
        // 1. Try the deps.json-based resolver first
        if (_resolver != null)
        {
            var resolvedPath = _resolver.ResolveAssemblyToPath(assemblyName);
            if (resolvedPath != null)
                return LoadFromAssemblyPath(resolvedPath);
        }

        // 2. Probe additional paths
        foreach (var probePath in _additionalProbePaths)
        {
            var candidate = Path.Combine(probePath, $"{assemblyName.Name}.dll");
            if (File.Exists(candidate))
                return LoadFromAssemblyPath(candidate);
        }

        // 3. Fall back to default context (shared framework assemblies)
        return null;
    }

    protected override IntPtr LoadUnmanagedDll(string unmanagedDllName)
    {
        // 1. Try the deps.json-based resolver
        if (_resolver != null)
        {
            var resolvedPath = _resolver.ResolveUnmanagedDllToPath(unmanagedDllName);
            if (resolvedPath != null)
                return LoadUnmanagedDllFromPath(resolvedPath);
        }

        // 2. Probe additional paths with platform-specific naming
        foreach (var probePath in _additionalProbePaths)
        {
            foreach (var candidate in GetNativeLibraryCandidates(probePath, unmanagedDllName))
            {
                if (File.Exists(candidate))
                    return LoadUnmanagedDllFromPath(candidate);
            }
        }

        return IntPtr.Zero;
    }

    private static string? FindDepsJson(string basePath)
    {
        // Look for *.deps.json in the base path
        if (Directory.Exists(basePath))
        {
            var depsFiles = Directory.GetFiles(basePath, "*.deps.json");
            if (depsFiles.Length > 0)
                return depsFiles[0]; // Use the first one found
        }

        // Check if basePath is itself a DLL — look for deps.json next to it
        if (File.Exists(basePath) && basePath.EndsWith(".dll", StringComparison.OrdinalIgnoreCase))
        {
            var dir = Path.GetDirectoryName(basePath)!;
            var name = Path.GetFileNameWithoutExtension(basePath);
            var depsFile = Path.Combine(dir, $"{name}.deps.json");
            if (File.Exists(depsFile))
                return depsFile;
        }

        return null;
    }

    private static string? GetNuGetPackagesPath()
    {
        // Check NUGET_PACKAGES env var first
        var envPath = Environment.GetEnvironmentVariable("NUGET_PACKAGES");
        if (!string.IsNullOrEmpty(envPath) && Directory.Exists(envPath))
            return envPath;

        // Default NuGet global-packages path
        var userProfile = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        var defaultPath = Path.Combine(userProfile, ".nuget", "packages");
        if (Directory.Exists(defaultPath))
            return defaultPath;

        return null;
    }

    private static IEnumerable<string> GetNativeLibraryCandidates(string basePath, string name)
    {
        if (OperatingSystem.IsWindows())
        {
            yield return Path.Combine(basePath, $"{name}.dll");
            yield return Path.Combine(basePath, name);
            yield return Path.Combine(basePath, "runtimes", "win-x64", "native", $"{name}.dll");
        }
        else if (OperatingSystem.IsLinux())
        {
            yield return Path.Combine(basePath, $"lib{name}.so");
            yield return Path.Combine(basePath, $"{name}.so");
            yield return Path.Combine(basePath, name);
            yield return Path.Combine(basePath, "runtimes", "linux-x64", "native", $"lib{name}.so");
        }
        else if (OperatingSystem.IsMacOS())
        {
            yield return Path.Combine(basePath, $"lib{name}.dylib");
            yield return Path.Combine(basePath, $"{name}.dylib");
            yield return Path.Combine(basePath, name);
            yield return Path.Combine(basePath, "runtimes", "osx-x64", "native", $"lib{name}.dylib");
            yield return Path.Combine(basePath, "runtimes", "osx-arm64", "native", $"lib{name}.dylib");
        }
    }
}
