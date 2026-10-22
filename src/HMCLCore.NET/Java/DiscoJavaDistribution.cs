using System;
using System.Collections.Generic;
using HMCLCore.NET.Util;

namespace HMCLCore.NET.Java;

/// <summary>
/// The Java distributions published through the Disco API of foojay.io.
/// </summary>
public enum DiscoJavaDistribution
{
    /// <summary>
    /// Eclipse Temurin, maintained by the Adoptium project.
    /// </summary>
    Temurin,

    /// <summary>
    /// BellSoft Liberica.
    /// </summary>
    Liberica,

    /// <summary>
    /// Azul Zulu.
    /// </summary>
    Zulu,

    /// <summary>
    /// Oracle GraalVM.
    /// </summary>
    Graalvm,

    /// <summary>
    /// IBM Semeru, using the OpenJ9 virtual machine.
    /// </summary>
    Semeru,

    /// <summary>
    /// Amazon Corretto.
    /// </summary>
    Corretto
}

/// <summary>
/// Describes the Java distributions of the Disco API: their display name, the
/// parameter identifying them in the API and the platforms they are published
/// for.
/// </summary>
public static class DiscoJavaDistributions
{
    /// <summary>
    /// All distributions offered by the launcher.
    /// </summary>
    public static IReadOnlyList<DiscoJavaDistribution> All { get; } = new[]
    {
        DiscoJavaDistribution.Temurin,
        DiscoJavaDistribution.Liberica,
        DiscoJavaDistribution.Zulu,
        DiscoJavaDistribution.Graalvm,
        DiscoJavaDistribution.Semeru,
        DiscoJavaDistribution.Corretto
    };

    /// <summary>
    /// Returns the name shown to the user for the given distribution.
    /// </summary>
    /// <param name="distribution">The distribution to describe.</param>
    /// <returns>The display name of the distribution.</returns>
    public static string GetDisplayName(DiscoJavaDistribution distribution) => distribution switch
    {
        DiscoJavaDistribution.Temurin => "Eclipse Temurin",
        DiscoJavaDistribution.Liberica => "BellSoft Liberica",
        DiscoJavaDistribution.Zulu => "Azul Zulu",
        DiscoJavaDistribution.Graalvm => "Oracle GraalVM",
        DiscoJavaDistribution.Semeru => "IBM Semeru (OpenJ9)",
        DiscoJavaDistribution.Corretto => "Amazon Corretto",
        _ => distribution.ToString()
    };

    /// <summary>
    /// Returns the vendor of the given distribution.
    /// </summary>
    /// <param name="distribution">The distribution to describe.</param>
    /// <returns>The vendor of the distribution.</returns>
    public static string GetVendor(DiscoJavaDistribution distribution) => distribution switch
    {
        DiscoJavaDistribution.Temurin => "Adoptium",
        DiscoJavaDistribution.Liberica => "BellSoft",
        DiscoJavaDistribution.Zulu => "Azul",
        DiscoJavaDistribution.Graalvm => "Oracle",
        DiscoJavaDistribution.Semeru => "IBM",
        DiscoJavaDistribution.Corretto => "Amazon",
        _ => string.Empty
    };

    /// <summary>
    /// Returns the value of the <c>distribution</c> parameter of the Disco API
    /// for the given distribution.
    /// </summary>
    /// <param name="distribution">The distribution to describe.</param>
    /// <returns>The API parameter of the distribution.</returns>
    public static string GetApiParameter(DiscoJavaDistribution distribution) => distribution switch
    {
        DiscoJavaDistribution.Temurin => "temurin",
        DiscoJavaDistribution.Liberica => "liberica",
        DiscoJavaDistribution.Zulu => "zulu",
        DiscoJavaDistribution.Graalvm => "graalvm",
        DiscoJavaDistribution.Semeru => "semeru",
        DiscoJavaDistribution.Corretto => "corretto",
        _ => distribution.ToString().ToLowerInvariant()
    };

    /// <summary>
    /// Returns whether the given distribution publishes full JDKs as well as
    /// JREs.
    /// </summary>
    /// <param name="distribution">The distribution to inspect.</param>
    /// <returns>Whether the distribution offers JDK builds.</returns>
    public static bool SupportsJdk(DiscoJavaDistribution distribution) => distribution switch
    {
        DiscoJavaDistribution.Temurin => true,
        DiscoJavaDistribution.Liberica => true,
        DiscoJavaDistribution.Zulu => true,
        DiscoJavaDistribution.Graalvm => true,
        DiscoJavaDistribution.Semeru => true,
        DiscoJavaDistribution.Corretto => true,
        _ => false
    };

    /// <summary>
    /// Returns whether the given distribution publishes JREs without a
    /// compiler.
    /// </summary>
    /// <param name="distribution">The distribution to inspect.</param>
    /// <returns>Whether the distribution offers JRE builds.</returns>
    public static bool SupportsJre(DiscoJavaDistribution distribution) => distribution switch
    {
        DiscoJavaDistribution.Temurin => true,
        DiscoJavaDistribution.Liberica => true,
        DiscoJavaDistribution.Zulu => true,
        DiscoJavaDistribution.Graalvm => false,
        DiscoJavaDistribution.Semeru => true,
        DiscoJavaDistribution.Corretto => false,
        _ => false
    };

    /// <summary>
    /// Returns whether the given distribution publishes builds for the given
    /// operating system and CPU architecture.
    /// </summary>
    /// <param name="distribution">The distribution to inspect.</param>
    /// <param name="os">The operating system of the platform.</param>
    /// <param name="arch">The CPU architecture of the platform.</param>
    /// <returns>Whether the distribution supports the platform.</returns>
    public static bool Supports(DiscoJavaDistribution distribution, OsType os, Arch arch) => (os, arch) switch
    {
        (OsType.Windows, Arch.X86_64) => true,
        (OsType.Windows, Arch.X86) => distribution is DiscoJavaDistribution.Temurin
            or DiscoJavaDistribution.Liberica
            or DiscoJavaDistribution.Zulu,
        (OsType.Windows, Arch.ARM64) => distribution is DiscoJavaDistribution.Temurin
            or DiscoJavaDistribution.Liberica
            or DiscoJavaDistribution.Zulu,
        (OsType.Linux, Arch.X86_64) => true,
        (OsType.Linux, Arch.X86) => distribution is DiscoJavaDistribution.Temurin
            or DiscoJavaDistribution.Liberica
            or DiscoJavaDistribution.Zulu,
        (OsType.Linux, Arch.ARM64) => true,
        (OsType.Linux, Arch.ARM32) => distribution is DiscoJavaDistribution.Temurin
            or DiscoJavaDistribution.Liberica
            or DiscoJavaDistribution.Zulu,
        (OsType.Linux, Arch.RISCV64) => distribution is DiscoJavaDistribution.Temurin
            or DiscoJavaDistribution.Liberica
            or DiscoJavaDistribution.Zulu,
        (OsType.Linux, Arch.PPC64) => distribution is DiscoJavaDistribution.Temurin,
        (OsType.Linux, Arch.PPC64LE) => true,
        (OsType.Linux, Arch.S390X) => distribution is DiscoJavaDistribution.Temurin
            or DiscoJavaDistribution.Semeru,
        (OsType.MacOs, Arch.X86_64) => true,
        (OsType.MacOs, Arch.ARM64) => true,
        _ => false
    };
}
