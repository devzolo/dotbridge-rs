using System.Collections.Concurrent;
using System.Dynamic;
using System.Linq.Expressions;
using System.Reflection;
using System.Text;

namespace DotBridgeBootstrap;

/// <summary>
/// Binary wire protocol for marshaling data between Rust and .NET.
/// Matches the ClrValue serialization format in dotbridge's marshal.rs.
/// </summary>
public static class WireProtocol
{
    private const int TagFunction = 1;
    private const int TagBuffer = 2;
    private const int TagArray = 3;
    private const int TagDate = 4;
    private const int TagObject = 5;
    private const int TagString = 6;
    private const int TagBoolean = 7;
    private const int TagInt32 = 8;
    private const int TagUInt32 = 9;
    private const int TagNumber = 10;
    private const int TagNull = 11;
    private const int TagException = 13;

    /// <summary>
    /// .NET DateTime ticks for 1970-01-01T00:00:00Z (Unix epoch).
    /// Used for precise DateTime/milliseconds conversion matching edge-js.
    /// </summary>
    private const long MinDateTimeTicks = 621355968000000000;

    /// <summary>
    /// Cached property/field accessors per type for performance.
    /// Uses compiled Expression trees for fast access.
    /// </summary>
    private static readonly ConcurrentDictionary<Type, PropertyAccessor[]> TypeAccessorCache = new();

    /// <summary>
    /// Namespaces whose types should be blocked for security.
    /// Returns empty object instead of reflecting properties.
    /// </summary>
    private static readonly HashSet<string> BlockedTypeNamespaces =
    [
        "System.Reflection",
    ];

    public static byte[] Serialize(object? value)
    {
        using var ms = new MemoryStream();
        using var writer = new BinaryWriter(ms);
        SerializeValue(writer, value);
        return ms.ToArray();
    }

    private static void SerializeValue(BinaryWriter writer, object? value)
    {
        switch (value)
        {
            case null:
                writer.Write(TagNull);
                break;
            case string s:
                WriteString(writer, s);
                break;
            case bool b:
                writer.Write(TagBoolean);
                writer.Write(b ? (byte)1 : (byte)0);
                break;
            case int i:
                writer.Write(TagInt32);
                writer.Write(i);
                break;
            case short s:
                writer.Write(TagInt32);
                writer.Write((int)s);
                break;
            case ushort u:
                writer.Write(TagInt32);
                writer.Write((int)u);
                break;
            case uint u:
                writer.Write(TagUInt32);
                writer.Write(u);
                break;
            case long l:
                writer.Write(TagNumber);
                writer.Write((double)l);
                break;
            case ulong ul:
                writer.Write(TagNumber);
                writer.Write((double)ul);
                break;
            case float f:
                writer.Write(TagNumber);
                writer.Write((double)f);
                break;
            case double d:
                writer.Write(TagNumber);
                writer.Write(d);
                break;
            case decimal dec:
                WriteString(writer, dec.ToString(System.Globalization.CultureInfo.InvariantCulture));
                break;
            case DateTime dt:
                writer.Write(TagDate);
                var ticks = dt.ToUniversalTime().Ticks;
                writer.Write((double)(ticks - MinDateTimeTicks) / 10000.0);
                break;
            case DateTimeOffset dto:
                writer.Write(TagDate);
                var dtoTicks = dto.UtcTicks;
                writer.Write((double)(dtoTicks - MinDateTimeTicks) / 10000.0);
                break;
            case Guid g:
                WriteString(writer, g.ToString());
                break;
            case Uri uri:
                WriteString(writer, uri.ToString());
                break;
            case char c:
                WriteString(writer, c.ToString());
                break;
            case Enum e:
                WriteString(writer, e.ToString());
                break;
            case byte[] buf:
                writer.Write(TagBuffer);
                writer.Write(buf.Length);
                writer.Write(buf);
                break;
            case Exception ex:
                SerializeException(writer, ex);
                break;
            case Func<object?, Task<object?>> func:
                var gcHandle = System.Runtime.InteropServices.GCHandle.Alloc(func);
                writer.Write(TagFunction);
                writer.Write((ulong)(long)System.Runtime.InteropServices.GCHandle.ToIntPtr(gcHandle));
                break;
            case IDictionary<string, object?> dict:
                writer.Write(TagObject);
                writer.Write(dict.Count);
                foreach (var kvp in dict)
                {
                    WriteStringBytes(writer, kvp.Key);
                    SerializeValue(writer, kvp.Value);
                }
                break;
            case System.Collections.IList list:
                writer.Write(TagArray);
                writer.Write(list.Count);
                foreach (var item in list)
                    SerializeValue(writer, item);
                break;
            case IEnumerable<byte> byteEnum:
                var byteArr = byteEnum.ToArray();
                writer.Write(TagBuffer);
                writer.Write(byteArr.Length);
                writer.Write(byteArr);
                break;
            default:
                SerializeReflectedObject(writer, value);
                break;
        }
    }

    private static void SerializeException(BinaryWriter writer, Exception ex)
    {
        var unwrapped = UnwrapException(ex);
        var props = new Dictionary<string, object?>
        {
            ["Message"] = unwrapped.Message,
            ["StackTrace"] = unwrapped.StackTrace,
            ["Name"] = unwrapped.GetType().FullName,
            ["Source"] = unwrapped.Source,
        };
        if (unwrapped.InnerException != null)
        {
            props["InnerException"] = new Dictionary<string, object?>
            {
                ["Message"] = unwrapped.InnerException.Message,
                ["Name"] = unwrapped.InnerException.GetType().FullName,
            };
        }
        writer.Write(TagObject);
        writer.Write(props.Count);
        foreach (var kvp in props)
        {
            WriteStringBytes(writer, kvp.Key);
            SerializeValue(writer, kvp.Value);
        }
    }

    private static void SerializeReflectedObject(BinaryWriter writer, object value)
    {
        var type = value.GetType();
        if (IsBlockedType(type))
        {
            writer.Write(TagObject);
            writer.Write(0);
            return;
        }
        var accessors = GetCachedAccessors(type);
        writer.Write(TagObject);
        writer.Write(accessors.Length);
        foreach (var accessor in accessors)
        {
            WriteStringBytes(writer, accessor.Name);
            try { SerializeValue(writer, accessor.GetValue(value)); }
            catch { SerializeValue(writer, null); }
        }
    }

    private static void WriteString(BinaryWriter writer, string s)
    {
        writer.Write(TagString);
        WriteStringBytes(writer, s);
    }

    private static void WriteStringBytes(BinaryWriter writer, string s)
    {
        var bytes = Encoding.UTF8.GetBytes(s);
        writer.Write(bytes.Length);
        writer.Write(bytes);
    }

    // =========================================================================
    // Deserialization
    // =========================================================================

    public static object? Deserialize(ReadOnlySpan<byte> data)
    {
        int offset = 0;
        return DeserializeValue(data, ref offset);
    }

    private static object? DeserializeValue(ReadOnlySpan<byte> data, ref int offset)
    {
        var tag = BitConverter.ToInt32(data.Slice(offset, 4));
        offset += 4;
        return tag switch
        {
            TagNull => null,
            TagString => ReadString(data, ref offset),
            TagBoolean => ReadBool(data, ref offset),
            TagInt32 => ReadInt32(data, ref offset),
            TagUInt32 => ReadUInt32(data, ref offset),
            TagNumber => ReadDouble(data, ref offset),
            TagDate => ReadDateTime(data, ref offset),
            TagBuffer => ReadBuffer(data, ref offset),
            TagArray => ReadArray(data, ref offset),
            TagObject => ReadExpandoObject(data, ref offset),
            TagFunction => ReadCallbackId(data, ref offset),
            _ => throw new Exception($"Unknown type tag: {tag}")
        };
    }

    private static string ReadString(ReadOnlySpan<byte> data, ref int offset)
    {
        var len = BitConverter.ToInt32(data.Slice(offset, 4));
        offset += 4;
        var s = Encoding.UTF8.GetString(data.Slice(offset, len));
        offset += len;
        return s;
    }

    private static bool ReadBool(ReadOnlySpan<byte> data, ref int offset)
    {
        var b = data[offset] != 0;
        offset += 1;
        return b;
    }

    private static int ReadInt32(ReadOnlySpan<byte> data, ref int offset)
    {
        var n = BitConverter.ToInt32(data.Slice(offset, 4));
        offset += 4;
        return n;
    }

    private static uint ReadUInt32(ReadOnlySpan<byte> data, ref int offset)
    {
        var n = BitConverter.ToUInt32(data.Slice(offset, 4));
        offset += 4;
        return n;
    }

    private static double ReadDouble(ReadOnlySpan<byte> data, ref int offset)
    {
        var n = BitConverter.ToDouble(data.Slice(offset, 8));
        offset += 8;
        return n;
    }

    private static DateTime ReadDateTime(ReadOnlySpan<byte> data, ref int offset)
    {
        var ms = BitConverter.ToDouble(data.Slice(offset, 8));
        offset += 8;
        var dtTicks = (long)(ms * 10000.0) + MinDateTimeTicks;
        return new DateTime(dtTicks, DateTimeKind.Utc);
    }

    private static byte[] ReadBuffer(ReadOnlySpan<byte> data, ref int offset)
    {
        var len = BitConverter.ToInt32(data.Slice(offset, 4));
        offset += 4;
        var buf = data.Slice(offset, len).ToArray();
        offset += len;
        return buf;
    }

    private static object?[] ReadArray(ReadOnlySpan<byte> data, ref int offset)
    {
        var count = BitConverter.ToInt32(data.Slice(offset, 4));
        offset += 4;
        var arr = new object?[count];
        for (int i = 0; i < count; i++)
            arr[i] = DeserializeValue(data, ref offset);
        return arr;
    }

    /// <summary>
    /// Deserialize objects as ExpandoObject for dynamic access from C#.
    /// </summary>
    private static ExpandoObject ReadExpandoObject(ReadOnlySpan<byte> data, ref int offset)
    {
        var count = BitConverter.ToInt32(data.Slice(offset, 4));
        offset += 4;
        var expando = new ExpandoObject();
        var dict = (IDictionary<string, object?>)expando;
        for (int i = 0; i < count; i++)
        {
            var key = ReadString(data, ref offset);
            var val = DeserializeValue(data, ref offset);
            dict[key] = val;
        }
        return expando;
    }

    private static ulong ReadCallbackId(ReadOnlySpan<byte> data, ref int offset)
    {
        var id = BitConverter.ToUInt64(data.Slice(offset, 8));
        offset += 8;
        return id;
    }

    // =========================================================================
    // Exception unwrapping
    // =========================================================================

    /// <summary>
    /// Unwrap AggregateException and TargetInvocationException to get the real inner exception.
    /// </summary>
    public static Exception UnwrapException(Exception ex)
    {
        while (true)
        {
            switch (ex)
            {
                case AggregateException ae when ae.InnerExceptions.Count == 1:
                    ex = ae.InnerExceptions[0];
                    continue;
                case TargetInvocationException tie when tie.InnerException != null:
                    ex = tie.InnerException;
                    continue;
                default:
                    return ex;
            }
        }
    }

    // =========================================================================
    // Cached reflection via Expression trees
    // =========================================================================

    private static PropertyAccessor[] GetCachedAccessors(Type type)
    {
        return TypeAccessorCache.GetOrAdd(type, static t =>
        {
            var result = new List<PropertyAccessor>();
            var props = t.GetProperties(BindingFlags.Public | BindingFlags.Instance);
            foreach (var prop in props)
            {
                if (!prop.CanRead) continue;
                var getter = CompileGetter(t, prop);
                result.Add(new PropertyAccessor(prop.Name, getter));
            }
            var fields = t.GetFields(BindingFlags.Public | BindingFlags.Instance);
            foreach (var field in fields)
            {
                var getter = CompileFieldGetter(t, field);
                result.Add(new PropertyAccessor(field.Name, getter));
            }
            return result.ToArray();
        });
    }

    private static Func<object, object?> CompileGetter(Type type, PropertyInfo prop)
    {
        var param = Expression.Parameter(typeof(object), "obj");
        var cast = Expression.Convert(param, type);
        var access = Expression.Property(cast, prop);
        var boxed = Expression.Convert(access, typeof(object));
        return Expression.Lambda<Func<object, object?>>(boxed, param).Compile();
    }

    private static Func<object, object?> CompileFieldGetter(Type type, FieldInfo field)
    {
        var param = Expression.Parameter(typeof(object), "obj");
        var cast = Expression.Convert(param, type);
        var access = Expression.Field(cast, field);
        var boxed = Expression.Convert(access, typeof(object));
        return Expression.Lambda<Func<object, object?>>(boxed, param).Compile();
    }

    private static bool IsBlockedType(Type type)
    {
        var ns = type.Namespace;
        if (ns == null) return false;
        foreach (var blocked in BlockedTypeNamespaces)
        {
            if (ns.StartsWith(blocked, StringComparison.Ordinal))
                return true;
        }
        return false;
    }

    private sealed class PropertyAccessor(string name, Func<object, object?> getter)
    {
        public string Name { get; } = name;
        public object? GetValue(object obj) => getter(obj);
    }
}
