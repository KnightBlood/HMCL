//! The details of a version of the game.
//!
//! The servers of the game publish the details of a version in a file; the
//! file holds the libraries the version needs, the files that are downloaded
//! and the arguments the game is started with.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::artifact::Artifact;
use super::release_type::ReleaseType;

/// The details of a version of the game.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionMetadata {
    /// The identifier of the version.
    pub id: String,
    /// The kind of the version.
    #[serde(rename = "type")]
    pub version_type: ReleaseType,
    /// The moment the version was released.
    #[serde(default, rename = "releaseTime")]
    pub release_time: Option<String>,
    /// The moment the version was published.
    #[serde(default)]
    pub time: Option<String>,
    /// The level the version asks the launcher to obey.
    #[serde(default, rename = "complianceLevel")]
    pub compliance_level: Option<i32>,
    /// The arguments the client writes into its log.
    #[serde(default)]
    pub logging: Option<Logging>,
    /// The version of Java the version needs.
    #[serde(default, rename = "javaVersion")]
    pub java_version: Option<JavaVersion>,
    /// The name of the assets of the version.
    #[serde(default)]
    pub assets: Option<String>,
    /// The index of the assets of the version.
    #[serde(default, rename = "assetIndex")]
    pub asset_index: Option<AssetIndex>,
    /// The class the game is started with.
    #[serde(default, rename = "mainClass")]
    pub main_class: Option<String>,
    /// The arguments of the game, in the old format.
    #[serde(default, rename = "minecraftArguments")]
    pub minecraft_arguments: Option<String>,
    /// The arguments of the game and of the virtual machine.
    #[serde(default)]
    pub arguments: Option<Arguments>,
    /// The libraries the version needs.
    #[serde(default)]
    pub libraries: Vec<Library>,
    /// The files that are downloaded for the version.
    #[serde(default)]
    pub downloads: Option<Downloads>,
}

impl VersionMetadata {
    /// Returns the arguments of the game.
    ///
    /// The versions that use the old format have the arguments in one text.
    pub fn game_arguments(&self) -> Vec<&Argument> {
        if let Some(arguments) = &self.arguments {
            if let Some(game) = &arguments.game {
                return game.iter().collect();
            }
        }
        Vec::new()
    }
}

/// The arguments of the game and of the virtual machine.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Arguments {
    /// The arguments of the game.
    #[serde(default)]
    pub game: Option<Vec<Argument>>,
    /// The arguments of the virtual machine.
    #[serde(default)]
    pub jvm: Option<Vec<Argument>>,
}

impl Arguments {
    /// Returns the arguments of two versions.
    ///
    /// The arguments of the parent are kept first, so the arguments of the
    /// instance are read last and may replace the arguments of the parent.
    pub fn merge(parent: Option<&Arguments>, child: Option<&Arguments>) -> Option<Arguments> {
        if parent.is_none() && child.is_none() {
            return None;
        }

        Some(Arguments {
            game: merge_arguments(
                parent.and_then(|arguments| arguments.game.as_ref()),
                child.and_then(|arguments| arguments.game.as_ref()),
            ),
            jvm: merge_arguments(
                parent.and_then(|arguments| arguments.jvm.as_ref()),
                child.and_then(|arguments| arguments.jvm.as_ref()),
            ),
        })
    }
}

/// Joins two lists of arguments, the first list first.
fn merge_arguments(
    first: Option<&Vec<Argument>>,
    second: Option<&Vec<Argument>>,
) -> Option<Vec<Argument>> {
    match (first, second) {
        (None, None) => None,
        _ => {
            let mut arguments = first.cloned().unwrap_or_default();
            arguments.extend(second.cloned().unwrap_or_default());
            Some(arguments)
        }
    }
}

/// An argument of the game or of the virtual machine.
///
/// The arguments are written in two ways: as a text, or as an object that
/// holds the rules that tell whether the argument is used.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Argument {
    /// The text of the argument.
    Text(String),
    /// The argument and the rules that tell whether it is used.
    Detailed(ArgumentDetails),
}

impl Argument {
    /// Returns the text of the argument, if it is one text.
    ///
    /// An argument may hold several texts; [`Argument::values`] returns all of
    /// them.
    pub fn value(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            Self::Detailed(details) => details.value.as_ref().and_then(ArgumentValue::first),
        }
    }

    /// Returns the texts of the argument.
    ///
    /// An argument that is written as one text holds one text; an argument
    /// without texts holds none.
    pub fn values(&self) -> Vec<&str> {
        match self {
            Self::Text(text) => vec![text.as_str()],
            Self::Detailed(details) => details
                .value
                .as_ref()
                .map(ArgumentValue::values)
                .unwrap_or_default(),
        }
    }

    /// Returns the rules that tell whether the argument is used.
    pub fn rules(&self) -> Option<&[CompatibilityRule]> {
        match self {
            Self::Text(_) => None,
            Self::Detailed(details) => details.rules.as_deref(),
        }
    }
}

/// The texts of an argument of the game or of the virtual machine.
///
/// The versions write the texts either as one text or as a list of texts; the
/// list is used by the arguments that take several values at once.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArgumentValue {
    /// One text.
    Text(String),
    /// Several texts.
    Texts(Vec<String>),
}

impl ArgumentValue {
    /// Returns the first of the texts.
    fn first(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            Self::Texts(texts) => texts.first().map(String::as_str),
        }
    }

    /// Returns the texts.
    fn values(&self) -> Vec<&str> {
        match self {
            Self::Text(text) => vec![text.as_str()],
            Self::Texts(texts) => texts.iter().map(String::as_str).collect(),
        }
    }
}

/// The details of an argument of the game or of the virtual machine.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ArgumentDetails {
    /// The rules that tell whether the argument is used.
    #[serde(default, rename = "rules")]
    pub rules: Option<Vec<CompatibilityRule>>,
    /// The texts of the argument.
    ///
    /// The versions write the texts either as `value` or as `values`.
    #[serde(default, alias = "values")]
    pub value: Option<ArgumentValue>,
}

/// A rule that tells whether a part of a version is used.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityRule {
    /// Whether the part is used or not.
    pub action: RuleAction,
    /// The systems the rule is about; the rule is about every system when it
    /// is not given.
    #[serde(default)]
    pub os: Option<OsRule>,
    /// The features the rule is about.
    ///
    /// A rule about a feature applies when the game is started with the same
    /// value of the feature, so a version may add arguments only when the
    /// player plays the demo or sets the resolution of the window.
    #[serde(default)]
    pub features: Option<BTreeMap<String, bool>>,
}

impl CompatibilityRule {
    /// Returns whether the rule is about the environment.
    ///
    /// A rule is about the environment when the system it names is the system
    /// the launcher runs on and when the features it names are the features
    /// the game is started with.
    pub fn applies(&self, context: &RuleContext<'_>) -> bool {
        if let Some(os) = &self.os {
            if !os.allows(context) {
                return false;
            }
        }

        if let Some(features) = &self.features {
            if features
                .iter()
                .any(|(name, value)| context.features.get(name) != Some(value))
            {
                return false;
            }
        }

        true
    }
}

/// The environment the rules of a version are read in.
#[derive(Debug, Clone, Copy)]
pub struct RuleContext<'a> {
    /// The name of the operating system, such as `windows`.
    pub os_name: &'a str,
    /// The version of the operating system; the version is not compared when
    /// it is empty, so the rules about the versions of the systems stay.
    pub os_version: &'a str,
    /// The architecture of the machine, such as `x86_64`.
    pub arch: &'a str,
    /// The features the game is started with.
    pub features: &'a BTreeMap<String, bool>,
}

impl<'a> RuleContext<'a> {
    /// Creates the environment the rules are read in.
    pub fn new(
        os_name: &'a str,
        os_version: &'a str,
        arch: &'a str,
        features: &'a BTreeMap<String, bool>,
    ) -> Self {
        Self {
            os_name,
            os_version,
            arch,
            features,
        }
    }
}

/// Returns whether the parts with the rules are used.
///
/// The rules are read in the order they are written; the part is not used
/// until a rule about the environment allows it, so a part with rules the
/// launcher does not know is not used. A part without rules is used.
pub fn rules_allow(rules: Option<&[CompatibilityRule]>, context: &RuleContext<'_>) -> bool {
    let Some(rules) = rules else {
        return true;
    };
    if rules.is_empty() {
        return true;
    }

    let mut allow = false;
    for rule in rules {
        if rule.applies(context) {
            allow = rule.action == RuleAction::Allow;
        }
    }
    allow
}

/// Whether a part of a version is used or not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleAction {
    /// The part is used.
    Allow,
    /// The part is not used.
    Disallow,
}

/// The systems a rule is about.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct OsRule {
    /// The name of the system, such as `windows`; the rule is about every
    /// system when it is not given.
    #[serde(default)]
    pub name: Option<String>,
    /// The version of the system the rule is about.
    #[serde(default)]
    pub version: Option<String>,
    /// The architecture the rule is about.
    #[serde(default)]
    pub arch: Option<String>,
}

impl OsRule {
    /// Returns whether the system of the environment is the system the rule is
    /// about.
    ///
    /// The modpacks sometimes write the name of the system and the
    /// architecture together, such as `win-x86`; the architectures are read
    /// the way the versions of Java name them.
    pub fn allows(&self, context: &RuleContext<'_>) -> bool {
        if let Some(name) = &self.name {
            if let Some((os, arch)) = name.split_once('-') {
                if let (Some(os), Some(arch)) = (parse_os_name(os), parse_arch(arch)) {
                    return os == context.os_name && Some(arch) == parse_arch(context.arch);
                }
            }

            if let Some(os) = parse_os_name(name) {
                if os != context.os_name {
                    return false;
                }
            }
        }

        // The version of the system is only compared when it is read.
        if let Some(version) = &self.version {
            if !context.os_version.is_empty() && !matches_pattern(version, context.os_version) {
                return false;
            }
        }

        if let Some(arch) = &self.arch {
            let matches = match parse_arch(arch) {
                Some(expected) => Some(expected) == parse_arch(context.arch),
                None => matches_pattern(arch, context.arch),
            };
            if !matches {
                return false;
            }
        }

        true
    }
}

/// Returns the name of the system a rule uses, in the names of the launcher.
fn parse_os_name(name: &str) -> Option<&'static str> {
    let name = name.to_ascii_lowercase();
    if name.starts_with("win") {
        Some("windows")
    } else if name == "osx" || name.starts_with("mac") {
        Some("osx")
    } else if name.starts_with("linux") {
        Some("linux")
    } else {
        // The rules about the systems the launcher does not know do not
        // reject the system the launcher runs on.
        None
    }
}

/// Returns the name of the architecture a rule uses, in the names of the
/// launcher.
fn parse_arch(arch: &str) -> Option<&'static str> {
    let arch = arch.to_ascii_lowercase();
    match arch.as_str() {
        "x86_64" | "amd64" | "x64" => Some("x86_64"),
        "x86" | "i386" | "i686" | "ia32" => Some("x86"),
        "arm64" | "aarch64" => Some("aarch64"),
        "arm" | "arm32" | "armv7" | "armv7l" => Some("arm"),
        _ => None,
    }
}

/// Returns whether the text matches the pattern of a rule.
///
/// The patterns of the rules are regular expressions; the launcher reads the
/// parts the versions use: the anchors `^` and `$`, the wildcard `.`, the
/// classes, the escapes and the quantifiers `*`, `+` and `?`. A pattern that
/// ends with an escaped dot is read as the beginning of the version, because
/// the versions use it to tell the versions of the systems apart.
fn matches_pattern(pattern: &str, text: &str) -> bool {
    let pattern = pattern.strip_prefix('^').unwrap_or(pattern);
    let pattern = pattern.strip_suffix('$').unwrap_or(pattern);

    if let Some(prefix) = pattern.strip_suffix("\\.") {
        return text.starts_with(&prefix.replace("\\.", "."));
    }

    match_atoms(&parse_pattern(pattern), &text.chars().collect::<Vec<_>>())
}

/// A part of a pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AtomKind {
    /// One character.
    Literal(char),
    /// Any character.
    Any,
    /// One of the characters of a class.
    Class {
        /// Whether the characters of the class are the ones that do not match.
        negated: bool,
    },
}

/// How often a part of a pattern is repeated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Repeat {
    /// Once.
    One,
    /// At most once.
    Optional,
    /// Any number of times.
    ZeroOrMore,
    /// At least once.
    OneOrMore,
}

/// A part of a pattern.
#[derive(Debug, Clone)]
struct Atom {
    /// The part itself.
    kind: AtomKind,
    /// The characters of a class.
    ranges: Vec<(char, char)>,
    /// How often the part is repeated.
    repeat: Repeat,
}

/// Reads a pattern into its parts.
fn parse_pattern(pattern: &str) -> Vec<Atom> {
    let characters: Vec<char> = pattern.chars().collect();
    let mut atoms = Vec::new();
    let mut index = 0;

    while index < characters.len() {
        let mut ranges = Vec::new();
        let kind = match characters[index] {
            '\\' => {
                index += 1;
                AtomKind::Literal(characters.get(index).copied().unwrap_or('\\'))
            }
            '.' => AtomKind::Any,
            '[' => {
                index += 1;
                let negated = characters.get(index) == Some(&'^');
                if negated {
                    index += 1;
                }
                while index < characters.len() && characters[index] != ']' {
                    let start = characters[index];
                    if characters.get(index + 1) == Some(&'-')
                        && characters.get(index + 2).is_some_and(|end| *end != ']')
                    {
                        ranges.push((start, characters[index + 2]));
                        index += 3;
                    } else {
                        ranges.push((start, start));
                        index += 1;
                    }
                }
                AtomKind::Class { negated }
            }
            character => AtomKind::Literal(character),
        };

        index += 1;
        let repeat = match characters.get(index) {
            Some('*') => {
                index += 1;
                Repeat::ZeroOrMore
            }
            Some('+') => {
                index += 1;
                Repeat::OneOrMore
            }
            Some('?') => {
                index += 1;
                Repeat::Optional
            }
            _ => Repeat::One,
        };

        atoms.push(Atom {
            kind,
            ranges,
            repeat,
        });
    }

    atoms
}

/// Returns whether the parts of a pattern match the text.
fn match_atoms(atoms: &[Atom], text: &[char]) -> bool {
    let Some((atom, rest)) = atoms.split_first() else {
        return text.is_empty();
    };

    let matches_at = |index: usize| {
        text.get(index)
            .is_some_and(|character| atom_matches(atom, *character))
    };

    match atom.repeat {
        Repeat::One => matches_at(0) && match_atoms(rest, &text[1..]),
        Repeat::Optional => {
            match_atoms(rest, text) || (matches_at(0) && match_atoms(rest, &text[1..]))
        }
        Repeat::ZeroOrMore => {
            let mut count = 0;
            while matches_at(count) {
                count += 1;
            }
            (0..=count)
                .rev()
                .any(|used| match_atoms(rest, &text[used..]))
        }
        Repeat::OneOrMore => {
            if !matches_at(0) {
                return false;
            }

            let mut count = 1;
            while matches_at(count) {
                count += 1;
            }
            (1..=count)
                .rev()
                .any(|used| match_atoms(rest, &text[used..]))
        }
    }
}

/// Returns whether the part matches the character.
fn atom_matches(atom: &Atom, character: char) -> bool {
    match atom.kind {
        AtomKind::Literal(literal) => literal == character,
        AtomKind::Any => true,
        AtomKind::Class { negated } => {
            let inside = atom
                .ranges
                .iter()
                .any(|(start, end)| (*start..=*end).contains(&character));
            inside != negated
        }
    }
}

/// A library the version needs.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Library {
    /// The name of the library, as the repositories of Java name it.
    pub name: String,
    /// The address the library is downloaded from.
    #[serde(default)]
    pub url: Option<String>,
    /// The files of the library.
    #[serde(default)]
    pub downloads: Option<LibraryDownloads>,
    /// The digests of the files of the library.
    #[serde(default)]
    pub checksums: Option<Vec<String>>,
    /// The files of the library that are not extracted.
    #[serde(default)]
    pub extract: Option<ExtractRules>,
    /// The files of the library that hold the native libraries.
    #[serde(default)]
    pub natives: Option<BTreeMap<String, String>>,
    /// The rules that tell whether the library is used.
    #[serde(default)]
    pub rules: Option<Vec<CompatibilityRule>>,
    /// The hint of the library.
    #[serde(default, alias = "MMC-hint")]
    pub hint: Option<String>,
    /// The name of the file of the library.
    #[serde(default, alias = "MMC-filename")]
    pub filename: Option<String>,
}

/// The files of a library.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct LibraryDownloads {
    /// The file of the library.
    pub artifact: Option<DownloadInfo>,
    /// The files of the native libraries.
    #[serde(default)]
    pub classifiers: Option<BTreeMap<String, DownloadInfo>>,
}

impl Library {
    /// Returns the coordinates of the library.
    ///
    /// The name of a library is the way the repositories of Java name it.
    pub fn artifact(&self) -> Result<Artifact, String> {
        Artifact::from_descriptor(&self.name)
    }
}

/// The rules that tell which files of a library are not extracted.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ExtractRules {
    /// The files that are not extracted.
    #[serde(default)]
    pub exclude: Vec<String>,
}

impl ExtractRules {
    /// Returns whether the file is extracted.
    pub fn should_extract(&self, path: &str) -> bool {
        self.exclude
            .iter()
            .all(|excluded| !path.starts_with(excluded))
    }
}

/// A file that is downloaded.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DownloadInfo {
    /// The address of the file.
    pub url: String,
    /// The path of the file in the directory of the libraries.
    ///
    /// The versions of the game tell the path of the files of their libraries;
    /// the place of the file is the path the version tells, so the launcher
    /// reads it instead of reading the name of the library.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// The digest of the file.
    #[serde(default)]
    pub sha1: Option<String>,
    /// The size of the file.
    #[serde(default)]
    pub size: Option<u64>,
}

impl DownloadInfo {
    /// Returns the digest of the file, if the version tells one.
    ///
    /// A few versions of the game write the text `invalid` instead of a digest,
    /// because the file is downloaded from a server that does not publish its
    /// digest; the launcher reads such a file as a file whose digest it does not
    /// know, so the file is not checked against a digest that is not there.
    pub fn sha1(&self) -> Option<&str> {
        self.sha1
            .as_deref()
            .filter(|sha1| !sha1.is_empty() && *sha1 != "invalid")
    }
}

/// The index of the assets of a version.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AssetIndex {
    /// The identifier of the index.
    #[serde(default)]
    pub id: Option<String>,
    /// The digest of the file of the index.
    #[serde(default)]
    pub sha1: Option<String>,
    /// The size of the file of the index.
    #[serde(default)]
    pub size: Option<u64>,
    /// The total size of the assets.
    #[serde(default, rename = "totalSize")]
    pub total_size: Option<u64>,
    /// The address of the file of the index.
    #[serde(default)]
    pub url: Option<String>,
}

/// The files that are downloaded for a version.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Downloads {
    /// The file of the client.
    #[serde(default)]
    pub client: Option<DownloadInfo>,
    /// The file of the server.
    #[serde(default)]
    pub server: Option<DownloadInfo>,
    /// The file that holds the names of the client.
    #[serde(default, rename = "client_mappings")]
    pub client_mappings: Option<DownloadInfo>,
    /// The file that holds the names of the server.
    #[serde(default, rename = "server_mappings")]
    pub server_mappings: Option<DownloadInfo>,
}

/// The version of Java a version needs.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct JavaVersion {
    /// The name of the component.
    #[serde(default)]
    pub component: Option<String>,
    /// The main version of Java.
    #[serde(default, rename = "majorVersion")]
    pub major_version: Option<i32>,
}

/// The arguments the client writes into its log.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Logging {
    /// The file the client writes into.
    #[serde(default)]
    pub client: Option<LoggingArgument>,
    /// The file the server writes into.
    #[serde(default)]
    pub server: Option<LoggingArgument>,
}

/// The argument of the file the client writes into.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoggingArgument {
    /// The text of the argument.
    pub argument: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    const METADATA: &str = r#"{
        "id": "1.20.1",
        "type": "release",
        "releaseTime": "2023-06-12T13:25:51+00:00",
        "time": "2023-06-12T13:25:51+00:00",
        "complianceLevel": 1,
        "logging": {
            "client": { "argument": "--logPath ${assets_index}/logs" }
        },
        "javaVersion": { "component": "java-runtime-gamma", "majorVersion": 17 },
        "assets": "1.20",
        "assetIndex": {
            "id": "1.20",
            "sha1": "abc",
            "size": 1234,
            "totalSize": 5678,
            "url": "https://piston-meta.mojang.com/v1/packages/zzz/1.20.json"
        },
        "mainClass": "net.minecraft.client.main.Main",
        "minecraftArguments": "--username ${auth_player_name}",
        "libraries": [
            {
                "name": "com.example:library:1.0",
                "downloads": {
                    "artifact": {
                        "path": "com/example/library/1.0/library-1.0.jar",
                        "sha1": "def",
                        "size": 42,
                        "url": "https://libraries.minecraft.net/library-1.0.jar"
                    }
                },
                "extract": { "exclude": [ "META-INF/" ] },
                "rules": [
                    { "action": "allow", "os": { "name": "osx", "version": "10.5", "arch": "x86_64" } },
                    { "action": "disallow", "os": { "name": "windows" } }
                ]
            }
        ],
        "downloads": {
            "client": { "sha1": "111", "size": 100, "url": "https://client.jar" },
            "server": { "sha1": "222", "size": 200, "url": "https://server.jar" }
        }
    }"#;

    #[test]
    fn the_details_of_a_version_are_read() {
        let metadata: VersionMetadata =
            serde_json::from_str(METADATA).expect("the details should be read");

        assert_eq!(metadata.id, "1.20.1");
        assert_eq!(metadata.version_type, ReleaseType::Release);
        assert_eq!(metadata.compliance_level, Some(1));
        assert_eq!(
            metadata.main_class.as_deref(),
            Some("net.minecraft.client.main.Main")
        );
        assert_eq!(
            metadata
                .java_version
                .as_ref()
                .and_then(|java| java.major_version),
            Some(17)
        );
        assert_eq!(metadata.assets.as_deref(), Some("1.20"));
        assert_eq!(
            metadata
                .asset_index
                .as_ref()
                .and_then(|index| index.total_size),
            Some(5678)
        );
        assert_eq!(metadata.libraries.len(), 1);
        assert_eq!(metadata.libraries[0].name, "com.example:library:1.0");
        assert_eq!(
            metadata.libraries[0]
                .artifact()
                .expect("the coordinates should be read")
                .path(),
            "com/example/library/1.0/library-1.0.jar"
        );
        assert_eq!(
            metadata.libraries[0]
                .downloads
                .as_ref()
                .and_then(|downloads| downloads.artifact.as_ref())
                .and_then(|artifact| artifact.size),
            Some(42)
        );
        assert_eq!(
            metadata
                .downloads
                .as_ref()
                .and_then(|downloads| downloads.client.as_ref())
                .and_then(|client| client.size),
            Some(100)
        );
    }

    #[test]
    fn the_rules_of_the_libraries_are_read() {
        let metadata: VersionMetadata =
            serde_json::from_str(METADATA).expect("the details should be read");

        let rules = metadata.libraries[0]
            .rules
            .as_ref()
            .expect("the rules should be read");
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].action, RuleAction::Allow);
        assert_eq!(
            rules[0].os.as_ref().and_then(|os| os.name.as_deref()),
            Some("osx")
        );
        assert_eq!(rules[1].action, RuleAction::Disallow);
    }

    #[test]
    fn the_files_that_are_not_extracted_are_read() {
        let metadata: VersionMetadata =
            serde_json::from_str(METADATA).expect("the details should be read");

        let extract = metadata.libraries[0]
            .extract
            .as_ref()
            .expect("the rules should be read");
        assert!(!extract.should_extract("META-INF/MANIFEST.MF"));
        assert!(extract.should_extract("com/example/library/1.0/library-1.0.jar"));
    }

    #[test]
    fn the_arguments_of_the_game_are_read() {
        let metadata: VersionMetadata = serde_json::from_str(
            r#"{
                "id": "1.20.1",
                "type": "release",
                "arguments": {
                    "game": [
                        "--username",
                        { "value": "${auth_player_name}" },
                        { "rules": [ { "action": "allow", "os": { "name": "osx" } } ], "value": "-XstartOnFirstThread" }
                    ],
                    "jvm": [ "-Xmx2G" ]
                }
            }"#,
        )
        .expect("the details should be read");

        let arguments = metadata
            .arguments
            .as_ref()
            .expect("the arguments should be read");
        assert_eq!(arguments.game.as_ref().map(Vec::len), Some(3));
        assert_eq!(arguments.jvm.as_ref().map(Vec::len), Some(1));
        assert_eq!(metadata.game_arguments().len(), 3);

        let game = arguments
            .game
            .as_ref()
            .expect("the arguments should be read");
        assert_eq!(game[0].value(), Some("--username"));
        assert_eq!(game[2].rules().map(<[CompatibilityRule]>::len), Some(1));
    }

    #[test]
    fn the_rules_are_read_in_order() {
        let features = BTreeMap::new();
        let context = RuleContext::new("windows", "10.0.19045", "x86_64", &features);

        // A part without rules is used.
        assert!(rules_allow(None, &context));
        assert!(rules_allow(Some(&[]), &context));

        let rules = [CompatibilityRule {
            action: RuleAction::Disallow,
            os: None,
            features: None,
        }];
        assert!(!rules_allow(Some(&rules), &context));

        let rules = [
            CompatibilityRule {
                action: RuleAction::Allow,
                os: Some(OsRule {
                    name: Some("osx".to_string()),
                    version: None,
                    arch: None,
                }),
                features: None,
            },
            CompatibilityRule {
                action: RuleAction::Allow,
                os: Some(OsRule {
                    name: Some("windows".to_string()),
                    version: None,
                    arch: None,
                }),
                features: None,
            },
        ];

        // Only the rules about the system of the launcher are read.
        assert!(rules_allow(Some(&rules), &context));
        assert!(!rules_allow(Some(&rules[..1]), &context));
    }

    #[test]
    fn the_versions_of_the_systems_are_read() {
        let features = BTreeMap::new();
        let context = RuleContext::new("windows", "10.0.19045", "x86_64", &features);

        let version_10 = OsRule {
            name: Some("windows".to_string()),
            version: Some("^10\\.".to_string()),
            arch: None,
        };
        assert!(version_10.allows(&context));

        let version_6 = OsRule {
            name: Some("windows".to_string()),
            version: Some("^6\\.".to_string()),
            arch: None,
        };
        assert!(!version_6.allows(&context));

        // The version is not compared when the launcher does not know it.
        let unknown = RuleContext::new("windows", "", "x86_64", &features);
        assert!(version_6.allows(&unknown));
    }

    #[test]
    fn the_features_of_the_rules_are_read() {
        let rule = CompatibilityRule {
            action: RuleAction::Allow,
            os: None,
            features: Some(BTreeMap::from([("is_demo_user".to_string(), true)])),
        };

        let enabled = BTreeMap::from([("is_demo_user".to_string(), true)]);
        let context = RuleContext::new("windows", "10.0", "x86_64", &enabled);
        assert!(rule.applies(&context));

        let disabled = BTreeMap::new();
        let context = RuleContext::new("windows", "10.0", "x86_64", &disabled);
        assert!(!rule.applies(&context));
    }

    #[test]
    fn the_name_and_the_architecture_are_read_together() {
        let features = BTreeMap::new();
        let context = RuleContext::new("windows", "10.0", "x86", &features);

        let rule = OsRule {
            name: Some("win-x86".to_string()),
            version: None,
            arch: None,
        };
        assert!(rule.allows(&context));

        let rule = OsRule {
            name: Some("win-x86_64".to_string()),
            version: None,
            arch: None,
        };
        assert!(!rule.allows(&context));
    }

    #[test]
    fn the_patterns_of_the_rules_are_read() {
        assert!(matches_pattern(".*", "anything"));
        assert!(matches_pattern("[0-9]+", "123"));
        assert!(!matches_pattern("[0-9]+", "12a"));
        assert!(matches_pattern("^10\\..*", "10.0.19045"));
        assert!(!matches_pattern("^10\\..*", "11.0"));
    }
}
