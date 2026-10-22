using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Threading;
using System.Threading.Tasks;
using HMCLCore.NET.Game;
using HMCLCore.NET.Tasks;

namespace HMCLCore.NET.Download;

/// <summary>
/// Extracts the native library JAR into the version's natives directory,
/// honoring the extract rules (excluded patterns) of the library.
/// </summary>
public sealed class NativeExtractTask : TaskBase
{
    private readonly string _nativeJarPath;
    private readonly string _nativesDirectory;
    private readonly IReadOnlyList<ExtractRule>? _extractRules;

    /// <summary>
    /// Creates a new NativeExtractTask.
    /// </summary>
    /// <param name="nativeJarPath">Path of the natives JAR to extract.</param>
    /// <param name="nativesDirectory">Directory receiving the extracted files.</param>
    /// <param name="extractRules">The library's extract rules, may be null.</param>
    public NativeExtractTask(string nativeJarPath, string nativesDirectory, IReadOnlyList<ExtractRule>? extractRules)
    {
        _nativeJarPath = nativeJarPath ?? throw new ArgumentNullException(nameof(nativeJarPath));
        _nativesDirectory = nativesDirectory ?? throw new ArgumentNullException(nameof(nativesDirectory));
        _extractRules = extractRules;
        Name = "解压原生库";
    }

    /// <summary>
    /// Extracts every JAR entry except the ones excluded by the extract rules.
    /// Directory entries are skipped; missing parent directories are created.
    /// </summary>
    public override Task Run(CancellationToken token = default)
    {
        token.ThrowIfCancellationRequested();

        if (!File.Exists(_nativeJarPath))
            throw new FileNotFoundException("Native library JAR not found", _nativeJarPath);

        Directory.CreateDirectory(_nativesDirectory);

        using var archive = ZipFile.OpenRead(_nativeJarPath);
        var total = 0L;
        foreach (var entry in archive.Entries)
        {
            token.ThrowIfCancellationRequested();

            // Zip archives represent directories with a trailing slash or empty name.
            if (string.IsNullOrEmpty(entry.Name))
                continue;

            if (IsExcluded(entry.FullName))
                continue;

            var destination = Path.Combine(_nativesDirectory, entry.FullName);
            Directory.CreateDirectory(Path.GetDirectoryName(destination)!);

            using var entryStream = entry.Open();
            using var fileStream = File.Create(destination);
            entryStream.CopyTo(fileStream);

            total++;
        }

        OnProgressChanged(total, total);
        return Task.CompletedTask;
    }

    /// <summary>
    /// Returns whether an archive entry matches an exclusion pattern.
    /// Patterns ending with a slash match directory prefixes; other patterns
    /// match anywhere within the entry's full name.
    /// </summary>
    private bool IsExcluded(string entryName)
    {
        if (_extractRules == null)
            return false;

        foreach (var rule in _extractRules)
        {
            if (!rule.Exclude || string.IsNullOrEmpty(rule.Pattern))
                continue;

            var pattern = rule.Pattern.Replace('/', Path.DirectorySeparatorChar);
            if (pattern.EndsWith(Path.DirectorySeparatorChar) ?
                entryName.StartsWith(pattern, StringComparison.OrdinalIgnoreCase) :
                entryName.Contains(pattern, StringComparison.OrdinalIgnoreCase))
            {
                return true;
            }
        }

        return false;
    }
}
