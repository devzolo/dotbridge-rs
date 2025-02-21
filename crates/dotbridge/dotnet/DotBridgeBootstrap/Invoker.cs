using System.Reflection;
using System.Runtime.InteropServices;
using System.Text;

namespace DotBridgeBootstrap;

/// <summary>
/// Handles invocation of compiled .NET methods from Rust.
/// All [UnmanagedCallersOnly] methods are called directly from Rust via hostfxr function pointers.
/// </summary>
public static class Invoker
{
    /// <summary>
    /// Load a pre-compiled assembly and get a GCHandle to a ManagedInvoker.
    /// Called from Rust to load assemblies.
    /// </summary>
    [UnmanagedCallersOnly]
    public static unsafe int GetFunc(
        byte* assemblyPathPtr, int assemblyPathLen,
        byte* typeNamePtr, int typeNameLen,
        byte* methodNamePtr, int methodNameLen,
        void** resultPtr,
        byte** errorPtr, int* errorLen)
    {
        try
        {
            var assemblyPath = Encoding.UTF8.GetString(assemblyPathPtr, assemblyPathLen);
            var typeName = Encoding.UTF8.GetString(typeNamePtr, typeNameLen);
            var methodName = Encoding.UTF8.GetString(methodNamePtr, methodNameLen);

            var fullPath = Path.GetFullPath(assemblyPath);
            var assemblyDir = Path.GetDirectoryName(fullPath);
            var alc = new DotBridgeAssemblyLoadContext(assemblyDir);
            var assembly = alc.LoadFromAssemblyPath(fullPath);

            var type = assembly.GetType(typeName)
                ?? throw new Exception($"Type '{typeName}' not found in assembly '{assemblyPath}'");

            var method = type.GetMethod(methodName, BindingFlags.Public | BindingFlags.Instance)
                ?? type.GetMethod(methodName, BindingFlags.Public | BindingFlags.Static)
                ?? throw new Exception($"Method '{methodName}' not found on type '{typeName}'");

            var instance = method.IsStatic ? null : Activator.CreateInstance(type);
            var invoker = new ManagedInvoker(instance, method);
            var handle = GCHandle.Alloc(invoker);
            *resultPtr = (void*)GCHandle.ToIntPtr(handle);
            return 0;
        }
        catch (Exception ex)
        {
            var unwrapped = WireProtocol.UnwrapException(ex);
            WriteError(unwrapped.ToString(), errorPtr, errorLen);
            return -1;
        }
    }

    /// <summary>
    /// Invoke a managed function identified by a GCHandle.
    /// Called from Rust via UnmanagedCallersOnly.
    /// On error, returns -1 and writes WireProtocol-serialized exception data
    /// so the Rust side can parse structured { Message, StackTrace, Name } fields.
    /// </summary>
    [UnmanagedCallersOnly]
    public static unsafe int Invoke(
        void* funcHandle,
        byte* inputPtr, int inputLen,
        byte** resultPtr, int* resultLen)
    {
        try
        {
            var handle = GCHandle.FromIntPtr((IntPtr)funcHandle);
            var invoker = (ManagedInvoker)handle.Target!;

            object? input = null;
            if (inputPtr != null && inputLen > 0)
            {
                var inputData = new ReadOnlySpan<byte>(inputPtr, inputLen);
                input = WireProtocol.Deserialize(inputData);
            }

            // Run the async method synchronously
            var result = invoker.InvokeAsync(input).GetAwaiter().GetResult();

            var serialized = WireProtocol.Serialize(result);
            var resBuf = (byte*)Marshal.AllocHGlobal(serialized.Length);
            Marshal.Copy(serialized, 0, (IntPtr)resBuf, serialized.Length);
            *resultPtr = resBuf;
            *resultLen = serialized.Length;

            return 0;
        }
        catch (Exception ex)
        {
            var unwrapped = WireProtocol.UnwrapException(ex);
            // Serialize the exception as a structured WireProtocol object
            var serialized = WireProtocol.Serialize(unwrapped);
            var errBuf = (byte*)Marshal.AllocHGlobal(serialized.Length);
            Marshal.Copy(serialized, 0, (IntPtr)errBuf, serialized.Length);
            *resultPtr = errBuf;
            *resultLen = serialized.Length;
            return -1;
        }
    }

    /// <summary>
    /// Free a GCHandle previously returned by GetFunc or CompileFunc.
    /// Called from Rust when a DotBridgeFunc is dropped.
    /// </summary>
    [UnmanagedCallersOnly]
    public static unsafe void FreeHandle(void* funcHandle)
    {
        if (funcHandle != null)
        {
            var handle = GCHandle.FromIntPtr((IntPtr)funcHandle);
            handle.Free();
        }
    }

    /// <summary>
    /// Free memory allocated by Invoke/GetFunc via Marshal.AllocHGlobal.
    /// </summary>
    [UnmanagedCallersOnly]
    public static unsafe void Free(byte* ptr, int len)
    {
        if (ptr != null)
            Marshal.FreeHGlobal((IntPtr)ptr);
    }

    private static unsafe void WriteError(string message, byte** errorPtr, int* errorLen)
    {
        var errorBytes = Encoding.UTF8.GetBytes(message);
        var errBuf = (byte*)Marshal.AllocHGlobal(errorBytes.Length);
        Marshal.Copy(errorBytes, 0, (IntPtr)errBuf, errorBytes.Length);
        *errorPtr = errBuf;
        *errorLen = errorBytes.Length;
    }
}
