using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.ComponentModel;
using System.IO;
using System.Runtime.CompilerServices;
using HMCLCore.NET.Game;
using HMCLCore.NET.Nbt;
using HMCL.NET.Util;

namespace HMCL.NET.ViewModels;

/// <summary>
/// A single node in the NBT tag tree, suitable for binding to a TreeView.
/// </summary>
public sealed class NbtTreeNode : INotifyPropertyChanged
{
    public event PropertyChangedEventHandler? PropertyChanged;

    private string _displayValue = string.Empty;
    private bool _isEditing;
    private string _editText = string.Empty;

    /// <summary>
    /// The tag name, or empty for unnamed tags (e.g. list entries).
    /// </summary>
    public string Name { get; }

    /// <summary>
    /// The tag type name (e.g. "Compound", "List", "String").
    /// </summary>
    public string TagType { get; }

    /// <summary>
    /// Child nodes for compound and list tags.
    /// </summary>
    public ObservableCollection<NbtTreeNode> Children { get; } = new();

    /// <summary>
    /// Whether this node holds an editable scalar value.
    /// </summary>
    public bool IsEditable { get; }

    /// <summary>
    /// The human-readable value shown in the UI.
    /// </summary>
    public string DisplayValue
    {
        get => _displayValue;
        set => SetField(ref _displayValue, value);
    }

    /// <summary>
    /// Whether the node is currently in edit mode.
    /// </summary>
    public bool IsEditing
    {
        get => _isEditing;
        set => SetField(ref _isEditing, value);
    }

    /// <summary>
    /// Text entered while editing the value.
    /// </summary>
    public string EditText
    {
        get => _editText;
        set => SetField(ref _editText, value);
    }

    /// <summary>
    /// The underlying NBT tag.
    /// </summary>
    public NbtTag Tag { get; }

    public NbtTreeNode(NbtTag tag, string name)
    {
        Tag = tag;
        Name = name;
        TagType = tag.TagType;

        switch (tag)
        {
            case NbtCompoundTag:
            case NbtListTag:
                IsEditable = false;
                DisplayValue = $"({Children.Count} children)";
                break;
            case NbtByteTag bt:
                IsEditable = true;
                DisplayValue = bt.Value.ToString();
                EditText = bt.Value.ToString();
                break;
            case NbtShortTag st:
                IsEditable = true;
                DisplayValue = st.Value.ToString();
                EditText = st.Value.ToString();
                break;
            case NbtIntTag it:
                IsEditable = true;
                DisplayValue = it.Value.ToString();
                EditText = it.Value.ToString();
                break;
            case NbtLongTag lt:
                IsEditable = true;
                DisplayValue = lt.Value.ToString();
                EditText = lt.Value.ToString();
                break;
            case NbtFloatTag ft:
                IsEditable = true;
                DisplayValue = ft.Value.ToString();
                EditText = ft.Value.ToString();
                break;
            case NbtDoubleTag dt:
                IsEditable = true;
                DisplayValue = dt.Value.ToString();
                EditText = dt.Value.ToString();
                break;
            case NbtStringTag str:
                IsEditable = true;
                DisplayValue = str.Value;
                EditText = str.Value;
                break;
            case NbtByteArrayTag bat:
                IsEditable = true;
                DisplayValue = $"byte[{bat.Value.Length}]";
                EditText = $"byte[{bat.Value.Length}]";
                break;
            case NbtIntArrayTag iat:
                IsEditable = true;
                DisplayValue = $"int[{iat.Value.Length}]";
                EditText = $"int[{iat.Value.Length}]";
                break;
            case NbtLongArrayTag lat:
                IsEditable = true;
                DisplayValue = $"long[{lat.Value.Length}]";
                EditText = $"long[{lat.Value.Length}]";
                break;
            case NbtEndTag:
                IsEditable = false;
                DisplayValue = "(end)";
                break;
            default:
                IsEditable = false;
                DisplayValue = "(unknown)";
                break;
        }

        PopulateChildren(tag);
    }

    private void PopulateChildren(NbtTag tag)
    {
        switch (tag)
        {
            case NbtCompoundTag compound:
                foreach (var kvp in compound.Children)
                    Children.Add(new NbtTreeNode(kvp.Value, kvp.Key));
                break;
            case NbtListTag list:
                for (int i = 0; i < list.Children.Count; i++)
                    Children.Add(new NbtTreeNode(list.Children[i], $"[{i}]"));
                break;
        }

        if (tag is NbtCompoundTag or NbtListTag)
            DisplayValue = $"({Children.Count} children)";
    }

    private bool SetField<T>(ref T field, T value, [CallerMemberName] string? propertyName = null)
    {
        if (Equals(field, value)) return false;
        field = value;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
        return true;
    }
}

/// <summary>
/// View model for the NBT editor dialog. Loads an NBT file and exposes its
/// tag tree for editing.
/// </summary>
public sealed class NbtEditorViewModel : INotifyPropertyChanged
{
    public event PropertyChangedEventHandler? PropertyChanged;

    private string _filePath = string.Empty;
    private string _statusMessage = string.Empty;
    private NbtTreeNode? _rootNode;
    private bool _hasChanges;

    /// <summary>
    /// The path of the NBT file being edited.
    /// </summary>
    public string FilePath
    {
        get => _filePath;
        set => SetField(ref _filePath, value);
    }

    /// <summary>
    /// Status message shown at the bottom of the dialog.
    /// </summary>
    public string StatusMessage
    {
        get => _statusMessage;
        set => SetField(ref _statusMessage, value);
    }

    /// <summary>
    /// The root node of the NBT tag tree.
    /// </summary>
    public NbtTreeNode? RootNode
    {
        get => _rootNode;
        set => SetField(ref _rootNode, value);
    }

    /// <summary>
    /// Whether any value has been modified but not saved.
    /// </summary>
    public bool HasChanges
    {
        get => _hasChanges;
        set => SetField(ref _hasChanges, value);
    }

    /// <summary>
    /// Loads an NBT file from the given path.
    /// </summary>
    /// <param name="path">The .nbt or level.dat file path.</param>
    /// <returns>True if the file was loaded successfully.</returns>
    public bool Load(string path)
    {
        try
        {
            FilePath = path;
            var nbtFile = NbtFile.Load(path);
            if (nbtFile is null)
            {
                StatusMessage = Lang.Get("hmcl.nbteditorviewmodel.818_");
                return false;
            }

            RootNode = new NbtTreeNode(nbtFile.Root, string.Empty);
            HasChanges = false;
            StatusMessage = $"已加载: {Path.GetFileName(path)}";
            return true;
        }
        catch (Exception ex)
        {
            StatusMessage = $"加载失败: {ex.Message}";
            return false;
        }
    }

    /// <summary>
    /// Saves the edited NBT tree back to the file.
    /// </summary>
    /// <returns>True if the file was saved successfully.</returns>
    public bool Save()
    {
        if (RootNode is null || string.IsNullOrEmpty(FilePath))
            return false;

        try
        {
            // Rebuild the NbtFile from the edited tree and save.
            var nbtFile = new NbtFile { Root = BuildTag(RootNode) };
            nbtFile.Save(FilePath);
            HasChanges = false;
            StatusMessage = Lang.Get("hmcl.nbteditorviewmodel.820_");
            return true;
        }
        catch (Exception ex)
        {
            StatusMessage = $"保存失败: {ex.Message}";
            return false;
        }
    }

    private static NbtTag BuildTag(NbtTreeNode node)
    {
        NbtTag tag = node.Tag switch
        {
            NbtByteTag => new NbtByteTag { Value = ParseByte(node.EditText) },
            NbtShortTag => new NbtShortTag { Value = ParseShort(node.EditText) },
            NbtIntTag => new NbtIntTag { Value = ParseInt(node.EditText) },
            NbtLongTag => new NbtLongTag { Value = ParseLong(node.EditText) },
            NbtFloatTag => new NbtFloatTag { Value = ParseFloat(node.EditText) },
            NbtDoubleTag => new NbtDoubleTag { Value = ParseDouble(node.EditText) },
            NbtStringTag => new NbtStringTag { Value = node.EditText },
            NbtCompoundTag => BuildCompoundTag(node),
            NbtListTag => BuildListTag(node),
            _ => node.Tag
        };
        tag.Name = node.Name;
        return tag;
    }

    private static NbtCompoundTag BuildCompoundTag(NbtTreeNode node)
    {
        var compound = new NbtCompoundTag { Name = node.Name };
        foreach (var child in node.Children)
            compound.Children[child.Name ?? string.Empty] = BuildTag(child);
        return compound;
    }

    private static NbtListTag BuildListTag(NbtTreeNode node)
    {
        var list = new NbtListTag { Name = node.Name };
        foreach (var child in node.Children)
            list.Children.Add(BuildTag(child));
        return list;
    }

    private static byte ParseByte(string text) => byte.TryParse(text, out var v) ? v : (byte)0;
    private static short ParseShort(string text) => short.TryParse(text, out var v) ? v : (short)0;
    private static int ParseInt(string text) => int.TryParse(text, out var v) ? v : 0;
    private static long ParseLong(string text) => long.TryParse(text, out var v) ? v : 0L;
    private static float ParseFloat(string text) => float.TryParse(text, out var v) ? v : 0f;
    private static double ParseDouble(string text) => double.TryParse(text, out var v) ? v : 0d;

    private bool SetField<T>(ref T field, T value, [CallerMemberName] string? propertyName = null)
    {
        if (Equals(field, value)) return false;
        field = value;
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
        return true;
    }
}
