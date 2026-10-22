using System;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;

namespace HMCLCore.NET.Util;

/// <summary>
/// Represents the CPU architecture.
/// </summary>
public enum Arch
{
    /// <summary>Intel x86 (32-bit).</summary>
    X86,
    /// <summary>Intel x86-64 / AMD64 (64-bit).</summary>
    X86_64,
    /// <summary>ARM 32-bit.</summary>
    ARM32,
    /// <summary>ARM 64-bit (aarch64).</summary>
    ARM64,
    /// <summary>MIPS 32-bit.</summary>
    MIPS,
    /// <summary>MIPS 64-bit.</summary>
    MIPS64,
    /// <summary>MIPS 32-bit little-endian.</summary>
    MIPSEL,
    /// <summary>MIPS 64-bit little-endian.</summary>
    MIPS64EL,
    /// <summary>PowerPC 32-bit.</summary>
    PPC,
    /// <summary>PowerPC 64-bit.</summary>
    PPC64,
    /// <summary>PowerPC 32-bit little-endian.</summary>
    PPCLE,
    /// <summary>PowerPC 64-bit little-endian.</summary>
    PPC64LE,
    /// <summary>IBM S/390 32-bit.</summary>
    S390,
    /// <summary>IBM S/390x 64-bit.</summary>
    S390X,
    /// <summary>RISC-V 32-bit.</summary>
    RISCV32,
    /// <summary>RISC-V 64-bit.</summary>
    RISCV64,
    /// <summary>LoongArch 32-bit.</summary>
    LOONGARCH32,
    /// <summary>LoongArch 64-bit (old world ABI).</summary>
    LOONGARCH64_OW,
    /// <summary>LoongArch 64-bit.</summary>
    LOONGARCH64,
    /// <summary>Unknown architecture.</summary>
    UNKNOWN
}

/// <summary>
/// Represents the word size of an architecture.
/// </summary>
public enum Bits
{
    /// <summary>32-bit.</summary>
    BIT_32,
    /// <summary>64-bit.</summary>
    BIT_64,
    /// <summary>Unknown word size.</summary>
    UNKNOWN
}

/// <summary>
/// Represents the operating system type.
/// </summary>
public enum OsType
{
    /// <summary>Microsoft Windows.</summary>
    Windows,
    /// <summary>Linux and other Unix-like systems.</summary>
    Linux,
    /// <summary>Apple macOS.</summary>
    MacOs,
    /// <summary>FreeBSD.</summary>
    FreeBsd,
    /// <summary>An operating system the launcher was not built for.</summary>
    Unknown
}

/// <summary>
/// Platform detection and system information.
/// </summary>
public static class Platform
{
    /// <summary>
    /// The operating system the running process was built for.
    /// </summary>
    public static OsType CurrentOs
    {
        get
        {
            if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
                return OsType.Windows;
            if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
                return OsType.MacOs;
            if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
                return OsType.Linux;
            if (RuntimeInformation.IsOSPlatform(OSPlatform.FreeBSD))
                return OsType.FreeBsd;
            return OsType.Unknown;
        }
    }

    /// <summary>
    /// The architecture of the running process.
    /// </summary>
    public static Arch CurrentArch
    {
        get
        {
            switch (RuntimeInformation.ProcessArchitecture)
            {
                case Architecture.X64:
                    return Arch.X86_64;
                case Architecture.X86:
                    return Arch.X86;
                case Architecture.Arm64:
                    return Arch.ARM64;
                case Architecture.Arm:
                    return Arch.ARM32;
                default:
                    return Arch.UNKNOWN;
            }
        }
    }

    /// <summary>
    /// The architecture of the system the process runs on.
    /// </summary>
    /// <remarks>
    /// This differs from <see cref="CurrentArch"/> when the process runs under
    /// an emulation layer, for example x86-64 code on an ARM64 host.
    /// </remarks>
    public static Arch SystemArch
    {
        get
        {
            var os = CurrentOs;
            if (os == OsType.Windows)
            {
                var processorIdentifier = Environment.GetEnvironmentVariable("PROCESSOR_IDENTIFIER");
                if (!string.IsNullOrEmpty(processorIdentifier))
                {
                    var space = processorIdentifier.IndexOf(' ');
                    if (space > 0)
                    {
                        var arch = ParseArchName(processorIdentifier[..space]);
                        if (arch != Arch.UNKNOWN)
                            return arch;
                    }
                }
            }
            else if (os == OsType.MacOs)
            {
                // Rosetta translation reports x86-64 as the process architecture.
                if (CurrentArch == Arch.X86_64 && File.Exists("/usr/libexec/rosetta/runtime"))
                    return Arch.ARM64;
            }
            else if (os is OsType.Linux or OsType.FreeBsd)
            {
                var arch = ParseArchName(RunUnameMachine());
                if (arch != Arch.UNKNOWN)
                    return arch;
            }

            return CurrentArch;
        }
    }

    /// <summary>
    /// The word size of the current architecture.
    /// </summary>
    public static Bits CurrentBits => GetBits(CurrentArch);

    /// <summary>
    /// The word size of the given architecture.
    /// </summary>
    public static Bits GetBits(Arch arch) => arch switch
    {
        Arch.X86 or Arch.ARM32 or Arch.MIPS or Arch.MIPSEL or Arch.PPC or Arch.PPCLE
            or Arch.S390 or Arch.RISCV32 or Arch.LOONGARCH32 => Bits.BIT_32,
        Arch.X86_64 or Arch.ARM64 or Arch.MIPS64 or Arch.MIPS64EL or Arch.PPC64
            or Arch.PPC64LE or Arch.S390X or Arch.RISCV64 or Arch.LOONGARCH64
            or Arch.LOONGARCH64_OW => Bits.BIT_64,
        _ => Bits.UNKNOWN
    };

    /// <summary>
    /// Returns whether the architecture belongs to the x86 family.
    /// </summary>
    public static bool IsX86(Arch arch) => arch is Arch.X86 or Arch.X86_64;

    /// <summary>
    /// Returns whether the architecture belongs to the ARM family.
    /// </summary>
    public static bool IsArm(Arch arch) => arch is Arch.ARM32 or Arch.ARM64;

    /// <summary>
    /// The name of the Java executable on the current operating system.
    /// </summary>
    public static string JavaExecutable => CurrentOs == OsType.Windows ? "java.exe" : "java";

    /// <summary>
    /// The name of the operating system as Mojang uses it in rule files.
    /// </summary>
    public static string MojangOsName => CurrentOs switch
    {
        OsType.Windows => "windows",
        OsType.MacOs => "osx",
        OsType.Linux or OsType.FreeBsd => "linux",
        _ => "universal"
    };

    /// <summary>
    /// The name of the operating system in lower case.
    /// </summary>
    public static string CheckedOsName => CurrentOs switch
    {
        OsType.Windows => "windows",
        OsType.Linux => "linux",
        OsType.MacOs => "macos",
        OsType.FreeBsd => "freebsd",
        _ => "universal"
    };

    /// <summary>
    /// Parses an architecture name such as `amd64` or `aarch64`.
    /// </summary>
    public static Arch ParseArchName(string? value)
    {
        if (string.IsNullOrEmpty(value))
            return Arch.UNKNOWN;

        value = value.Trim().ToLowerInvariant();

        return value switch
        {
            "x8664" or "x86-64" or "x86_64" or "amd64" or "ia32e" or "em64t" or "x64" or "intel64" => Arch.X86_64,
            "x8632" or "x86-32" or "x86_32" or "x86" or "i86pc" or "i386" or "i486" or "i586" or "i686"
                or "ia32" or "x32" => Arch.X86,
            "arm64" or "aarch64" => Arch.ARM64,
            "arm" or "arm32" => Arch.ARM32,
            "mips64" => Arch.MIPS64,
            "mips64el" => Arch.MIPS64EL,
            "mips" or "mips32" => Arch.MIPS,
            "mipsel" or "mips32el" => Arch.MIPSEL,
            "riscv" or "risc-v" or "riscv64" => Arch.RISCV64,
            "riscv32" => Arch.RISCV32,
            "ppc64" or "powerpc64" => Arch.PPC64,
            "ppc64le" or "powerpc64le" => Arch.PPC64LE,
            "ppc" or "ppc32" or "powerpc" or "powerpc32" => Arch.PPC,
            "ppcle" or "ppc32le" or "powerpcle" or "powerpc32le" => Arch.PPCLE,
            "s390" => Arch.S390,
            "s390x" => Arch.S390X,
            "loongarch32" => Arch.LOONGARCH32,
            "loongarch64" => Arch.LOONGARCH64,
            "loongarch64_ow" => Arch.LOONGARCH64_OW,
            _ => value.StartsWith("armv7", StringComparison.Ordinal) ? Arch.ARM32
                : value.StartsWith("armv8", StringComparison.Ordinal) || value.StartsWith("armv9", StringComparison.Ordinal)
                    ? Arch.ARM64
                    : Arch.UNKNOWN
        };
    }

    /// <summary>
    /// Parses an operating system name such as `Windows 10`.
    /// </summary>
    public static OsType ParseOsName(string? name)
    {
        if (string.IsNullOrEmpty(name))
            return OsType.Unknown;

        name = name.Trim().ToLowerInvariant();

        if (name.Contains("mac") || name.Contains("darwin") || name.Contains("osx"))
            return OsType.MacOs;
        if (name.Contains("win"))
            return OsType.Windows;
        if (name.Contains("solaris") || name.Contains("linux") || name.Contains("unix") || name.Contains("sunos"))
            return OsType.Linux;
        if (name.Equals("freebsd"))
            return OsType.FreeBsd;
        return OsType.Unknown;
    }

    /// <summary>
    /// Returns whether the launcher runs under Wine.
    /// </summary>
    public static bool IsRunningUnderWine
    {
        get
        {
            if (CurrentOs != OsType.Windows)
                return false;

            foreach (var name in new[] { "WINEPREFIX", "WINELOADER", "WINEDLLPATH" })
            {
                if (Environment.GetEnvironmentVariable(name) != null)
                    return true;
            }
            return false;
        }
    }

    /// <summary>
    /// Returns the total physical memory in bytes, if available.
    /// </summary>
    public static long? TotalPhysicalMemory => (long)GetMemoryStatus().ullTotalPhys;

    /// <summary>
    /// Returns the available physical memory in bytes, if available.
    /// </summary>
    public static long? AvailablePhysicalMemory => (long)GetMemoryStatus().ullAvailPhys;

    /// <summary>
    /// Reads the machine architecture from the `uname` command on Unix-like systems.
    /// </summary>
    private static string RunUnameMachine()
    {
        foreach (var uname in new[] { "/bin/uname", "/usr/bin/uname" })
        {
            if (!File.Exists(uname))
                continue;

            try
            {
                using var process = Process.Start(new ProcessStartInfo
                {
                    FileName = uname,
                    Arguments = "-m",
                    RedirectStandardOutput = true,
                    UseShellExecute = false,
                    CreateNoWindow = true
                });

                if (process == null)
                    continue;

                if (!process.WaitForExit(3000))
                {
                    process.Kill();
                    continue;
                }

                if (process.ExitCode != 0)
                    continue;

                return process.StandardOutput.ReadToEnd().Trim();
            }
            catch (Exception e) when (e is IOException or System.ComponentModel.Win32Exception)
            {
                // Ignore and try the next candidate.
            }
        }

        return string.Empty;
    }

    /// <summary>
    /// Returns the system memory status.
    /// </summary>
    private static MEMORYSTATUS GetMemoryStatus()
    {
        var status = new MEMORYSTATUS { dwLength = (uint)Marshal.SizeOf<MEMORYSTATUS>() };
        GlobalMemoryStatusEx(ref status);
        return status;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct MEMORYSTATUS
    {
        public uint dwLength;
        public uint dwMemoryLoad;
        public nuint ullTotalPhys;
        public nuint ullAvailPhys;
        public nuint ullTotalVirtual;
        public nuint ullAvailVirtual;
        public nuint dwReserved;
    }

    [DllImport("kernel32.dll", SetLastError = true)]
    private static extern void GlobalMemoryStatusEx(ref MEMORYSTATUS lpBuffer);
}