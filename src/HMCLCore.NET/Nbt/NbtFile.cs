using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Text;

namespace HMCLCore.NET.Nbt;

/// <summary>
/// Service for reading and writing Minecraft NBT files.
/// Supports the standard NBT binary format used by level.dat, region files, etc.
/// </summary>
public sealed class NbtFile
{
    /// <summary>
    /// Loads an NBT file from the specified path.
    /// </summary>
    /// <param name="path">The file path to load.</param>
    /// <returns>The parsed NBT file, or null if the file does not exist.</returns>
    public static NbtFile? Load(string path)
    {
        if (!File.Exists(path))
            return null;

        using var fs = File.OpenRead(path);
        return Load(fs);
    }

    /// <summary>
    /// Loads an NBT file from a stream.
    /// </summary>
    /// <param name="stream">The stream containing NBT data.</param>
    /// <returns>The parsed NBT file.</returns>
    public static NbtFile Load(Stream stream)
    {
        // The files of Minecraft, like the level data of the worlds, are
        // compressed with gzip; the plain format is accepted as well.
        var buffered = new MemoryStream();
        stream.CopyTo(buffered);
        buffered.Position = 0;

        var header = new byte[2];
        var read = buffered.Read(header, 0, 2);
        buffered.Position = 0;

        Stream source = buffered;
        GZipStream? gzip = null;

        if (read == 2 && header[0] == 0x1F && header[1] == 0x8B)
        {
            gzip = new GZipStream(buffered, CompressionMode.Decompress);
            source = gzip;
        }

        try
        {
            using var reader = new BinaryReader(source);
            return new NbtFile(ReadTag(reader, out _));
        }
        finally
        {
            gzip?.Dispose();
        }
    }

    /// <summary>
    /// Saves the NBT file to the specified path.
    /// </summary>
    /// <param name="path">The destination file path.</param>
    /// <param name="compressed">
    /// Whether the file is compressed with gzip, like the files of Minecraft.
    /// </param>
    public void Save(string path, bool compressed = true)
    {
        using var fs = File.Create(path);

        if (!compressed)
        {
            using var writer = new BinaryWriter(fs);
            WriteTag(writer, Root);
            return;
        }

        using var gzip = new GZipStream(fs, CompressionLevel.Optimal);
        using var compressedWriter = new BinaryWriter(gzip);
        WriteTag(compressedWriter, Root);
    }

    /// <summary>
    /// The root tag of the NBT file.
    /// </summary>
    public NbtTag Root { get; set; }

    /// <summary>
    /// Creates a new empty NBT file.
    /// </summary>
    public NbtFile()
    {
        Root = new NbtCompoundTag { Name = "" };
    }

    private NbtFile(NbtTag root)
    {
        Root = root;
    }

    private static NbtTag ReadTag(BinaryReader reader, out string? name)
    {
        var type = (NbtTagType)reader.ReadByte();

        // The end tag of a compound has no name.
        if (type == NbtTagType.End)
        {
            name = null;
            return new NbtEndTag();
        }

        name = ReadString(reader);

        return type switch
        {
            NbtTagType.Byte => new NbtByteTag { Value = reader.ReadByte() },
            NbtTagType.Short => new NbtShortTag { Value = reader.ReadInt16() },
            NbtTagType.Int => new NbtIntTag { Value = reader.ReadInt32() },
            NbtTagType.Long => new NbtLongTag { Value = reader.ReadInt64() },
            NbtTagType.Float => new NbtFloatTag { Value = reader.ReadSingle() },
            NbtTagType.Double => new NbtDoubleTag { Value = reader.ReadDouble() },
            NbtTagType.ByteArray => new NbtByteArrayTag { Value = reader.ReadBytes(reader.ReadInt32()) },
            NbtTagType.String => new NbtStringTag { Value = ReadString(reader) },
            NbtTagType.List => ReadListTag(reader),
            NbtTagType.Compound => ReadCompoundTag(reader),
            NbtTagType.IntArray => new NbtIntArrayTag { Value = ReadIntArray(reader) },
            NbtTagType.LongArray => new NbtLongArrayTag { Value = ReadLongArray(reader) },
            _ => throw new InvalidDataException($"Unknown NBT tag type: {type}")
        };
    }

    private static string ReadString(BinaryReader reader)
    {
        var length = reader.ReadInt16();
        var bytes = reader.ReadBytes(length);
        return Encoding.UTF8.GetString(bytes);
    }

    private static NbtListTag ReadListTag(BinaryReader reader)
    {
        var elementType = (NbtTagType)reader.ReadByte();
        var count = reader.ReadInt32();
        var list = new NbtListTag { ElementType = elementType };
        for (var i = 0; i < count; i++)
        {
            // The elements of a list have no name; only their payload is
            // read.
            list.Children.Add(ReadTagPayload(reader, elementType));
        }
        return list;
    }

    /// <summary>
    /// Reads the payload of a tag of the given type.
    /// </summary>
    /// <param name="reader">The reader of the NBT data.</param>
    /// <param name="type">The type of the tag.</param>
    /// <returns>The tag read from the data.</returns>
    private static NbtTag ReadTagPayload(BinaryReader reader, NbtTagType type) => type switch
    {
        NbtTagType.End => new NbtEndTag(),
        NbtTagType.Byte => new NbtByteTag { Value = reader.ReadByte() },
        NbtTagType.Short => new NbtShortTag { Value = reader.ReadInt16() },
        NbtTagType.Int => new NbtIntTag { Value = reader.ReadInt32() },
        NbtTagType.Long => new NbtLongTag { Value = reader.ReadInt64() },
        NbtTagType.Float => new NbtFloatTag { Value = reader.ReadSingle() },
        NbtTagType.Double => new NbtDoubleTag { Value = reader.ReadDouble() },
        NbtTagType.ByteArray => new NbtByteArrayTag { Value = reader.ReadBytes(reader.ReadInt32()) },
        NbtTagType.String => new NbtStringTag { Value = ReadString(reader) },
        NbtTagType.List => ReadListTag(reader),
        NbtTagType.Compound => ReadCompoundTag(reader),
        NbtTagType.IntArray => new NbtIntArrayTag { Value = ReadIntArray(reader) },
        NbtTagType.LongArray => new NbtLongArrayTag { Value = ReadLongArray(reader) },
        _ => throw new InvalidDataException($"Unknown NBT tag type: {type}")
    };

    private static NbtCompoundTag ReadCompoundTag(BinaryReader reader)
    {
        var compound = new NbtCompoundTag();
        while (true)
        {
            var tag = ReadTag(reader, out var name);
            if (tag is NbtEndTag)
                break;
            tag.Name = name;
            compound.Children[tag.Name ?? string.Empty] = tag;
        }
        return compound;
    }

    private static int[] ReadIntArray(BinaryReader reader)
    {
        var length = reader.ReadInt32();
        var result = new int[length];
        for (var i = 0; i < length; i++)
            result[i] = reader.ReadInt32();
        return result;
    }

    private static long[] ReadLongArray(BinaryReader reader)
    {
        var length = reader.ReadInt32();
        var result = new long[length];
        for (var i = 0; i < length; i++)
            result[i] = reader.ReadInt64();
        return result;
    }

    private static void WriteTag(BinaryWriter writer, NbtTag tag)
    {
        writer.Write((byte)GetTagType(tag));
        WriteString(writer, tag.Name);
        WriteTagPayload(writer, tag);
    }

    private static NbtTagType GetTagType(NbtTag tag)
    {
        return tag switch
        {
            NbtEndTag => NbtTagType.End,
            NbtByteTag => NbtTagType.Byte,
            NbtShortTag => NbtTagType.Short,
            NbtIntTag => NbtTagType.Int,
            NbtLongTag => NbtTagType.Long,
            NbtFloatTag => NbtTagType.Float,
            NbtDoubleTag => NbtTagType.Double,
            NbtByteArrayTag => NbtTagType.ByteArray,
            NbtStringTag => NbtTagType.String,
            NbtListTag => NbtTagType.List,
            NbtCompoundTag => NbtTagType.Compound,
            NbtIntArrayTag => NbtTagType.IntArray,
            NbtLongArrayTag => NbtTagType.LongArray,
            _ => throw new InvalidOperationException($"Unknown NBT tag type: {tag.GetType()}")
        };
    }

    private static void WriteString(BinaryWriter writer, string? value)
    {
        var bytes = Encoding.UTF8.GetBytes(value ?? string.Empty);
        writer.Write((short)bytes.Length);
        writer.Write(bytes);
    }

    private static void WriteTagPayload(BinaryWriter writer, NbtTag tag)
    {
        switch (tag)
        {
            case NbtByteTag bt:
                writer.Write(bt.Value);
                break;
            case NbtShortTag st:
                writer.Write(st.Value);
                break;
            case NbtIntTag it:
                writer.Write(it.Value);
                break;
            case NbtLongTag lt:
                writer.Write(lt.Value);
                break;
            case NbtFloatTag ft:
                writer.Write(ft.Value);
                break;
            case NbtDoubleTag dt:
                writer.Write(dt.Value);
                break;
            case NbtByteArrayTag bat:
                writer.Write(bat.Value.Length);
                writer.Write(bat.Value);
                break;
            case NbtStringTag str:
                WriteString(writer, str.Value);
                break;
            case NbtListTag list:
                writer.Write((byte)list.ElementType);
                writer.Write(list.Children.Count);
                // The elements of a list have no name; only their payload is
                // written.
                foreach (var child in list.Children)
                    WriteTagPayload(writer, child);
                break;
            case NbtCompoundTag compound:
                foreach (var kvp in compound.Children)
                {
                    // The name of a member of a compound is its key; the name
                    // of the tag itself is not used.
                    kvp.Value.Name = kvp.Key;
                    WriteTag(writer, kvp.Value);
                }
                writer.Write((byte)NbtTagType.End);
                break;
            case NbtIntArrayTag iat:
                writer.Write(iat.Value.Length);
                foreach (var v in iat.Value)
                    writer.Write(v);
                break;
            case NbtLongArrayTag lat:
                writer.Write(lat.Value.Length);
                foreach (var v in lat.Value)
                    writer.Write(v);
                break;
            default:
                throw new InvalidOperationException($"Unknown NBT tag type: {tag.GetType()}");
        }
    }
}

/// <summary>
/// NBT tag type identifiers.
/// </summary>
public enum NbtTagType : byte
{
    End = 0,
    Byte = 1,
    Short = 2,
    Int = 3,
    Long = 4,
    Float = 5,
    Double = 6,
    ByteArray = 7,
    String = 8,
    List = 9,
    Compound = 10,
    IntArray = 11,
    LongArray = 12
}

/// <summary>
/// Base class for all NBT tags.
/// </summary>
public abstract class NbtTag
{
    /// <summary>
    /// The tag name, or null for unnamed tags (e.g. list entries).
    /// </summary>
    public string? Name { get; set; }

    /// <summary>
    /// The tag type name (e.g. "Compound", "List", "String").
    /// </summary>
    public virtual string TagType => GetType().Name.StartsWith("Nbt")
        ? GetType().Name[3..].Replace("Tag", string.Empty)
        : GetType().Name;

    protected NbtTag()
    {
    }
}

/// <summary>
/// An end tag, used to terminate compound tags.
/// </summary>
public sealed class NbtEndTag : NbtTag
{
}

/// <summary>
/// A byte (8-bit) tag.
/// </summary>
public sealed class NbtByteTag : NbtTag
{
    public byte Value { get; set; }
}

/// <summary>
/// A short (16-bit) tag.
/// </summary>
public sealed class NbtShortTag : NbtTag
{
    public short Value { get; set; }
}

/// <summary>
/// An int (32-bit) tag.
/// </summary>
public sealed class NbtIntTag : NbtTag
{
    public int Value { get; set; }
}

/// <summary>
/// A long (64-bit) tag.
/// </summary>
public sealed class NbtLongTag : NbtTag
{
    public long Value { get; set; }
}

/// <summary>
/// A float (32-bit) tag.
/// </summary>
public sealed class NbtFloatTag : NbtTag
{
    public float Value { get; set; }
}

/// <summary>
/// A double (64-bit) tag.
/// </summary>
public sealed class NbtDoubleTag : NbtTag
{
    public double Value { get; set; }
}

/// <summary>
/// A byte array tag.
/// </summary>
public sealed class NbtByteArrayTag : NbtTag
{
    public byte[] Value { get; set; } = Array.Empty<byte>();
}

/// <summary>
/// A string tag.
/// </summary>
public sealed class NbtStringTag : NbtTag
{
    public string Value { get; set; } = string.Empty;
}

/// <summary>
/// A list tag containing ordered child tags.
/// </summary>
public sealed class NbtListTag : NbtTag
{
    /// <summary>
    /// The NBT type of elements in this list.
    /// </summary>
    public NbtTagType ElementType { get; set; } = NbtTagType.End;

    /// <summary>
    /// The child tags in order.
    /// </summary>
    public List<NbtTag> Children { get; } = new();
}

/// <summary>
/// A compound tag containing named child tags.
/// </summary>
public sealed class NbtCompoundTag : NbtTag
{
    /// <summary>
    /// The child tags, keyed by name.
    /// </summary>
    public Dictionary<string, NbtTag> Children { get; } = new(StringComparer.Ordinal);
}

/// <summary>
/// An int array tag.
/// </summary>
public sealed class NbtIntArrayTag : NbtTag
{
    public int[] Value { get; set; } = Array.Empty<int>();
}

/// <summary>
/// A long array tag.
/// </summary>
public sealed class NbtLongArrayTag : NbtTag
{
    public long[] Value { get; set; } = Array.Empty<long>();
}
