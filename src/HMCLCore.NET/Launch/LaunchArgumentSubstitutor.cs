using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Text;
using HMCLCore.NET.Auth;
using HMCLCore.NET.Auth.Microsoft;
using HMCLCore.NET.Auth.Yggdrasil;
using HMCLCore.NET.Game;

namespace HMCLCore.NET.Launch;

/// <summary>
/// Resolves the placeholders of the launch arguments of a version manifest,
/// for example <c>${auth_player_name}</c> or <c>${natives_directory}</c>.
/// </summary>
/// <remarks>
/// The version manifests of Minecraft describe their launch arguments with
/// placeholders. The official launcher substitutes them with the values of
/// the running session; arguments with an unknown placeholder have to be
/// dropped, because the game would not be able to parse them.
/// </remarks>
public sealed class LaunchArgumentSubstitutor
{
    private readonly Dictionary<string, string> _values;

    private LaunchArgumentSubstitutor(Dictionary<string, string> values)
    {
        _values = values;
    }

    /// <summary>
    /// Creates a substitutor for the given launch configuration.
    /// </summary>
    /// <param name="config">The launch configuration providing the session values.</param>
    /// <param name="manifest">The manifest of the launched version.</param>
    /// <returns>The substitutor holding the known placeholder values.</returns>
    public static LaunchArgumentSubstitutor Create(LaunchConfig config, GameInstanceManifest manifest)
    {
        var account = config.Account;
        var accessToken = ResolveAccessToken(account);
        var uuid = ResolveUuid(account);
        var userType = ResolveUserType(account);

        var primaryJar = config.PrimaryJarPath;
        var primaryJarName = string.IsNullOrEmpty(primaryJar)
            ? string.Empty
            : Path.GetFileName(primaryJar) ?? string.Empty;

        var values = new Dictionary<string, string>(StringComparer.Ordinal)
        {
            // Placeholders defined by the official launcher.
            ["auth_player_name"] = account.Username,
            ["auth_session"] = accessToken,
            ["auth_access_token"] = accessToken,
            ["auth_uuid"] = uuid,
            ["auth_xuid"] = string.Empty,
            ["clientid"] = string.Empty,
            ["user_type"] = userType,
            ["version_name"] = config.VersionName,
            ["version_type"] = config.VersionType,
            ["profile_name"] = "Minecraft",
            ["game_directory"] = config.GameDirectory,
            ["assets_root"] = config.AssetsDirectory,
            ["game_assets"] = config.AssetsDirectory,
            ["assets_index_name"] = config.AssetIndex ?? string.Empty,
            ["user_properties"] = "{}",
            ["resolution_width"] = config.WindowWidth.ToString(CultureInfo.InvariantCulture),
            ["resolution_height"] = config.WindowHeight.ToString(CultureInfo.InvariantCulture),
            ["library_directory"] = config.LibrariesDirectory,
            ["classpath"] = config.ClassPath,
            ["classpath_separator"] = Path.PathSeparator.ToString(),
            ["primary_jar"] = primaryJar ?? string.Empty,
            ["primary_jar_name"] = primaryJarName,
            ["language"] = CultureInfo.CurrentUICulture.Name,

            // Placeholders defined by HMCL.
            ["libraries_directory"] = config.LibrariesDirectory,
            ["file_separator"] = Path.DirectorySeparatorChar.ToString(),
            ["natives_directory"] = config.NativesDirectory,
            ["launcher_name"] = Metadata.SHORT_TITLE,
            ["launcher_version"] = Metadata.VERSION,
        };

        return new LaunchArgumentSubstitutor(values);
    }

    /// <summary>
    /// Substitutes every known placeholder of the given argument.
    /// </summary>
    /// <param name="argument">The argument to substitute.</param>
    /// <param name="result">The substituted argument.</param>
    /// <returns>False when the argument contains a placeholder that is not
    /// known, in which case the argument has to be dropped.</returns>
    public bool TrySubstitute(string argument, out string result)
    {
        result = argument;

        var start = argument.IndexOf("${", StringComparison.Ordinal);
        if (start < 0)
            return true;

        var builder = new StringBuilder();
        var position = 0;

        while (start >= 0)
        {
            var end = argument.IndexOf('}', start + 2);
            if (end < 0)
                return false;

            var key = argument[(start + 2)..end];
            if (!_values.TryGetValue(key, out var value))
                return false;

            builder.Append(argument, position, start - position);
            builder.Append(value);

            position = end + 1;
            start = argument.IndexOf("${", position, StringComparison.Ordinal);
        }

        builder.Append(argument, position, argument.Length - position);
        result = builder.ToString();
        return true;
    }

    /// <summary>
    /// Returns the access token of the given account.
    /// </summary>
    /// <param name="account">The account to resolve the token of.</param>
    /// <returns>The access token, or an empty string.</returns>
    private static string ResolveAccessToken(Account account) => account switch
    {
        MicrosoftAccount microsoft => microsoft.MinecraftAccessToken ?? string.Empty,
        YggdrasilAccount yggdrasil => yggdrasil.YggdrasilAccessToken ?? string.Empty,
        OAuthAccount oauth => oauth.AccessToken ?? string.Empty,
        _ => "0"
    };

    /// <summary>
    /// Returns the profile UUID of the given account.
    /// </summary>
    /// <param name="account">The account to resolve the UUID of.</param>
    /// <returns>The UUID, or an empty string.</returns>
    private static string ResolveUuid(Account account) => account switch
    {
        MicrosoftAccount microsoft => microsoft.MinecraftUuid ?? string.Empty,
        YggdrasilAccount yggdrasil => yggdrasil.ProfileUuid ?? string.Empty,
        _ => ComputeOfflineUuid(account.Username)
    };

    /// <summary>
    /// Computes the UUID of an offline player, the same way the official
    /// launcher does: a version 3 UUID of the name <c>OfflinePlayer:Name</c>.
    /// </summary>
    /// <param name="username">The name of the player.</param>
    /// <returns>The UUID of the offline player.</returns>
    public static string ComputeOfflineUuid(string username)
    {
        if (string.IsNullOrEmpty(username))
            return "00000000-0000-0000-0000-000000000000";

        var nameBytes = System.Text.Encoding.UTF8.GetBytes("OfflinePlayer:" + username);
        var hash = System.Security.Cryptography.MD5.HashData(nameBytes);

        hash[6] = (byte)((hash[6] & 0x0F) | 0x30);
        hash[8] = (byte)((hash[8] & 0x3F) | 0x80);

        return $"{hash[0]:x2}{hash[1]:x2}{hash[2]:x2}{hash[3]:x2}-{hash[4]:x2}{hash[5]:x2}-{hash[6]:x2}{hash[7]:x2}-{hash[8]:x2}{hash[9]:x2}-{hash[10]:x2}{hash[11]:x2}{hash[12]:x2}{hash[13]:x2}{hash[14]:x2}{hash[15]:x2}";
    }

    /// <summary>
    /// Returns the user type of the given account.
    /// </summary>
    /// <param name="account">The account to resolve the user type of.</param>
    /// <returns>The user type reported to the game.</returns>
    private static string ResolveUserType(Account account) => account switch
    {
        MicrosoftAccount => "msa",
        YggdrasilAccount => "mojang",
        _ => "legacy"
    };
}
