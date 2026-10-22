using System;
using System.Collections.Generic;
using HMCLCore.NET.Setting;
using HMCLCore.NET.Util;

namespace HMCLCore.NET.Download;

/// <summary>
/// A rule rewriting the address of an official source to the address of a
/// mirror.
/// </summary>
/// <param name="Source">The prefix of the addresses of the official source.</param>
/// <param name="Target">The prefix of the addresses of the mirror.</param>
/// <param name="Fallback">
/// Whether the official address is preferred even when the mirror is the
/// preferred one; used by the mirrors that are known to be incomplete.
/// </param>
public sealed record MirrorRule(string Source, string Target, bool Fallback = false);

/// <summary>
/// Chooses the addresses of the downloads, following the download sources
/// chosen by the user, like the download providers of Java HMCL.
/// </summary>
public static class MirrorSource
{
    /// <summary>
    /// The default address of the BMCLAPI mirror.
    /// </summary>
    public const string DefaultBmclapiRoot = "https://bmclapi2.bangbang93.com";

    private static readonly object _lock = new();

    private static string _mirrorRoot = DefaultBmclapiRoot;
    private static IReadOnlyList<MirrorRule> _rules = BuildRules(DefaultBmclapiRoot);

    /// <summary>
    /// The source used to download the version lists.
    /// </summary>
    public static DownloadSource VersionListSource { get; set; } = DownloadSource.Default;

    /// <summary>
    /// The source used to download the files of the game.
    /// </summary>
    public static DownloadSource FileSource { get; set; } = DownloadSource.Default;

    /// <summary>
    /// The address of the BMCLAPI mirror.
    /// </summary>
    public static string MirrorRoot
    {
        get
        {
            lock (_lock)
                return _mirrorRoot;
        }
        set
        {
            lock (_lock)
            {
                _mirrorRoot = value.TrimEnd('/');
                _rules = BuildRules(_mirrorRoot);
            }
        }
    }

    /// <summary>
    /// The rules rewriting the addresses of the official sources to the
    /// addresses of the mirrors.
    /// </summary>
    public static IReadOnlyList<MirrorRule> Rules
    {
        get
        {
            lock (_lock)
                return _rules;
        }
    }

    /// <summary>
    /// Whether the machine is located in the mainland of China; the mirror is
    /// preferred automatically on such machines.
    /// </summary>
    public static bool IsChinaMainland => RegionUtils.IsChinaMainland;

    /// <summary>
    /// Sets the sources used to download the version lists and the files.
    /// </summary>
    /// <param name="versionListSource">The source of the version lists.</param>
    /// <param name="fileSource">The source of the files.</param>
    public static void SetSources(DownloadSource versionListSource, DownloadSource fileSource)
    {
        VersionListSource = versionListSource;
        FileSource = fileSource;
    }

    /// <summary>
    /// Sets the sources from the name of a download provider stored in the
    /// settings of the launcher.
    /// </summary>
    /// <remarks>
    /// The names containing "BMCLAPI" select the mirror; the other names select
    /// the official sources.
    /// </remarks>
    /// <param name="providerName">The name of the download provider.</param>
    public static void SetFromProviderName(string? providerName)
    {
        var source = providerName?.Contains("BMCLAPI", StringComparison.OrdinalIgnoreCase) == true
            ? DownloadSource.Mirror
            : DownloadSource.Official;

        SetSources(source, source);
    }

    /// <summary>
    /// Returns the addresses of a file of the game, ordered by the preference
    /// of the user; the first address is the preferred one.
    /// </summary>
    /// <param name="url">The address of the official source.</param>
    /// <returns>The addresses of the file, the preferred one first.</returns>
    public static IReadOnlyList<string> GetFileCandidates(string? url)
        => GetCandidates(FileSource, url);

    /// <summary>
    /// Returns the addresses of a version list, ordered by the preference of
    /// the user; the first address is the preferred one.
    /// </summary>
    /// <param name="url">The address of the official source.</param>
    /// <returns>The addresses of the version list, the preferred one first.</returns>
    public static IReadOnlyList<string> GetVersionListCandidates(string? url)
        => GetCandidates(VersionListSource, url);

    /// <summary>
    /// Returns the address of a file of the game, following the source chosen
    /// by the user.
    /// </summary>
    /// <param name="url">The address of the official source.</param>
    /// <returns>The address of the file to download.</returns>
    public static string ReplaceFile(string? url)
        => GetFileCandidates(url)[0];

    /// <summary>
    /// Returns the address of a version list, following the source chosen by
    /// the user.
    /// </summary>
    /// <param name="url">The address of the official source.</param>
    /// <returns>The address of the version list to download.</returns>
    public static string ReplaceVersionList(string? url)
        => GetVersionListCandidates(url)[0];

    /// <summary>
    /// Returns the address of a file of the game, following the source chosen
    /// by the user.
    /// </summary>
    /// <param name="url">The address of the official source.</param>
    /// <returns>The address of the file to download.</returns>
    public static string Replace(string? url)
        => ReplaceFile(url);

    /// <summary>
    /// Returns the addresses of a file of the game as URIs, ordered by the
    /// preference of the user; the first address is the preferred one.
    /// </summary>
    /// <param name="url">The address of the official source.</param>
    /// <returns>The addresses of the file, the preferred one first.</returns>
    public static IReadOnlyList<Uri> GetFileUris(string? url)
        => ToUris(GetFileCandidates(url));

    /// <summary>
    /// Returns the addresses of a version list as URIs, ordered by the
    /// preference of the user; the first address is the preferred one.
    /// </summary>
    /// <param name="url">The address of the official source.</param>
    /// <returns>The addresses of the version list, the preferred one first.</returns>
    public static IReadOnlyList<Uri> GetVersionListUris(string? url)
        => ToUris(GetVersionListCandidates(url));

    /// <summary>
    /// Converts addresses into URIs, skipping the invalid ones.
    /// </summary>
    /// <param name="urls">The addresses to convert.</param>
    /// <returns>The URIs of the valid addresses.</returns>
    private static IReadOnlyList<Uri> ToUris(IReadOnlyList<string> urls)
    {
        var uris = new List<Uri>(urls.Count);
        foreach (var url in urls)
        {
            if (Uri.TryCreate(url, UriKind.Absolute, out var uri))
                uris.Add(uri);
        }

        return uris;
    }

    /// <summary>
    /// Returns the addresses of a resource, ordered by the preference of the
    /// user; the first address is the preferred one.
    /// </summary>
    /// <param name="url">The address of the official source.</param>
    /// <returns>The addresses of the resource, the preferred one first.</returns>
    public static IReadOnlyList<string> GetCandidates(DownloadSource source, string? url)
    {
        if (string.IsNullOrEmpty(url))
            return new[] { url ?? string.Empty };

        var official = url;
        var mirror = ApplyRules(url);

        if (string.Equals(mirror, official, StringComparison.Ordinal))
            return new[] { official };

        // Outside the mainland of China the mirror is only used when the user
        // asked for it; inside it the mirror is preferred by default.
        var mirrorFirst = IsChinaMainland
            ? source != DownloadSource.Official
            : source == DownloadSource.Mirror;

        return mirrorFirst
            ? new[] { mirror, official }
            : new[] { official, mirror };
    }

    /// <summary>
    /// Rewrites an address of an official source to the address of the mirror
    /// matching it.
    /// </summary>
    /// <param name="url">The address of the official source.</param>
    /// <returns>The address of the mirror, or the given address when no
    /// mirror matches it.</returns>
    private static string ApplyRules(string url)
    {
        foreach (var rule in Rules)
        {
            if (url.StartsWith(rule.Source, StringComparison.Ordinal))
                return rule.Target + url.Substring(rule.Source.Length);
        }

        return url;
    }

    /// <summary>
    /// Builds the rules rewriting the addresses of the official sources to the
    /// addresses of the given mirror.
    /// </summary>
    /// <param name="root">The address of the mirror.</param>
    /// <returns>The rules of the mirror.</returns>
    private static IReadOnlyList<MirrorRule> BuildRules(string root)
    {
        root = root.TrimEnd('/');

        return new[]
        {
            new MirrorRule("https://bmclapi2.bangbang93.com", root),
            new MirrorRule("https://launchermeta.mojang.com", root),
            new MirrorRule("https://piston-meta.mojang.com", root),
            new MirrorRule("https://piston-data.mojang.com", root),
            new MirrorRule("https://launcher.mojang.com", root),
            new MirrorRule("https://resources.download.minecraft.net", root + "/assets"),
            new MirrorRule("https://libraries.minecraft.net", root + "/libraries"),
            new MirrorRule("http://files.minecraftforge.net/maven", root + "/maven"),
            new MirrorRule("https://files.minecraftforge.net/maven", root + "/maven"),
            new MirrorRule("https://maven.minecraftforge.net", root + "/maven"),
            new MirrorRule("https://maven.neoforged.net/releases/", root + "/maven/"),
            new MirrorRule("http://dl.liteloader.com/versions", root + "/maven"),
            new MirrorRule("https://dl.liteloader.com/versions", root + "/maven"),
            new MirrorRule("https://meta.fabricmc.net", root + "/fabric-meta"),
            new MirrorRule("https://maven.fabricmc.net", root + "/maven"),
            new MirrorRule("https://authlib-injector.yushi.moe", root + "/mirrors/authlib-injector"),
            new MirrorRule("https://repo1.maven.org/maven2", "https://mirrors.cloud.tencent.com/nexus/repository/maven-public"),
            new MirrorRule("https://repo.maven.apache.org/maven2", "https://mirrors.cloud.tencent.com/nexus/repository/maven-public"),
            new MirrorRule("https://api.modrinth.com", "https://mod.mcimirror.top/modrinth", true),
            new MirrorRule("https://cdn.modrinth.com", "https://mod.mcimirror.top", true),
            new MirrorRule("https://api.curseforge.com", "https://mod.mcimirror.top/curseforge", true),
            new MirrorRule("https://edge.forgecdn.net", "https://mod.mcimirror.top", true)
        };
    }
}
