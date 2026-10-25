//! The makers of the parts of the machine.
//!
//! The launcher reads the name of the graphics card of the machine to tell
//! which card the game runs on, and to tell the cards of the same maker apart.
//! The name the machine gives is not the name the maker uses, so the launcher
//! cleans it and reads the maker from it.
//!
//! The makers are read from three kinds of names: the name of the card, the
//! number the bus of the machine gives the card and the number the processor
//! of the machine gives itself. The numbers are the ones the makers registered,
//! so a card whose name says nothing is still read.

/// The maker of a part of the machine.
///
/// The name of the maker is the one the launcher shows, so a part whose maker
/// is not known keeps the name the machine gave it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HardwareVendor {
    /// Intel.
    Intel,
    /// NVIDIA.
    Nvidia,
    /// AMD.
    Amd,
    /// Apple.
    Apple,
    /// ARM.
    Arm,
    /// Qualcomm.
    Qualcomm,
    /// MediaTek.
    Mtk,
    /// VMware.
    Vmware,
    /// Parallel.
    Parallel,
    /// Microsoft.
    Microsoft,
    /// Moore Threads.
    MooreThreads,
    /// Broadcom.
    Broadcom,
    /// Imagination.
    Img,
    /// Loongson.
    Loongson,
    /// Jingjia Micro.
    JingjiaMicro,
    /// Huawei.
    Huawei,
    /// Zhaoxin.
    Zhaoxin,
    /// Samsung.
    Samsung,
    /// Marvell.
    Marvell,
    /// Ampere.
    Ampere,
    /// Rockchip.
    Rockchip,
    /// T-Head.
    Thead,
    /// StarFive.
    Starfive,
    /// ESWIN.
    Eswin,
    /// SpacemiT.
    Spacemit,
    /// A maker the launcher does not know, named by its text.
    Other(String),
}

impl HardwareVendor {
    /// Returns the name the launcher shows for the maker.
    pub fn name(&self) -> &str {
        match self {
            Self::Intel => "Intel",
            Self::Nvidia => "NVIDIA",
            Self::Amd => "AMD",
            Self::Apple => "Apple",
            Self::Arm => "ARM",
            Self::Qualcomm => "Qualcomm",
            Self::Mtk => "MTK",
            Self::Vmware => "VMware",
            Self::Parallel => "Parallel",
            Self::Microsoft => "Microsoft",
            Self::MooreThreads => "Moore Threads",
            Self::Broadcom => "Broadcom",
            Self::Img => "Imagination",
            Self::Loongson => "Loongson",
            Self::JingjiaMicro => "Jingjia Micro",
            Self::Huawei => "Huawei",
            Self::Zhaoxin => "Zhaoxin",
            Self::Samsung => "Samsung",
            Self::Marvell => "Marvell",
            Self::Ampere => "Ampere",
            Self::Rockchip => "Rockchip",
            Self::Thead => "T-Head",
            Self::Starfive => "StarFive",
            Self::Eswin => "ESWIN",
            Self::Spacemit => "SpacemiT",
            Self::Other(name) => name,
        }
    }

    /// Reads the maker from the name of a part.
    ///
    /// The name is read without care for the case of its letters, because the
    /// machines write the names of the makers in more than one way. A name
    /// that does not begin with the name of a known maker is kept as it is,
    /// so the launcher still shows the part the machine told it about.
    pub fn of(name: &str) -> Self {
        Self::get_known(name).unwrap_or_else(|| Self::Other(name.to_string()))
    }

    /// Reads the maker from the name of a part, if the launcher knows it.
    ///
    /// The rules of the launcher of Java are kept: the name is read from its
    /// beginning, and `AMD` is only read when the letters that follow it are
    /// not letters, so `AMD` is not read from a name such as `AMDYZ`.
    pub fn get_known(name: &str) -> Option<Self> {
        let lower = name.to_lowercase();

        if lower.starts_with("intel") || lower.starts_with("genuineintel") {
            return Some(Self::Intel);
        }
        if lower.starts_with("nvidia") {
            return Some(Self::Nvidia);
        }
        if lower.starts_with("advanced micro devices")
            || lower.starts_with("authenticamd")
            || (lower.starts_with("amd")
                && !(lower.len() > 3
                    && lower
                        .chars()
                        .nth(3)
                        .is_some_and(|character| character.is_alphabetic())))
        {
            return Some(Self::Amd);
        }
        if lower == "brcm" || lower.starts_with("broadcom") {
            return Some(Self::Broadcom);
        }
        if lower.starts_with("mediatek") {
            return Some(Self::Mtk);
        }
        if lower == "qcom" || lower.starts_with("qualcomm") {
            return Some(Self::Qualcomm);
        }
        if lower.starts_with("apple") {
            return Some(Self::Apple);
        }
        if lower.starts_with("microsoft") {
            return Some(Self::Microsoft);
        }
        if lower.starts_with("imagination") || lower == "img" {
            return Some(Self::Img);
        }
        if lower.starts_with("loongson") {
            return Some(Self::Loongson);
        }
        if lower.starts_with("moore threads") {
            return Some(Self::MooreThreads);
        }
        if lower.starts_with("jingjia") {
            return Some(Self::JingjiaMicro);
        }
        if lower.starts_with("huawei") || lower.starts_with("hisilicon") {
            return Some(Self::Huawei);
        }
        if lower.starts_with("zhaoxin") {
            return Some(Self::Zhaoxin);
        }
        if lower.starts_with("marvell") {
            return Some(Self::Marvell);
        }
        if lower.starts_with("samsung") {
            return Some(Self::Samsung);
        }
        if lower.starts_with("ampere") {
            return Some(Self::Ampere);
        }
        if lower.starts_with("rockchip") {
            return Some(Self::Rockchip);
        }
        if lower.starts_with("thead") || lower.starts_with("t-head") {
            return Some(Self::Thead);
        }
        if lower.starts_with("starfive") {
            return Some(Self::Starfive);
        }
        if lower.starts_with("eswin") {
            return Some(Self::Eswin);
        }
        if lower.starts_with("spacemit") {
            return Some(Self::Spacemit);
        }

        None
    }

    /// Reads the maker from the number the bus of the machine gives a part.
    ///
    /// The numbers are the ones the makers registered, and a number that no
    /// known maker registered is not read.
    pub fn of_pci_vendor_id(vendor_id: u16) -> Option<Self> {
        match vendor_id {
            0x106b => Some(Self::Apple),
            0x1002 | 0x1022 | 0x1dd8 | 0x1924 => Some(Self::Amd),
            0x8086 | 0x8087 | 0x03e7 => Some(Self::Intel),
            0x0955 | 0x10de | 0x12d2 => Some(Self::Nvidia),
            0x1ed5 => Some(Self::MooreThreads),
            0x168c | 0x5143 => Some(Self::Qualcomm),
            0x14c3 => Some(Self::Mtk),
            0x15ad => Some(Self::Vmware),
            0x1ab8 => Some(Self::Parallel),
            0x1414 => Some(Self::Microsoft),
            0x182f | 0x14e4 => Some(Self::Broadcom),
            0x0014 => Some(Self::Loongson),
            0x0731 => Some(Self::JingjiaMicro),
            0x19e5 => Some(Self::Huawei),
            0x1d17 => Some(Self::Zhaoxin),
            _ => None,
        }
    }

    /// Reads the maker from the number the processor of the machine gives.
    ///
    /// The numbers are the ones the makers registered for the processors of
    /// the machines of the kind of ARM, and a number that no known maker
    /// registered is not read.
    pub fn of_arm_implementer_id(implementer_id: u8) -> Option<Self> {
        match implementer_id {
            0x41 => Some(Self::Arm),
            0x42 => Some(Self::Broadcom),
            0x48 => Some(Self::Huawei),
            0x4e => Some(Self::Nvidia),
            0x51 => Some(Self::Qualcomm),
            0x53 => Some(Self::Samsung),
            0x56 => Some(Self::Marvell),
            0x61 => Some(Self::Apple),
            0x69 => Some(Self::Intel),
            0x6d => Some(Self::Microsoft),
            0xc0 => Some(Self::Ampere),
            _ => None,
        }
    }
}

impl std::fmt::Display for HardwareVendor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.name())
    }
}

/// The graphics cards of the machine.
pub struct GraphicsCard;

impl GraphicsCard {
    /// Cleans the name of a card the way the launcher shows it.
    ///
    /// The machines write the names of the cards with marks of the trade and
    /// with the word `GPU`, which the launcher leaves out. The cards of
    /// Qualcomm are written with a long name that says the same for every
    /// card of the family, so the launcher writes the name of the family for
    /// them.
    pub fn clean_name(name: &str) -> String {
        let mut name = remove_trademark(name);
        name = name.replace(" GPU", "");

        if name.contains("Snapdragon") {
            name = crate::util::string::normalize_whitespaces(&name);
            if let Some(rest) = name.strip_prefix("Snapdragon ") {
                if is_snapdragon_x_elite(rest) {
                    name = "Qualcomm Adreno Graphics".to_string();
                }
            }
        }

        crate::util::string::normalize_whitespaces(&name)
    }
}

/// The processors of the machine.
pub struct CentralProcessor;

impl CentralProcessor {
    /// Cleans the name of a processor the way the launcher shows it.
    ///
    /// The machines write the names of the processors with the clock, with the
    /// marks of the trade and with the number of the cores, which the launcher
    /// leaves out. The names of the makers are written in one way each, so the
    /// launcher writes the name of the maker and the name of the processor in
    /// the order the maker uses.
    pub fn clean_name(name: &str) -> String {
        // The clock is left out, but only when the name does not begin with
        // it, because a name that begins with it is a name the launcher does
        // not read the clock out of.
        let mut name = match name.find('@') {
            Some(index) if index > 0 => name[..index].to_string(),
            _ => name.to_string(),
        };

        name = remove_core_count(&name);
        name = remove_words(&name, &[" CPU", " FPU", " APU", " Processor"]);
        name = remove_trademark(&name);

        if name.contains("Intel") {
            name = remove_intel_prefix(&name);
            name = name.replace("Core(TM)2", "Core 2");
        } else if name.contains("AMD") {
            name = name.replace("(tm)", "");
            if let Some(index) = find_any(
                &name,
                &[" w/ Radeon ", " with Radeon ", " with AMD Radeon "],
            ) {
                if index > 0 {
                    name = name[..index].to_string();
                }
            }
        } else if name.contains("Loongson") {
            name = remove_loongson_prefix(&name);
        } else if name.contains("Snapdragon") {
            name = crate::util::string::normalize_whitespaces(&name);

            if let Some(rest) = name.strip_prefix("Snapdragon ") {
                if let Some(identifier) = snapdragon_x_elite_identifier(rest) {
                    name = format!("Qualcomm Snapdragon X Elite {identifier}");
                } else if !name.contains("Qualcomm") {
                    name = format!("Qualcomm {name}");
                }
            }
        }

        crate::util::string::normalize_whitespaces(&name)
    }
}

/// Removes the count of the cores from the name of a processor.
///
/// The count is written as a number or as a word, followed by `-core` or
/// `-cores`, and only the first count is left out, which is what the launcher
/// of Java does.
fn remove_core_count(name: &str) -> String {
    // The count is read from the space that begins it, so a name that holds
    // the count in the middle of a word is left alone.
    for (index, _) in name.match_indices(' ') {
        if let Some(after) = strip_core_count(&name[index + 1..]) {
            let mut result = String::with_capacity(name.len());
            result.push_str(&name[..index]);
            result.push_str(after);
            return result;
        }
    }

    name.to_string()
}

/// Returns the rest of the name after the count of the cores, if the name
/// begins with one.
///
/// The count is a number or one of the words the launcher reads, followed by
/// `-core` or `-cores`, and the letters of the case of the name are not read,
/// so both `8-Core` and `8-core` are counts.
fn strip_core_count(name: &str) -> Option<&str> {
    let words = ["Dual", "Quad", "Six", "Eight", "Ten"];

    // The count is a number when the name begins with one, and a word when the
    // name begins with one of the words the launcher reads.
    let digits = name
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .count();

    let after_count = if digits > 0 {
        &name[digits..]
    } else {
        let word = words.iter().find(|word| {
            name.get(..word.len())
                .is_some_and(|beginning| beginning.eq_ignore_ascii_case(word))
        })?;
        &name[word.len()..]
    };

    // The count is followed by `-core` or `-cores`, and the letters of the
    // case of the name are not read.
    let rest = after_count.strip_prefix('-')?;
    for suffix in ["cores", "core"] {
        if rest
            .get(..suffix.len())
            .is_some_and(|beginning| beginning.eq_ignore_ascii_case(suffix))
        {
            return Some(&rest[suffix.len()..]);
        }
    }

    None
}

/// Removes the words that stand for themselves from a name.
fn remove_words(name: &str, words: &[&str]) -> String {
    let mut result = name.to_string();
    for word in words {
        result = result.replace(word, "");
    }
    result
}

/// Returns the place of the first of the texts that is in the name.
fn find_any(name: &str, texts: &[&str]) -> Option<usize> {
    texts.iter().filter_map(|text| name.find(text)).min()
}

/// Removes the generation and the maker from the name of a processor of Intel.
///
/// The generation is written as `12th Gen ` and the maker as `Intel `, and
/// the name of the processor follows them, so the launcher writes the maker
/// once and the name of the processor after it.
fn remove_intel_prefix(name: &str) -> String {
    let mut rest = name;

    // The maker is left out when it stands for itself, so the name of the
    // processor is written after the name of the maker.
    if let Some(after) = rest.strip_prefix("Intel ") {
        rest = after;
    } else {
        for generation in [
            "1st Gen ",
            "2nd Gen ",
            "3rd Gen ",
            "4th Gen ",
            "5th Gen ",
            "6th Gen ",
            "7th Gen ",
            "8th Gen ",
            "9th Gen ",
            "10th Gen ",
            "11th Gen ",
            "12th Gen ",
            "13th Gen ",
            "14th Gen ",
        ] {
            if let Some(after) = rest.strip_prefix(generation) {
                if let Some(after) = after.strip_prefix("Intel ") {
                    rest = after;
                }
                break;
            }
        }
    }

    format!("Intel {rest}")
}

/// Removes the name of the family from the name of a processor of Loongson.
///
/// The name is written as `Loongson-3A R3 (Loongson-3A3000)`, and the name of
/// the family is left out, so the name of the processor is the one in the
/// brackets.
fn remove_loongson_prefix(name: &str) -> String {
    if let Some(inner) = name
        .strip_prefix("Loongson-3A R")
        .and_then(|rest| rest.split_once(' '))
        .and_then(|(_, after)| after.strip_prefix('('))
        .and_then(|after| after.split_once(')'))
    {
        return inner.0.to_string();
    }

    name.to_string()
}

/// Returns the identifier of a processor of Snapdragon X Elite.
///
/// The name is written as `X Elite - X1E... - Qualcomm Oryon`, and the
/// identifier of the processor is the part that begins with `X1E`.
fn snapdragon_x_elite_identifier(name: &str) -> Option<&str> {
    let after_family = name.strip_prefix("X Elite - ")?;
    let identifier = after_family
        .split_once(' ')
        .map_or(after_family, |(identifier, _)| identifier);

    if identifier.starts_with("X1E") && identifier.len() > 3 {
        Some(identifier)
    } else {
        None
    }
}

/// Removes the marks of the trade that stand for themselves.
///
/// The marks are `(TM)` and `(R)`, and they are only left out when a space or
/// the end of the name follows them, so a mark that is a part of a word is
/// kept.
fn remove_trademark(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    let mut rest = name;

    while let Some(index) = rest.find('(') {
        result.push_str(&rest[..index]);

        let after = &rest[index..];
        // The mark is left out whole, and only when a space or the end of the
        // name follows it, so a mark that is a part of a word is kept.
        let mark = after
            .strip_prefix("(TM)")
            .or_else(|| after.strip_prefix("(R)"));

        match mark {
            Some(after) if after.is_empty() || after.starts_with(char::is_whitespace) => {
                rest = after;
            }
            _ => {
                // The mark is a part of the name, so the bracket is kept.
                result.push('(');
                rest = &after[1..];
            }
        }
    }

    result.push_str(rest);
    result
}

/// Returns whether the name is the one of a card of Snapdragon X Elite.
///
/// The name is written as `X Elite - <identifier> - Qualcomm Adreno`, where
/// the identifier begins with `X1E` and holds no space. The identifier of
/// every card is its own, and the launcher writes the name of the family for
/// all of them, so only the shape of the name is read.
fn is_snapdragon_x_elite(name: &str) -> bool {
    let Some(after_family) = name.strip_prefix("X Elite - ") else {
        return false;
    };
    let Some(after_identifier) = after_family.strip_prefix("X1E") else {
        return false;
    };

    // The identifier ends at the space that begins the rest of the name, and
    // it must not be empty.
    let (identifier, after) = match after_identifier.split_once(' ') {
        Some((identifier, after)) => (identifier, after),
        None => (after_identifier, ""),
    };

    !identifier.is_empty() && after == "- Qualcomm Adreno"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_name_of_the_maker_is_the_one_the_launcher_shows() {
        assert_eq!(HardwareVendor::Nvidia.name(), "NVIDIA");
        assert_eq!(HardwareVendor::Amd.name(), "AMD");
        assert_eq!(HardwareVendor::MooreThreads.name(), "Moore Threads");
        assert_eq!(
            HardwareVendor::Other("My Card".to_string()).name(),
            "My Card"
        );
        assert_eq!(HardwareVendor::Thead.to_string(), "T-Head");
    }

    #[test]
    fn the_makers_are_read_from_the_names_of_the_parts() {
        assert_eq!(
            HardwareVendor::of("Intel(R) UHD Graphics"),
            HardwareVendor::Intel
        );
        assert_eq!(HardwareVendor::of("GenuineIntel"), HardwareVendor::Intel);
        assert_eq!(
            HardwareVendor::of("NVIDIA GeForce RTX 4060"),
            HardwareVendor::Nvidia
        );
        assert_eq!(
            HardwareVendor::of("Advanced Micro Devices, Inc."),
            HardwareVendor::Amd
        );
        assert_eq!(HardwareVendor::of("AuthenticAMD"), HardwareVendor::Amd);
        assert_eq!(HardwareVendor::of("AMD Radeon"), HardwareVendor::Amd);
        // A name that begins with AMD and continues with letters is not read
        // as the maker, like the launcher of Java reads it.
        assert_eq!(
            HardwareVendor::of("AMDYZ Graphics"),
            HardwareVendor::Other("AMDYZ Graphics".to_string())
        );
        assert_eq!(HardwareVendor::of("brcm"), HardwareVendor::Broadcom);
        assert_eq!(HardwareVendor::of("Broadcom BCM"), HardwareVendor::Broadcom);
        assert_eq!(HardwareVendor::of("Apple M2"), HardwareVendor::Apple);
        assert_eq!(
            HardwareVendor::of("Hisilicon Kirin"),
            HardwareVendor::Huawei
        );
        assert_eq!(HardwareVendor::of("T-Head C906"), HardwareVendor::Thead);
        assert_eq!(HardwareVendor::of("SpacemiT X60"), HardwareVendor::Spacemit);
        // A maker the launcher does not know keeps its name.
        assert_eq!(
            HardwareVendor::of("My Own Card"),
            HardwareVendor::Other("My Own Card".to_string())
        );
    }

    #[test]
    fn the_maker_of_a_known_number_is_read() {
        assert_eq!(
            HardwareVendor::get_known("NVIDIA"),
            Some(HardwareVendor::Nvidia)
        );
        assert_eq!(HardwareVendor::get_known("My Card"), None);
    }

    #[test]
    fn the_makers_are_read_from_the_numbers_of_the_bus() {
        assert_eq!(
            HardwareVendor::of_pci_vendor_id(0x10de),
            Some(HardwareVendor::Nvidia)
        );
        assert_eq!(
            HardwareVendor::of_pci_vendor_id(0x8086),
            Some(HardwareVendor::Intel)
        );
        assert_eq!(
            HardwareVendor::of_pci_vendor_id(0x1002),
            Some(HardwareVendor::Amd)
        );
        assert_eq!(
            HardwareVendor::of_pci_vendor_id(0x106b),
            Some(HardwareVendor::Apple)
        );
        assert_eq!(
            HardwareVendor::of_pci_vendor_id(0x1ed5),
            Some(HardwareVendor::MooreThreads)
        );
        assert_eq!(
            HardwareVendor::of_pci_vendor_id(0x1d17),
            Some(HardwareVendor::Zhaoxin)
        );
        // A number no known maker registered is not read.
        assert_eq!(HardwareVendor::of_pci_vendor_id(0x0000), None);
        assert_eq!(HardwareVendor::of_pci_vendor_id(0xffff), None);
    }

    #[test]
    fn the_makers_are_read_from_the_numbers_of_the_processors() {
        assert_eq!(
            HardwareVendor::of_arm_implementer_id(0x41),
            Some(HardwareVendor::Arm)
        );
        assert_eq!(
            HardwareVendor::of_arm_implementer_id(0x61),
            Some(HardwareVendor::Apple)
        );
        assert_eq!(
            HardwareVendor::of_arm_implementer_id(0x69),
            Some(HardwareVendor::Intel)
        );
        assert_eq!(
            HardwareVendor::of_arm_implementer_id(0xc0),
            Some(HardwareVendor::Ampere)
        );
        // A number no known maker registered is not read.
        assert_eq!(HardwareVendor::of_arm_implementer_id(0x00), None);
        assert_eq!(HardwareVendor::of_arm_implementer_id(0xff), None);
    }

    #[test]
    fn the_names_of_the_processors_are_cleaned() {
        // The cases of the tests of the launcher of Java are kept.
        assert_eq!(
            CentralProcessor::clean_name("12th Gen Intel(R) Core(TM) i7-12700K"),
            "Intel Core i7-12700K"
        );
        assert_eq!(
            CentralProcessor::clean_name("Intel(R) Core(TM) Ultra 9 285K"),
            "Intel Core Ultra 9 285K"
        );
        assert_eq!(
            CentralProcessor::clean_name("Intel(R) Xeon(R) Platinum 8380 CPU @ 2.30GHz"),
            "Intel Xeon Platinum 8380"
        );
        assert_eq!(
            CentralProcessor::clean_name("Intel(R) Xeon(R) CPU E5-2660 v2 @ 2.20GHz"),
            "Intel Xeon E5-2660 v2"
        );
        assert_eq!(
            CentralProcessor::clean_name("Intel(R) Xeon Phi(TM) CPU 7250 @ 1.40GHz"),
            "Intel Xeon Phi 7250"
        );
        assert_eq!(
            CentralProcessor::clean_name("Intel(R) Celeron(R) N5105 @ 2.00GHz"),
            "Intel Celeron N5105"
        );
        assert_eq!(
            CentralProcessor::clean_name("Intel(R) Pentium(R) Silver J5005 CPU @ 1.50GHz"),
            "Intel Pentium Silver J5005"
        );
        assert_eq!(
            CentralProcessor::clean_name("Intel(R) Atom(TM) Processor E3940 @ 1.60GHz"),
            "Intel Atom E3940"
        );
        assert_eq!(
            CentralProcessor::clean_name("Intel(R) Core(TM) i7 CPU       X 990  @ 3.47GHz"),
            "Intel Core i7 X 990"
        );
        assert_eq!(
            CentralProcessor::clean_name("Intel(R) Core(TM)2 Duo CPU     T7500  @ 2.20GHz"),
            "Intel Core 2 Duo T7500"
        );
        assert_eq!(
            CentralProcessor::clean_name("Intel(R) Core(TM)2 Quad CPU    Q9500  @ 2.83GHz"),
            "Intel Core 2 Quad Q9500"
        );
        assert_eq!(
            CentralProcessor::clean_name("AMD Ryzen 7 7840HS w/ Radeon 780M Graphics"),
            "AMD Ryzen 7 7840HS"
        );
        assert_eq!(
            CentralProcessor::clean_name("AMD Ryzen 7 6800U with Radeon Graphics"),
            "AMD Ryzen 7 6800U"
        );
        assert_eq!(
            CentralProcessor::clean_name("AMD Ryzen 7 5800X 8-Core Processor"),
            "AMD Ryzen 7 5800X"
        );
        assert_eq!(
            CentralProcessor::clean_name("AMD Ryzen 5 2400G with Radeon Vega Graphics"),
            "AMD Ryzen 5 2400G"
        );
        assert_eq!(
            CentralProcessor::clean_name("AMD EPYC 7713 64-Core Processor"),
            "AMD EPYC 7713"
        );
        assert_eq!(
            CentralProcessor::clean_name("AMD Ryzen Threadripper 3960X 24-Core Processor"),
            "AMD Ryzen Threadripper 3960X"
        );
        assert_eq!(
            CentralProcessor::clean_name("AMD Ryzen Threadripper PRO 5995WX 64-Cores"),
            "AMD Ryzen Threadripper PRO 5995WX"
        );
        assert_eq!(
            CentralProcessor::clean_name("AMD Ryzen Embedded V2748 with Radeon Graphics"),
            "AMD Ryzen Embedded V2748"
        );
        assert_eq!(
            CentralProcessor::clean_name("AMD A8-7410 APU with AMD Radeon R5 Graphics"),
            "AMD A8-7410"
        );
        assert_eq!(
            CentralProcessor::clean_name("AMD FX(tm)-8350 Eight-Core Processor"),
            "AMD FX-8350"
        );
        assert_eq!(
            CentralProcessor::clean_name("AMD Phenom(tm) II X6 1055T Processor"),
            "AMD Phenom II X6 1055T"
        );
        assert_eq!(
            CentralProcessor::clean_name("AMD Athlon(tm) 5350 APU with Radeon(tm) R3"),
            "AMD Athlon 5350"
        );
        assert_eq!(
            CentralProcessor::clean_name(
                "Snapdragon(R) X Elite - X1E78100 - Qualcomm(R) Oryon(TM) CPU"
            ),
            "Qualcomm Snapdragon X Elite X1E78100"
        );
        assert_eq!(
            CentralProcessor::clean_name("Snapdragon (TM) 850 @ 2.96 GHz"),
            "Qualcomm Snapdragon 850"
        );
        assert_eq!(
            CentralProcessor::clean_name("Hygon C86 7285 32-core Processor"),
            "Hygon C86 7285"
        );
        assert_eq!(
            CentralProcessor::clean_name("Hygon C86 3250  8-core Processor"),
            "Hygon C86 3250"
        );
        assert_eq!(
            CentralProcessor::clean_name("ZHAOXIN KaiXian KX-6640MA@2.2+GHz"),
            "ZHAOXIN KaiXian KX-6640MA"
        );
        assert_eq!(
            CentralProcessor::clean_name("ZHAOXIN KaiXian KX-U6780A@2.7GHz"),
            "ZHAOXIN KaiXian KX-U6780A"
        );
        assert_eq!(
            CentralProcessor::clean_name("ZHAOXIN KaiSheng KH-40000/16@2.2GHz"),
            "ZHAOXIN KaiSheng KH-40000/16"
        );
        assert_eq!(
            CentralProcessor::clean_name("ZHAOXIN KaiSheng KH-37800D@2.7GHz"),
            "ZHAOXIN KaiSheng KH-37800D"
        );
        assert_eq!(
            CentralProcessor::clean_name("Loongson-3A R3 (Loongson-3A3000) @ 1400MHz"),
            "Loongson-3A3000"
        );
        assert_eq!(
            CentralProcessor::clean_name("Loongson-3A R4 (Loongson-3B4000) @ 1800MHz"),
            "Loongson-3B4000"
        );
        assert_eq!(
            CentralProcessor::clean_name("Loongson-3A6000"),
            "Loongson-3A6000"
        );
    }
    #[test]
    fn the_marks_of_the_trade_are_left_out_of_the_names() {
        assert_eq!(
            GraphicsCard::clean_name("Intel(R) UHD Graphics"),
            "Intel UHD Graphics"
        );
        assert_eq!(
            GraphicsCard::clean_name("NVIDIA(TM) GeForce"),
            "NVIDIA GeForce"
        );
        // A mark that stands inside a word is kept.
        assert_eq!(GraphicsCard::clean_name("ABC(R)DEF"), "ABC(R)DEF");
        assert_eq!(GraphicsCard::clean_name("ABC(R) DEF"), "ABC DEF");
    }

    #[test]
    fn the_word_of_the_graphics_is_left_out_of_the_names() {
        assert_eq!(GraphicsCard::clean_name("Intel UHD GPU"), "Intel UHD");
        // Only the word that stands for itself is left out.
        assert_eq!(GraphicsCard::clean_name("GPU Intel"), "GPU Intel");
    }

    #[test]
    fn the_runs_of_white_space_are_written_as_one_space() {
        assert_eq!(
            GraphicsCard::clean_name("Intel   UHD\tGraphics"),
            "Intel UHD Graphics"
        );
        assert_eq!(GraphicsCard::clean_name("  Intel UHD  "), "Intel UHD");
    }

    #[test]
    fn the_long_names_of_the_cards_of_qualcomm_are_written_as_the_family() {
        assert_eq!(
            GraphicsCard::clean_name("Snapdragon X Elite - X1E-78-100 - Qualcomm Adreno"),
            "Qualcomm Adreno Graphics"
        );
        assert_eq!(
            GraphicsCard::clean_name("Snapdragon X Elite - X1E8400 - Qualcomm Adreno"),
            "Qualcomm Adreno Graphics"
        );
        // A card of another family keeps its name.
        assert_eq!(
            GraphicsCard::clean_name("Snapdragon 8 Gen 3 - Qualcomm Adreno"),
            "Snapdragon 8 Gen 3 - Qualcomm Adreno"
        );
        // A name that does not hold the family is kept as it is.
        assert_eq!(
            GraphicsCard::clean_name("Snapdragon X Elite - X1E - Qualcomm Adreno"),
            "Snapdragon X Elite - X1E - Qualcomm Adreno"
        );
        // The identifier holds no space, so a name whose identifier is broken
        // in two is not one of the family.
        assert_eq!(
            GraphicsCard::clean_name("Snapdragon X Elite - X1E 78-100 - Qualcomm Adreno"),
            "Snapdragon X Elite - X1E 78-100 - Qualcomm Adreno"
        );
    }
}
