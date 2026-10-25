//! Detection of the operating system the launcher runs on.
//!
//! The launcher needs to know the operating system and the architecture of
//! the machine it runs on, because the files it downloads and the commands
//! it runs depend on them.

use std::fmt;

use super::os_version::OsVersion;

/// The operating systems the launcher distinguishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsType {
    /// Microsoft Windows.
    Windows,
    /// Linux or another distribution that reports `linux`.
    Linux,
    /// Apple macOS.
    MacOs,
    /// An operating system the launcher was not built for.
    Unknown,
}

impl OsType {
    /// Returns the operating system the current binary was compiled for.
    pub const fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "linux") {
            Self::Linux
        } else if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Unknown
        }
    }

    /// Returns whether this is a Windows system.
    pub const fn is_windows(self) -> bool {
        matches!(self, Self::Windows)
    }

    /// Returns whether this is a Linux system.
    pub const fn is_linux(self) -> bool {
        matches!(self, Self::Linux)
    }

    /// Returns whether this is a macOS system.
    pub const fn is_macos(self) -> bool {
        matches!(self, Self::MacOs)
    }

    /// Returns whether this is a Unix-like system.
    pub const fn is_unix_like(self) -> bool {
        matches!(self, Self::Linux | Self::MacOs)
    }

    /// Returns the name of the operating system the rules of the versions of
    /// the game use.
    ///
    /// The names are the ones the manifests of the versions write, so macOS is
    /// `osx`, because the manifests of Mojang write it that way. An operating
    /// system the launcher does not know has no name, so the rules about the
    /// operating systems do not apply to it.
    pub const fn mojang_name(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Linux => "linux",
            Self::MacOs => "osx",
            Self::Unknown => "",
        }
    }

    /// Reads the name of an operating system.
    ///
    /// The names are read the way the names of the Java launcher are read: a
    /// name that holds `mac` or `darwin` is macOS, a name that holds `win` is
    /// Windows, and a name that holds `linux` is Linux. A name the launcher
    /// does not know becomes [`OsType::Unknown`].
    pub fn parse_name(name: &str) -> Self {
        let name = name.trim().to_ascii_lowercase();

        if name.contains("mac") || name.contains("darwin") || name.contains("osx") {
            Self::MacOs
        } else if name.contains("win") {
            Self::Windows
        } else if name.contains("solaris")
            || name.contains("linux")
            || name.contains("unix")
            || name.contains("sunos")
        {
            Self::Linux
        } else {
            Self::Unknown
        }
    }

    /// Returns the version of this operating system on the machine the
    /// launcher runs on.
    ///
    /// The version is read from the operating system; the parts of the text
    /// the launcher does not read are kept, so the version can be told to the
    /// player as it is.
    pub fn system_version(self) -> OsVersion {
        OsVersion::of(self, &system_version_text(self))
    }
}

impl fmt::Display for OsType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Windows => "Windows",
            Self::Linux => "Linux",
            Self::MacOs => "macOS",
            Self::Unknown => "Unknown",
        };
        formatter.write_str(name)
    }
}

/// Returns the operating system of the running process.
pub const fn current_os() -> OsType {
    OsType::current()
}

/// Returns whether the launcher runs under Wine.
///
/// Wine sets the variables of the environment it runs the programs of Windows
/// in, so their presence tells the launcher that the commands it writes for
/// Windows will not run.
pub fn is_running_under_wine() -> bool {
    if !OsType::current().is_windows() {
        return false;
    }

    ["WINEPREFIX", "WINELOADER", "WINEDLLPATH"]
        .iter()
        .any(|name| std::env::var_os(name).is_some())
}

/// Returns whether the version of Windows is Windows 7 or a newer one.
///
/// The commands the launcher writes for Windows use the features of the
/// console of Windows 7 and newer versions.
pub fn is_windows_7_or_later() -> bool {
    OsType::current()
        .system_version()
        .is_at_least(&OsVersion::WINDOWS_7)
}

/// The architectures the launcher distinguishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    /// x86_64, also known as AMD64.
    X86_64,
    /// x86, also known as IA-32.
    X86,
    /// ARM 64-bit, also known as AArch64.
    Aarch64,
    /// ARM 32-bit.
    Arm,
    /// An architecture the launcher does not know.
    Unknown,
}

impl Arch {
    /// Returns the architecture the current binary was compiled for.
    pub const fn current() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            Self::X86_64
        }
        #[cfg(target_arch = "x86")]
        {
            Self::X86
        }
        #[cfg(target_arch = "aarch64")]
        {
            Self::Aarch64
        }
        #[cfg(target_arch = "arm")]
        {
            Self::Arm
        }
        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "x86",
            target_arch = "aarch64",
            target_arch = "arm"
        )))]
        {
            Self::Unknown
        }
    }

    /// Returns whether the architecture is a 64-bit one.
    pub const fn is_64bit(self) -> bool {
        matches!(self, Self::X86_64 | Self::Aarch64)
    }

    /// Returns whether the architecture reads the instructions of x86.
    ///
    /// The native libraries of the versions of the game are built for the x86
    /// architectures by Mojang itself.
    pub const fn is_x86(self) -> bool {
        matches!(self, Self::X86 | Self::X86_64)
    }

    /// Returns the name of the architecture the rules of the versions use.
    ///
    /// The rules about the architectures are read in the names of Java; the
    /// architecture the launcher does not know has no name, so the rules about
    /// the architectures do not apply to it.
    pub const fn rule_name(self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64",
            Self::X86 => "x86",
            Self::Aarch64 => "aarch64",
            Self::Arm => "arm",
            Self::Unknown => "",
        }
    }
}

impl fmt::Display for Arch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::X86_64 => "x86_64",
            Self::X86 => "x86",
            Self::Aarch64 => "aarch64",
            Self::Arm => "arm",
            Self::Unknown => "Unknown",
        };
        formatter.write_str(name)
    }
}

/// Returns the CPU architecture of the running process.
pub const fn current_arch() -> Arch {
    Arch::current()
}

/// The information about the machine the launcher runs on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemInfo {
    /// The operating system.
    pub os: OsType,
    /// The CPU architecture.
    pub arch: Arch,
    /// The total amount of the physical memory, in bytes, if it is known.
    pub total_physical_memory: Option<u64>,
    /// The amount of the physical memory that is free, in bytes, if it is
    /// known.
    pub available_physical_memory: Option<u64>,
}

impl SystemInfo {
    /// Detects the information about the machine the launcher runs on.
    ///
    /// The amount of the memory is read from the operating system; the
    /// information about the graphics hardware is not read yet.
    pub fn detect() -> Self {
        Self {
            os: current_os(),
            arch: current_arch(),
            total_physical_memory: total_physical_memory(),
            available_physical_memory: available_physical_memory(),
        }
    }
}

impl Default for SystemInfo {
    fn default() -> Self {
        Self::detect()
    }
}

/// Returns the total amount of the physical memory, in bytes.
///
/// The amount is not known when the operating system does not tell it.
pub fn total_physical_memory() -> Option<u64> {
    memory_status().map(|status| status.total)
}

/// Returns the amount of the physical memory that is free, in bytes.
///
/// The amount is not known when the operating system does not tell it.
pub fn available_physical_memory() -> Option<u64> {
    memory_status().map(|status| status.available)
}

/// The amount of the memory of the machine.
struct MemoryStatus {
    /// The total amount of the physical memory, in bytes.
    total: u64,
    /// The amount of the physical memory that is free, in bytes.
    available: u64,
}

/// Reads the version of an operating system as the launcher tells it.
///
/// The version is read from the running system, so a Windows system tells the
/// numbers of its version and the name of its edition, and a Linux system
/// tells the version of its kernel; an operating system the launcher cannot
/// read the version of tells nothing.
fn system_version_text(os: OsType) -> String {
    if os == OsType::Windows {
        // The numbers of the version of Windows are read from the registry of
        // the system, so the name of the edition does not get in their way.
        if let Some(text) = windows_registry_version() {
            return text;
        }
    }

    // The name of the version of the operating system is not read yet, so the
    // version of the kernel is read from the system where it is available.
    match os {
        OsType::Linux => kernel_version().unwrap_or_default(),
        _ => String::new(),
    }
}

/// Reads the numbers of the version of Windows from the registry of the
/// system.
#[cfg(windows)]
fn windows_registry_version() -> Option<String> {
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;

    const ERROR_SUCCESS: u32 = 0;
    const HKEY_LOCAL_MACHINE: isize = 0x8000_0002u32 as isize;
    const REG_SZ: u32 = 0x0000_0001;
    const REG_DWORD: u32 = 0x0000_0004;
    const KEY_QUERY_VALUE: u32 = 0x0001;
    const KEY_WOW64_64KEY: u32 = 0x0100;

    #[link(name = "advapi32")]
    extern "system" {
        fn RegOpenKeyExW(
            key: isize,
            sub_key: *const u16,
            options: u32,
            access: u32,
            result: *mut isize,
        ) -> u32;
        fn RegQueryValueExW(
            key: isize,
            value_name: *const u16,
            reserved: *mut u32,
            kind: *mut u32,
            data: *mut u8,
            data_size: *mut u32,
        ) -> u32;
        fn RegCloseKey(key: isize) -> u32;
    }

    /// Reads a text of the registry of the system.
    unsafe fn read(key: isize, value: *const u16) -> Option<String> {
        let mut kind = 0u32;
        let mut size = 0u32;

        // The length of the text is read first, so the buffer is written to
        // only once.
        let status = RegQueryValueExW(
            key,
            value,
            ptr::null_mut(),
            &mut kind,
            ptr::null_mut(),
            &mut size,
        );
        if status != ERROR_SUCCESS || size == 0 {
            return None;
        }

        let mut buffer = vec![0u8; size as usize];
        let status = RegQueryValueExW(
            key,
            value,
            ptr::null_mut(),
            &mut kind,
            buffer.as_mut_ptr(),
            &mut size,
        );
        if status != ERROR_SUCCESS {
            return None;
        }

        // A number of the registry is a number of four bytes; the numbers are
        // written as text, so the launcher reads both forms.
        if kind == REG_DWORD {
            let bytes: [u8; 4] = buffer[..4].try_into().ok()?;
            return Some(u32::from_le_bytes(bytes).to_string());
        }

        if kind != REG_SZ {
            return None;
        }

        // The text of the registry is a text of Windows ended by a character
        // that is not written.
        let text = &buffer[..size as usize];
        let (pairs, _) = text.as_chunks::<2>();
        let wide: Vec<u16> = pairs
            .iter()
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .take_while(|character| *character != 0)
            .collect();
        String::from_utf16(&wide).ok()
    }

    let sub_key: Vec<u16> = std::ffi::OsStr::new("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut key: isize = 0;
    // The call is safe, because the key is written to by the call and the text
    // is a text of Windows ended by a character that is not written.
    let opened = unsafe {
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            sub_key.as_ptr(),
            0,
            KEY_QUERY_VALUE | KEY_WOW64_64KEY,
            &mut key,
        )
    };
    if opened != ERROR_SUCCESS {
        return None;
    }

    let value = |name: &str| {
        let wide: Vec<u16> = std::ffi::OsStr::new(name)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        unsafe { read(key, wide.as_ptr()) }
    };

    let current_build = value("CurrentBuild");
    let ubr = value("UBR");
    unsafe {
        RegCloseKey(key);
    }

    let current_build = current_build?;
    let ubr = ubr.and_then(|ubr| ubr.trim().parse::<u32>().ok());

    // The version of the system is `10.0` for Windows 10 and Windows 11, so
    // the number of the build is what tells them apart.
    let mut version = format!("10.0.{}", current_build.trim());
    if let Some(ubr) = ubr {
        version.push_str(&format!(".{ubr}"));
    }

    Some(version)
}

/// Reads the numbers of the version of Windows from the registry of the
/// system.
#[cfg(not(windows))]
fn windows_registry_version() -> Option<String> {
    None
}

/// Reads the version of the kernel of Linux.
#[cfg(target_os = "linux")]
fn kernel_version() -> Option<String> {
    // The name of the release of the kernel is the text the command `uname`
    // writes, so it is read from the system where the command is not needed.
    let release = std::fs::read_to_string("/proc/sys/kernel/osrelease").ok()?;
    let release = release.trim();
    if release.is_empty() {
        return None;
    }

    // The text of the release is `6.8.0-45-generic`; the numbers of the
    // version are compared, so the rest of the text is kept as it is.
    let numbers: String = release
        .chars()
        .take_while(|character| character.is_ascii_digit() || *character == '.')
        .collect();
    if numbers.is_empty() {
        return None;
    }

    Some(numbers)
}

/// Reads the version of the kernel of Linux.
#[cfg(not(target_os = "linux"))]
fn kernel_version() -> Option<String> {
    None
}

/// Reads the amount of the memory from the operating system.
#[cfg(windows)]
fn memory_status() -> Option<MemoryStatus> {
    use std::mem::MaybeUninit;

    #[repr(C)]
    struct MemoryStatusEx {
        length: u32,
        memory_load: u32,
        total_physical: u64,
        available_physical: u64,
        total_page_file: u64,
        available_page_file: u64,
        total_virtual: u64,
        available_virtual: u64,
        available_extended_virtual: u64,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalMemoryStatusEx(status: *mut MemoryStatusEx) -> i32;
    }

    let mut status: MaybeUninit<MemoryStatusEx> = MaybeUninit::zeroed();
    let length = std::mem::size_of::<MemoryStatusEx>() as u32;

    // The structure is filled in by the call, so it is written to before it
    // is read.
    unsafe {
        std::ptr::addr_of_mut!((*status.as_mut_ptr()).length).write(length);
    }

    // The call is safe, because the structure is of a known size and the
    // pointer points to it.
    let filled = unsafe { GlobalMemoryStatusEx(status.as_mut_ptr()) };
    if filled == 0 {
        return None;
    }

    let status = unsafe { status.assume_init() };
    Some(MemoryStatus {
        total: status.total_physical,
        available: status.available_physical,
    })
}

/// Reads the amount of the memory from the operating system.
#[cfg(target_os = "linux")]
fn memory_status() -> Option<MemoryStatus> {
    let content = std::fs::read_to_string("/proc/meminfo").ok()?;

    let mut total = None;
    let mut available = None;

    for line in content.lines() {
        let (name, value) = match line.split_once(':') {
            Some(parts) => parts,
            None => continue,
        };

        // The values are written in kibibytes.
        let value: u64 = value.split_whitespace().next()?.parse().ok()?;
        let value = value * 1_024;

        match name {
            "MemTotal" => total = Some(value),
            "MemAvailable" => available = Some(value),
            _ => {}
        }
    }

    Some(MemoryStatus {
        total: total?,
        available: available?,
    })
}

/// Reads the amount of the memory from the operating system.
#[cfg(not(any(windows, target_os = "linux")))]
fn memory_status() -> Option<MemoryStatus> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exactly_one_system_is_selected() {
        let selected = [
            current_os().is_windows(),
            current_os().is_linux(),
            current_os().is_macos(),
        ]
        .into_iter()
        .filter(|selected| *selected)
        .count();

        assert!(selected <= 1);
    }

    #[test]
    fn display_names_are_not_empty() {
        for os in [
            OsType::Windows,
            OsType::Linux,
            OsType::MacOs,
            OsType::Unknown,
        ] {
            assert!(!os.to_string().is_empty());
        }

        for arch in [
            Arch::X86_64,
            Arch::X86,
            Arch::Aarch64,
            Arch::Arm,
            Arch::Unknown,
        ] {
            assert!(!arch.to_string().is_empty());
        }
    }

    #[test]
    fn the_system_information_is_detected() {
        let info = SystemInfo::detect();
        assert_eq!(info.os, current_os());
        assert_eq!(info.arch, current_arch());

        // The amount of the memory changes while the tests run, so the default
        // information is only compared by the system and the architecture.
        let default = SystemInfo::default();
        assert_eq!(default.os, info.os);
        assert_eq!(default.arch, info.arch);
    }

    #[test]
    fn the_amount_of_the_memory_is_read() {
        let info = SystemInfo::detect();

        // The amount of the memory is only known on the systems the launcher
        // reads it from.
        if let Some(total) = info.total_physical_memory {
            assert!(total > 0);
        }
        if let (Some(total), Some(available)) =
            (info.total_physical_memory, info.available_physical_memory)
        {
            assert!(available <= total);
        }
    }

    #[test]
    fn the_names_of_the_systems_are_read() {
        assert_eq!(OsType::parse_name("Windows"), OsType::Windows);
        assert_eq!(OsType::parse_name("Windows 11"), OsType::Windows);
        assert_eq!(OsType::parse_name("Linux"), OsType::Linux);
        assert_eq!(OsType::parse_name("Darwin"), OsType::MacOs);
        assert_eq!(OsType::parse_name("Mac OS X"), OsType::MacOs);
        assert_eq!(OsType::parse_name("FreeBSD"), OsType::Unknown);
        assert_eq!(OsType::parse_name(""), OsType::Unknown);
    }

    #[test]
    fn the_names_of_the_manifests_are_written() {
        assert_eq!(OsType::Windows.mojang_name(), "windows");
        assert_eq!(OsType::Linux.mojang_name(), "linux");
        assert_eq!(OsType::MacOs.mojang_name(), "osx");
        assert_eq!(OsType::Unknown.mojang_name(), "");
    }

    #[test]
    fn the_version_of_the_system_is_read() {
        let version = OsType::current().system_version();

        // The version is read from the system the launcher runs on, so only
        // the text it was read from is told apart.
        assert_eq!(version.operating_system(), OsType::current());
        if OsType::current() != OsType::Unknown {
            assert!(!version.version().is_empty());
        }

        if OsType::current() == OsType::Windows {
            // The numbers of the version of Windows are read from the registry
            // of the system, so the version is Windows 10 or a newer one.
            println!("Windows version: {}", version.version());
            assert!(version.is_at_least(&OsVersion::WINDOWS_10));
        }
    }

    #[test]
    fn the_launcher_tells_whether_it_runs_under_wine() {
        // The variables of the environment of Wine are only set under Wine, so
        // the launcher does not run under Wine when the tests are not.
        if OsType::current() != OsType::Windows {
            assert!(!is_running_under_wine());
        }
    }

    #[test]
    fn the_launcher_tells_whether_windows_is_new_enough() {
        // The version of Windows is only read on Windows.
        if OsType::current() == OsType::Windows {
            assert!(is_windows_7_or_later());
        }
    }

    #[test]
    fn the_architectures_are_told_apart() {
        assert!(Arch::X86_64.is_64bit());
        assert!(Arch::Aarch64.is_64bit());
        assert!(!Arch::X86.is_64bit());
        assert!(!Arch::Arm.is_64bit());
        assert!(!Arch::Unknown.is_64bit());
    }
}
