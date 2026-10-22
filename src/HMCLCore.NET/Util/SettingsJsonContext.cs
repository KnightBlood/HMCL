using System.Text.Json.Serialization;
using HMCLCore.NET.Setting;

namespace HMCLCore.NET.Util;

/// <summary>
/// The metadata of the settings file of the launcher, generated when the
/// launcher is compiled, so that the launcher can be published with the
/// native ahead-of-time compiler.
/// </summary>
/// <remarks>
/// The enumerations of the settings are written as their names, so that the
/// files stay readable and the values do not change when the order of the
/// enumerations changes.
/// </remarks>
[JsonSourceGenerationOptions(
    PropertyNameCaseInsensitive = true,
    WriteIndented = true,
    Converters = new[]
    {
        typeof(JsonStringEnumConverter<BackgroundType>),
        typeof(JsonStringEnumConverter<ThemeColorStyle>),
        typeof(JsonStringEnumConverter<ThemeContrast>),
        typeof(JsonStringEnumConverter<JavaVersionType>),
        typeof(JsonStringEnumConverter<ProcessPriority>),
        typeof(JsonStringEnumConverter<LauncherVisibility>),
        typeof(JsonStringEnumConverter<GraphicsBackend>),
        typeof(JsonStringEnumConverter<QuickPlayMode>)
    })]
[JsonSerializable(typeof(LauncherSettings))]
internal partial class SettingsJsonContext : JsonSerializerContext
{
}
