using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using HMCLCore.NET.Util;

namespace HMCLCore.NET.Game;

/// <summary>
/// Parses Mojang-style version JSON (vanilla, Forge, NeoForge, Fabric profiles)
/// into a <see cref="GameInstanceManifest"/>, evaluating platform rules,
/// legacy arguments and version inheritance.
/// </summary>
public static class VersionJsonParser
{
    private const int MaxInheritanceDepth = 8;

    /// <summary>
    /// Parses a version JSON document.
    /// </summary>
    /// <param name="json">The version JSON text.</param>
    /// <param name="parentJsonLoader">
    /// Resolves the JSON of an inherited version by its id; may be null when
    /// inheritance is not supported by the caller.
    /// </param>
    /// <returns>The parsed manifest, or null when the JSON is invalid.</returns>
    public static GameInstanceManifest? Parse(string json, Func<string, string?>? parentJsonLoader = null)
    {
        try
        {
            using var document = JsonDocument.Parse(json);
            return ParseRoot(document.RootElement, parentJsonLoader, 0);
        }
        catch (JsonException)
        {
            return null;
        }
    }

    /// <summary>
    /// Parses one version JSON root, recursively merging its parent when present.
    /// </summary>
    private static GameInstanceManifest? ParseRoot(
        JsonElement root, Func<string, string?>? parentJsonLoader, int depth)
    {
        if (root.ValueKind != JsonValueKind.Object)
            return null;

        var id = GetString(root, "id") ?? GetString(root, "inheritsFrom") ?? string.Empty;
        var mainClass = GetString(root, "mainClass");
        var javaVersion = 0;
        if (root.TryGetProperty("javaVersion", out var javaElement) &&
            javaElement.ValueKind == JsonValueKind.Object &&
            javaElement.TryGetProperty("majorVersion", out var majorElement))
            javaVersion = majorElement.GetInt32();

        var jvmArguments = new List<string>();
        var gameArguments = new List<string>();
        ParseArguments(root, jvmArguments, gameArguments);

        var libraries = ParseLibraries(root);
        var assetIndex = ParseAssetIndex(root);
        var downloads = ParseDownloads(root);
        var logging = ParseLogging(root);

        var child = new GameInstanceManifest
        {
            Version = id,
            MainClass = string.IsNullOrEmpty(mainClass) ? "net.minecraft.client.main.Main" : mainClass,
            JavaVersion = javaVersion,
            Arguments = new GameInstanceManifest.LaunchArguments
            {
                Jvm = jvmArguments,
                Game = gameArguments
            },
            Libraries = libraries,
            AssetIndex = assetIndex,
            Downloads = downloads,
            LoggingConfig = logging
        };

        // Merge the parent version when this JSON inherits from another one.
        var inheritsFrom = GetString(root, "inheritsFrom");
        GameInstanceManifest merged;
        if (string.IsNullOrEmpty(inheritsFrom) ||
            parentJsonLoader == null ||
            depth >= MaxInheritanceDepth)
        {
            merged = child;
        }
        else
        {
            var parentJson = parentJsonLoader(inheritsFrom);
            if (string.IsNullOrEmpty(parentJson))
            {
                merged = child;
            }
            else
            {
                using var parentDocument = JsonDocument.Parse(parentJson);
                var parent = ParseRoot(parentDocument.RootElement, parentJsonLoader, depth + 1);
                if (parent is null)
                {
                    merged = child;
                }
                else
                {
                    var mergedLibraries = new List<Library>(parent.Libraries);
                    var seen = new HashSet<string>(
                        mergedLibraries.Select(l => l.Name), StringComparer.OrdinalIgnoreCase);
                    foreach (var library in child.Libraries)
                    {
                        if (seen.Add(library.Name))
                            mergedLibraries.Add(library);
                    }

                    var mergedJvm = new List<string>(parent.Arguments.Jvm);
                    mergedJvm.AddRange(child.Arguments.Jvm);
                    var mergedGame = new List<string>(parent.Arguments.Game);
                    mergedGame.AddRange(child.Arguments.Game);

                    merged = new GameInstanceManifest
                    {
                        Version = string.IsNullOrEmpty(child.Version) ? parent.Version : child.Version,
                        MainClass = string.IsNullOrEmpty(mainClass) ? parent.MainClass : child.MainClass,
                        JavaVersion = child.JavaVersion > 0 ? child.JavaVersion : parent.JavaVersion,
                        Arguments = new GameInstanceManifest.LaunchArguments
                        {
                            Jvm = mergedJvm,
                            Game = mergedGame
                        },
                        Libraries = mergedLibraries,
                        AssetIndex = child.AssetIndex ?? parent.AssetIndex,
                        Downloads = child.Downloads ?? parent.Downloads,
                        LoggingConfig = child.LoggingConfig ?? parent.LoggingConfig,
                        Demo = parent.Demo,
                        CustomResolution = parent.CustomResolution,
                        Patches = new List<GameInstancePatch>(parent.Patches)
                    };
                }
            }
        }

        // The patches of the version JSON are applied in the order of their
        // priority.
        foreach (var patch in ParsePatches(root).OrderBy(patch => patch.EffectivePriority))
            merged = patch.ApplyTo(merged);

        return merged;
    }

    /// <summary>
    /// Parses the patches array of a version JSON.
    /// </summary>
    /// <param name="root">The root of the version JSON.</param>
    /// <returns>The patches of the version.</returns>
    private static List<GameInstancePatch> ParsePatches(JsonElement root)
    {
        var patches = new List<GameInstancePatch>();
        if (!root.TryGetProperty("patches", out var array) || array.ValueKind != JsonValueKind.Array)
            return patches;

        foreach (var item in array.EnumerateArray())
        {
            if (item.ValueKind != JsonValueKind.Object)
                continue;

            var jvmArguments = new List<string>();
            var gameArguments = new List<string>();
            ParseArguments(item, jvmArguments, gameArguments);

            var javaVersion = 0;
            if (item.TryGetProperty("javaVersion", out var javaElement) &&
                javaElement.ValueKind == JsonValueKind.Object &&
                javaElement.TryGetProperty("majorVersion", out var majorElement))
                javaVersion = majorElement.GetInt32();

            patches.Add(new GameInstancePatch
            {
                Id = GetString(item, "id"),
                Version = GetString(item, "version"),
                Priority = GetInt(item, "priority"),
                MainClass = GetString(item, "mainClass"),
                Libraries = ParseLibraries(item),
                Arguments = new GameInstanceManifest.LaunchArguments
                {
                    Jvm = jvmArguments,
                    Game = gameArguments
                },
                MinecraftArguments = GetString(item, "minecraftArguments"),
                InheritsFrom = GetString(item, "inheritsFrom"),
                Jar = GetString(item, "jar"),
                AssetIndex = ParseAssetIndex(item),
                Assets = GetString(item, "assets"),
                JavaVersion = javaVersion,
                Hidden = item.TryGetProperty("hidden", out var hidden) &&
                         hidden.ValueKind == JsonValueKind.True,
                Type = GetString(item, "type")
            });
        }

        return patches;
    }

    /// <summary>
    /// Parses both modern argument arrays and the legacy Minecraft arguments string.
    /// </summary>
    private static void ParseArguments(JsonElement root, List<string> jvm, List<string> game)
    {
        if (root.TryGetProperty("minecraftArguments", out var legacy) &&
            legacy.ValueKind == JsonValueKind.String &&
            legacy.GetString() is { } legacyText)
        {
            game.AddRange(legacyText.Split(' ', StringSplitOptions.RemoveEmptyEntries));
        }

        if (!root.TryGetProperty("arguments", out var arguments) ||
            arguments.ValueKind != JsonValueKind.Object)
            return;

        if (arguments.TryGetProperty("jvm", out var jvmArray))
            AppendArgumentArray(jvmArray, jvm);
        if (arguments.TryGetProperty("game", out var gameArray))
            AppendArgumentArray(gameArray, game);
    }

    /// <summary>
    /// Appends the allowed entries of an argument array to the target list.
    /// </summary>
    private static void AppendArgumentArray(JsonElement array, List<string> target)
    {
        if (array.ValueKind != JsonValueKind.Array)
            return;

        foreach (var item in array.EnumerateArray())
        {
            if (item.ValueKind == JsonValueKind.String)
            {
                if (item.GetString() is { } value)
                    target.Add(value);
                continue;
            }

            if (item.ValueKind != JsonValueKind.Object || !IsAllowed(item))
                continue;

            if (!item.TryGetProperty("value", out var valueElement))
                continue;

            if (valueElement.ValueKind == JsonValueKind.String)
            {
                if (valueElement.GetString() is { } single)
                    target.Add(single);
            }
            else if (valueElement.ValueKind == JsonValueKind.Array)
            {
                foreach (var entry in valueElement.EnumerateArray())
                {
                    if (entry.ValueKind == JsonValueKind.String && entry.GetString() is { } text)
                        target.Add(text);
                }
            }
        }
    }

    /// <summary>
    /// Evaluates the rules of a rule-bearing JSON object.
    /// </summary>
    private static bool IsAllowed(JsonElement item)
    {
        if (!item.TryGetProperty("rules", out var rules) || rules.ValueKind != JsonValueKind.Array)
            return true;

        var allowed = false;
        foreach (var rule in rules.EnumerateArray())
        {
            if (!RuleMatches(rule))
                continue;

            var action = rule.TryGetProperty("action", out var actionElement)
                ? actionElement.GetString()
                : "allow";
            allowed = string.Equals(action, "allow", StringComparison.OrdinalIgnoreCase);
        }

        return allowed;
    }

    /// <summary>
    /// Returns whether a single rule matches the current platform.
    /// Feature-gated rules are treated as non-matching because the launcher
    /// does not expose demo/custom-resolution/quick-play features.
    /// </summary>
    private static bool RuleMatches(JsonElement rule)
    {
        if (rule.TryGetProperty("features", out _))
            return false;

        if (!rule.TryGetProperty("os", out var os) || os.ValueKind != JsonValueKind.Object)
            return true;

        if (os.TryGetProperty("name", out var nameElement) &&
            nameElement.GetString() is { } name &&
            !name.Equals(Platform.MojangOsName, StringComparison.OrdinalIgnoreCase))
            return false;

        if (os.TryGetProperty("arch", out var archElement) &&
            archElement.GetString() is { } arch)
        {
            var current = Platform.GetBits(Platform.CurrentArch) == Bits.BIT_64 ? "x64" : "x86";
            if (!arch.Equals(current, StringComparison.OrdinalIgnoreCase))
                return false;
        }

        return true;
    }

    /// <summary>
    /// Parses the libraries array into <see cref="Library"/> entries.
    /// </summary>
    private static List<Library> ParseLibraries(JsonElement root)
    {
        var libraries = new List<Library>();
        if (!root.TryGetProperty("libraries", out var array) || array.ValueKind != JsonValueKind.Array)
            return libraries;

        foreach (var item in array.EnumerateArray())
        {
            var name = GetString(item, "name");
            if (string.IsNullOrEmpty(name))
                continue;

            var rules = ParseRules(item);
            var extract = ParseExtract(item);

            libraries.Add(new Library
            {
                Name = name,
                Url = GetString(item, "url"),
                Downloads = ParseLibraryDownloads(item),
                Rules = rules,
                Natives = ParseNatives(item),
                Extract = extract
            });
        }

        return libraries;
    }

    /// <summary>
    /// Parses library download info.
    /// </summary>
    private static LibraryDownloads? ParseLibraryDownloads(JsonElement item)
    {
        if (!item.TryGetProperty("downloads", out var downloads) ||
            downloads.ValueKind != JsonValueKind.Object)
            return null;

        DownloadInfo? artifact = null;
        if (downloads.TryGetProperty("artifact", out var artifactElement))
            artifact = ParseDownloadInfo(artifactElement);

        Dictionary<string, DownloadInfo>? classifiers = null;
        if (downloads.TryGetProperty("classifiers", out var classifiersElement) &&
            classifiersElement.ValueKind == JsonValueKind.Object)
        {
            classifiers = new Dictionary<string, DownloadInfo>(StringComparer.OrdinalIgnoreCase);
            foreach (var property in classifiersElement.EnumerateObject())
            {
                var info = ParseDownloadInfo(property.Value);
                if (info != null)
                    classifiers[property.Name] = info;
            }
        }

        return new LibraryDownloads { Artifact = artifact, Classifiers = classifiers };
    }

    /// <summary>
    /// Parses one download info object.
    /// </summary>
    private static DownloadInfo? ParseDownloadInfo(JsonElement element)
    {
        if (element.ValueKind != JsonValueKind.Object)
            return null;

        var url = GetString(element, "url");
        if (string.IsNullOrEmpty(url))
            return null;

        return new DownloadInfo
        {
            Url = url,
            Sha1 = GetString(element, "sha1") ?? string.Empty,
            Size = element.TryGetProperty("size", out var sizeElement) ? sizeElement.GetInt64() : 0
        };
    }

    /// <summary>
    /// Resolves the native classifier keys for the current operating system.
    /// </summary>
    private static List<string>? ParseNatives(JsonElement item)
    {
        if (!item.TryGetProperty("natives", out var natives) ||
            natives.ValueKind != JsonValueKind.Object)
            return null;

        if (!natives.TryGetProperty(Platform.MojangOsName, out var classifierElement) ||
            classifierElement.GetString() is not { } classifier)
            return null;

        classifier = classifier.Replace("${arch}", Environment.Is64BitProcess ? "64" : "32");
        return new List<string> { classifier };
    }

    /// <summary>
    /// Parses library compatibility rules.
    /// </summary>
    private static List<CompatibilityRule>? ParseRules(JsonElement item)
    {
        if (!item.TryGetProperty("rules", out var rules) || rules.ValueKind != JsonValueKind.Array)
            return null;

        var parsed = new List<CompatibilityRule>();
        foreach (var rule in rules.EnumerateArray())
        {
            parsed.Add(new CompatibilityRule
            {
                Action = GetString(rule, "action") ?? "allow",
                Os = ParseOsRule(rule)
            });
        }

        return parsed.Count > 0 ? parsed : null;
    }

    /// <summary>
    /// Parses the OS condition of a rule.
    /// </summary>
    private static CompatibilityRule.OsRule? ParseOsRule(JsonElement rule)
    {
        if (!rule.TryGetProperty("os", out var os) || os.ValueKind != JsonValueKind.Object)
            return null;

        return new CompatibilityRule.OsRule
        {
            Name = GetString(os, "name"),
            Arch = GetString(os, "arch")
        };
    }

    /// <summary>
    /// Parses native extraction exclude rules.
    /// </summary>
    private static List<ExtractRule>? ParseExtract(JsonElement item)
    {
        if (!item.TryGetProperty("extract", out var extract) ||
            extract.ValueKind != JsonValueKind.Object ||
            !extract.TryGetProperty("exclude", out var exclude) ||
            exclude.ValueKind != JsonValueKind.Array)
            return null;

        var rules = new List<ExtractRule>();
        foreach (var pattern in exclude.EnumerateArray())
        {
            if (pattern.GetString() is { } text)
                rules.Add(new ExtractRule { Pattern = text, Exclude = true });
        }

        return rules.Count > 0 ? rules : null;
    }

    /// <summary>
    /// Parses the asset index info.
    /// </summary>
    private static GameInstanceManifest.AssetIndexInfo? ParseAssetIndex(JsonElement root)
    {
        if (!root.TryGetProperty("assetIndex", out var assetIndex) ||
            assetIndex.ValueKind != JsonValueKind.Object)
            return null;

        return new GameInstanceManifest.AssetIndexInfo
        {
            Id = GetString(assetIndex, "id") ?? string.Empty,
            Sha1 = GetString(assetIndex, "sha1") ?? string.Empty,
            Size = assetIndex.TryGetProperty("size", out var size) ? size.GetInt64() : 0,
            TotalSize = assetIndex.TryGetProperty("totalSize", out var total) ? total.GetInt64() : 0,
            Url = GetString(assetIndex, "url") ?? string.Empty
        };
    }

    /// <summary>
    /// Parses the client JAR download info.
    /// </summary>
    private static GameInstanceManifest.GameDownloads? ParseDownloads(JsonElement root)
    {
        if (!root.TryGetProperty("downloads", out var downloads) ||
            downloads.ValueKind != JsonValueKind.Object)
            return null;

        GameInstanceManifest.ClientFile? client = null;
        if (downloads.TryGetProperty("client", out var clientElement))
        {
            var url = GetString(clientElement, "url");
            if (!string.IsNullOrEmpty(url))
            {
                client = new GameInstanceManifest.ClientFile
                {
                    Url = url,
                    Sha1 = GetString(clientElement, "sha1") ?? string.Empty,
                    Size = clientElement.TryGetProperty("size", out var size) ? size.GetInt64() : 0
                };
            }
        }

        return client == null ? null : new GameInstanceManifest.GameDownloads { Client = client };
    }

    /// <summary>
    /// Parses the logging configuration.
    /// </summary>
    private static LoggingInfo? ParseLogging(JsonElement root)
    {
        if (!root.TryGetProperty("logging", out var logging) ||
            logging.ValueKind != JsonValueKind.Object ||
            !logging.TryGetProperty("client", out var client) ||
            client.ValueKind != JsonValueKind.Object)
            return null;

        var file = client.TryGetProperty("file", out var fileElement)
            ? GetString(fileElement, "id")
            : null;

        return new LoggingInfo
        {
            Type = GetString(client, "type") ?? string.Empty,
            File = file
        };
    }

    /// <summary>
    /// Reads an optional string property.
    /// </summary>
    private static string? GetString(JsonElement element, string propertyName) =>
        element.TryGetProperty(propertyName, out var value) && value.ValueKind == JsonValueKind.String
            ? value.GetString()
            : null;

    /// <summary>
    /// Reads an optional integer property.
    /// </summary>
    private static int? GetInt(JsonElement element, string propertyName) =>
        element.TryGetProperty(propertyName, out var value) && value.ValueKind == JsonValueKind.Number &&
        value.TryGetInt32(out var number)
            ? number
            : null;
}
