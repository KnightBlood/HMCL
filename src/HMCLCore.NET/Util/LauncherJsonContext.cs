using System.Collections.Generic;
using System.Text.Json.Serialization;
using HMCLCore.NET.Addon;
using HMCLCore.NET.Auth;
using HMCLCore.NET.Download;
using HMCLCore.NET.Game;
using HMCLCore.NET.Modpack;

namespace HMCLCore.NET.Util;

/// <summary>
/// The metadata of the JSON documents read and written by the launcher,
/// generated when the launcher is compiled, so that the launcher can be
/// published with the native ahead-of-time compiler.
/// </summary>
/// <remarks>
/// The documents of the network are read and written with the same metadata,
/// so the property names of the documents are matched without taking the case
/// into account.
/// </remarks>
[JsonSourceGenerationOptions(PropertyNameCaseInsensitive = true, WriteIndented = true)]
[JsonSerializable(typeof(GameInstanceManifest))]
[JsonSerializable(typeof(PackMcMeta))]
[JsonSerializable(typeof(ModrinthSearchResponse))]
[JsonSerializable(typeof(ModrinthProject))]
[JsonSerializable(typeof(ModrinthVersion))]
[JsonSerializable(typeof(List<ModrinthVersion>))]
[JsonSerializable(typeof(CurseForgeSearchResponse))]
[JsonSerializable(typeof(CurseForgeModResponse))]
[JsonSerializable(typeof(CurseForgeFilesResponse))]
[JsonSerializable(typeof(CurseForgeDownloadUrlResponse))]
[JsonSerializable(typeof(MicrosoftTokens))]
[JsonSerializable(typeof(YggdrasilResponse))]
[JsonSerializable(typeof(YggdrasilAuthRequest))]
[JsonSerializable(typeof(YggdrasilRefreshRequest))]
[JsonSerializable(typeof(Auth.Microsoft.XboxTokenResponse))]
[JsonSerializable(typeof(Auth.Microsoft.XboxTokenRequest))]
[JsonSerializable(typeof(Auth.Microsoft.MinecraftLoginResponse))]
[JsonSerializable(typeof(Auth.Microsoft.MinecraftIdentityRequest))]
[JsonSerializable(typeof(Auth.Microsoft.MinecraftProfile))]
[JsonSerializable(typeof(AssetObjectsDownloadTask.AssetIndexFile))]
[JsonSerializable(typeof(CacheIndex))]
[JsonSerializable(typeof(MetaLoaderInstaller.MetaLoaderEntry))]
[JsonSerializable(typeof(List<MetaLoaderInstaller.MetaLoaderEntry>))]
[JsonSerializable(typeof(MetaLoaderInstaller.MetaLoaderProfile))]
[JsonSerializable(typeof(ModrinthPackIndex))]
[JsonSerializable(typeof(McbbsPackManifest))]
[JsonSerializable(typeof(MultiMcPackManifest))]
[JsonSerializable(typeof(CurseForgePackManifest))]
[JsonSerializable(typeof(ModpackConfiguration))]
[JsonSerializable(typeof(Announcement))]
[JsonSerializable(typeof(List<Announcement>))]
[JsonSerializable(typeof(IReadOnlyDictionary<string, string>))]
[JsonSerializable(typeof(Dictionary<string, string>))]
[JsonSerializable(typeof(string[]))]
[JsonSerializable(typeof(string[][]))]
internal partial class LauncherJsonContext : JsonSerializerContext
{
}
