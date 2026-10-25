//! The arguments of the command a version of the game is started with.

use std::collections::BTreeMap;

use crate::download::game::{
    rules_allow, Argument, ArgumentDetails, ArgumentValue, CompatibilityRule, OsRule, RuleAction,
    RuleContext,
};

use super::variables::LaunchVariables;

/// Returns the texts the arguments of a version are started with.
///
/// The arguments about other systems and about other features are skipped; the
/// variables of the arguments are replaced with the values of the instance.
pub fn resolve_arguments(
    arguments: &[Argument],
    variables: &LaunchVariables,
    context: &RuleContext<'_>,
) -> Vec<String> {
    let mut texts = Vec::new();

    for argument in arguments {
        if !rules_allow(argument.rules(), context) {
            continue;
        }

        for value in argument.values() {
            texts.push(variables.substitute(value));
        }
    }

    texts
}

/// Returns the arguments of a version that writes them in one text.
///
/// The arguments are separated by white space; the variables of the arguments
/// are replaced with the values of the instance.
pub fn split_arguments(text: &str, variables: &LaunchVariables) -> Vec<String> {
    tokenize(text)
        .into_iter()
        .map(|argument| variables.substitute(&argument))
        .collect()
}

/// Splits the arguments of a version that writes them in one text.
///
/// The arguments are separated by white space; an argument that holds white
/// space is written between quotes.
pub fn tokenize(text: &str) -> Vec<String> {
    let mut arguments = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut has_text = false;

    for character in text.chars() {
        match quote {
            Some(expected) => {
                if character == expected {
                    quote = None;
                } else {
                    current.push(character);
                }
            }
            None => match character {
                '"' | '\'' => {
                    quote = Some(character);
                    has_text = true;
                }
                character if character.is_whitespace() => {
                    if has_text {
                        arguments.push(std::mem::take(&mut current));
                        has_text = false;
                    }
                }
                character => {
                    current.push(character);
                    has_text = true;
                }
            },
        }
    }

    if has_text {
        arguments.push(current);
    }

    arguments
}

/// Returns the arguments the versions without their own arguments of the
/// virtual machine are started with.
///
/// The versions before the arguments were written as a list need the path of
/// the native libraries and the classpath; the versions of Windows need the
/// names of the system the game reports.
pub fn default_jvm_arguments() -> Vec<Argument> {
    vec![
        ruled(
            Some(vec![allow_os("windows")]),
            &[
                "-XX:HeapDumpPath=MojangTricksIntelDriversForPerformance_javaw.exe_minecraft.exe.heapdump",
            ],
        ),
        ruled(
            Some(vec![allow_windows_10()]),
            &["-Dos.name=Windows 10", "-Dos.version=10.0"],
        ),
        text("-Djava.library.path=${natives_directory}"),
        text("-Dminecraft.launcher.brand=${launcher_name}"),
        text("-Dminecraft.launcher.version=${launcher_version}"),
        text("-cp"),
        text("${classpath}"),
    ]
}

/// Returns the arguments the versions that write the arguments of the game in
/// one text are started with.
///
/// The versions before the arguments were written as a list do not read the
/// size of the window themselves, so the launcher passes it when the player
/// set one.
pub fn default_game_arguments() -> Vec<Argument> {
    vec![ruled(
        Some(vec![allow_feature("has_custom_resolution", true)]),
        &[
            "--width",
            "${resolution_width}",
            "--height",
            "${resolution_height}",
        ],
    )]
}

/// Returns an argument that is always used.
fn text(value: &str) -> Argument {
    Argument::Text(value.to_string())
}

/// Returns an argument that is used when the rules allow it.
fn ruled(rules: Option<Vec<CompatibilityRule>>, values: &[&str]) -> Argument {
    Argument::Detailed(ArgumentDetails {
        rules,
        value: Some(ArgumentValue::Texts(
            values.iter().map(|value| (*value).to_string()).collect(),
        )),
    })
}

/// Returns a rule about a system.
fn allow_os(name: &str) -> CompatibilityRule {
    CompatibilityRule {
        action: RuleAction::Allow,
        os: Some(OsRule {
            name: Some(name.to_string()),
            version: None,
            arch: None,
        }),
        features: None,
    }
}

/// Returns a rule about the version 10 of Windows.
fn allow_windows_10() -> CompatibilityRule {
    CompatibilityRule {
        action: RuleAction::Allow,
        os: Some(OsRule {
            name: Some("windows".to_string()),
            version: Some("^10\\.".to_string()),
            arch: None,
        }),
        features: None,
    }
}

/// Returns a rule about a feature of the game.
fn allow_feature(name: &str, value: bool) -> CompatibilityRule {
    CompatibilityRule {
        action: RuleAction::Allow,
        os: None,
        features: Some(BTreeMap::from([(name.to_string(), value)])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::download::game::Arguments;
    use crate::game::launch::classpath::join_classpath;
    use std::path::PathBuf;

    fn variables() -> LaunchVariables {
        let mut variables = LaunchVariables::default();
        variables.set("auth_player_name", "Steve");
        variables.set("natives_directory", "natives");
        variables.set("launcher_name", "HMCL");
        variables.set("launcher_version", "1.0");
        variables.set("classpath", join_classpath(&[PathBuf::from("a.jar")]));
        variables.set("resolution_width", "1280");
        variables.set("resolution_height", "720");
        variables
    }

    fn context<'a>(os: &'a str, features: &'a BTreeMap<String, bool>) -> RuleContext<'a> {
        RuleContext::new(os, "10.0", "x86_64", features)
    }

    #[test]
    fn the_arguments_about_other_systems_are_skipped() {
        let arguments = vec![
            Argument::Text("--username".to_string()),
            ruled(Some(vec![allow_os("osx")]), &["-XstartOnFirstThread"]),
            Argument::Text("${auth_player_name}".to_string()),
        ];
        let features = BTreeMap::new();
        let values = variables();

        assert_eq!(
            resolve_arguments(&arguments, &values, &context("windows", &features)),
            vec!["--username", "Steve"]
        );
        assert_eq!(
            resolve_arguments(&arguments, &values, &context("osx", &features)),
            vec!["--username", "-XstartOnFirstThread", "Steve"]
        );
    }

    #[test]
    fn the_arguments_about_a_feature_are_read() {
        let arguments = default_game_arguments();
        let values = variables();

        let mut features = BTreeMap::new();
        features.insert("has_custom_resolution".to_string(), true);
        assert_eq!(
            resolve_arguments(&arguments, &values, &context("windows", &features)),
            vec!["--width", "1280", "--height", "720"]
        );

        features.insert("has_custom_resolution".to_string(), false);
        assert!(resolve_arguments(&arguments, &values, &context("windows", &features)).is_empty());
    }

    #[test]
    fn the_arguments_of_the_old_versions_are_split() {
        assert_eq!(
            tokenize("--username Steve --demo"),
            vec!["--username", "Steve", "--demo"]
        );
        assert_eq!(
            tokenize("--title \"Hello World\" --empty \"\""),
            vec!["--title", "Hello World", "--empty", ""]
        );
        assert_eq!(
            split_arguments("--username ${auth_player_name}", &variables()),
            vec!["--username", "Steve"]
        );
    }

    #[test]
    fn the_default_arguments_pass_the_classpath() {
        let arguments = default_jvm_arguments();
        let features = BTreeMap::new();
        let resolved = resolve_arguments(&arguments, &variables(), &context("windows", &features));

        assert!(resolved.iter().any(|argument| argument == "-cp"));
        assert!(resolved.iter().any(|argument| argument == "a.jar"));
        assert!(resolved
            .iter()
            .any(|argument| argument == "-Djava.library.path=natives"));
    }

    #[test]
    fn the_default_arguments_of_the_other_systems_are_skipped() {
        let arguments = default_jvm_arguments();
        let features = BTreeMap::new();
        let resolved = resolve_arguments(&arguments, &variables(), &context("linux", &features));

        assert!(!resolved
            .iter()
            .any(|argument| argument.contains("HeapDumpPath")));
        assert!(!resolved
            .iter()
            .any(|argument| argument == "-Dos.name=Windows 10"));
    }

    #[test]
    fn the_arguments_of_a_modern_version_are_read() {
        let arguments: Arguments = serde_json::from_str(
            r#"{
                "game": [
                    "--username", "${auth_player_name}",
                    { "rules": [ { "action": "allow", "features": { "is_demo_user": true } } ], "value": ["--demo"] },
                    { "rules": [ { "action": "allow", "os": { "name": "windows" } } ], "value": "--windows" }
                ]
            }"#,
        )
        .expect("the arguments should be read");
        let features = BTreeMap::new();

        let resolved = resolve_arguments(
            arguments.game.as_deref().unwrap_or_default(),
            &variables(),
            &context("windows", &features),
        );

        assert_eq!(resolved, vec!["--username", "Steve", "--windows"]);
    }
}
