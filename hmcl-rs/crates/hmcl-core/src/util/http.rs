//! The way the launcher asks the servers of the game for files.
//!
//! The launcher downloads the files of the game, the metadata of the versions,
//! the add-ons and the skins from servers of other people, so it has to behave
//! the way those servers expect: a user agent they do not refuse, a proxy the
//! player wrote, and redirects that are followed by hand because some servers
//! answer with a `Location` that is not encoded.
//!
//! The module holds the parts of that behaviour that do not need a connection:
//! the headers the launcher always sends, the way the settings of the proxy are
//! written as a proxy the program of the launcher can use, and the rules the
//! redirects follow. The program the launcher runs in reads them and opens the
//! connections itself, so the launcher of Java and this one ask the same
//! servers in the same way.

use std::fmt;
use std::time::Duration;

use crate::metadata;
use crate::setting::launcher::{ProxyAddress, ProxyCredentials};

/// The value of the `User-Agent` header the launcher sends.
///
/// The launcher of Java reads the property `http.agent` and falls back to
/// `HMCL`; this one names the launcher and its version instead, because the
/// servers of the game and the mirrors refuse a client they cannot name.
pub const USER_AGENT: &str = concat!("HMCL", "/", env!("CARGO_PKG_VERSION"));

/// The moment the launcher waits for a connection to be made.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// The moment the launcher waits for the body of an answer.
pub const READ_TIMEOUT: Duration = Duration::from_secs(10);

/// The number of redirects the launcher follows before it gives up.
///
/// The launcher of Java refuses a chain of more than twenty redirects, which is
/// what the servers of the game need when a file moved to a mirror.
pub const MAX_REDIRECTS: usize = 20;

/// The way the program of the launcher reaches the network.
///
/// The settings of the launcher name the mode of the proxy and the host of it;
/// this is the proxy written the way the programs that open connections expect
/// it, so the program does not have to read the settings itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxySetting {
    /// The proxy of the system, which the launcher does not know.
    System,
    /// No proxy at all.
    Direct,
    /// A proxy of a protocol at a host and a port, with the credentials the
    /// player wrote when the proxy asks for them.
    Custom {
        /// The protocol of the proxy, written the way the programs expect it,
        /// which is `http`, `socks4`, `socks4a` or `socks5`.
        scheme: &'static str,
        /// The host of the proxy.
        host: String,
        /// The port of the proxy.
        port: u16,
        /// The user name of the proxy, when the player wrote one.
        user: Option<String>,
        /// The password of the proxy, when the player wrote one.
        password: Option<String>,
    },
}

impl ProxySetting {
    /// Writes the proxy the way the programs that open connections read it.
    ///
    /// The address is `<scheme>://[<user>:<password>@]<host>:<port>`, which is
    /// the form the launcher of Java writes its proxy options in and the form
    /// the programs of the network parse. A proxy of the system and a direct
    /// connection are not written, because the program already knows them.
    pub fn to_url(&self) -> Option<String> {
        match self {
            Self::System | Self::Direct => None,
            Self::Custom {
                scheme,
                host,
                port,
                user,
                password,
            } => {
                let credentials = match (user, password) {
                    (Some(user), Some(password)) => format!("{user}:{password}@"),
                    _ => String::new(),
                };
                Some(format!("{scheme}://{credentials}{host}:{port}"))
            }
        }
    }
}

/// Returns the way the program of the launcher reaches the network.
///
/// A mode of the launcher's own with a host the player did not write is the
/// same as no proxy at all, which is what [`ProxyAddress`] already decided when
/// the settings were read.
pub fn proxy_setting(
    address: &ProxyAddress,
    credentials: Option<&ProxyCredentials>,
) -> ProxySetting {
    match address {
        ProxyAddress::System => ProxySetting::System,
        ProxyAddress::Direct => ProxySetting::Direct,
        ProxyAddress::Custom {
            proxy_type,
            host,
            port,
        } => {
            // The launcher of Java writes `socks` for both SOCKS4 and SOCKS5,
            // because the programs of the network read it as SOCKS5.
            let scheme = match proxy_type {
                crate::setting::launcher::ProxyType::Http => "http",
                crate::setting::launcher::ProxyType::Socks => "socks5",
                crate::setting::launcher::ProxyType::System
                | crate::setting::launcher::ProxyType::Direct => "http",
            };

            ProxySetting::Custom {
                scheme,
                host: host.clone(),
                port: *port,
                user: credentials.map(|credentials| credentials.user.clone()),
                password: credentials.map(|credentials| credentials.password.clone()),
            }
        }
    }
}

/// The headers the launcher sends with every request.
///
/// The launcher of Java sends the language of the player and its user agent
/// with every connection it makes, so the servers of the game answer in the
/// language of the player and do not refuse the client.
pub fn default_headers(language_tag: &str) -> Vec<(&'static str, String)> {
    vec![
        ("Accept-Language", language_tag.to_string()),
        ("User-Agent", USER_AGENT.to_string()),
    ]
}

/// Returns the address a redirect points to.
///
/// The `Location` of an answer is often a relative address, and some servers
/// write it without encoding the characters that are not allowed in an address,
/// so the launcher resolves it against the address it asked and encodes it the
/// way the servers expect. `None` is returned when the address is not one the
/// launcher may ask, which is the case for an empty location and for a scheme
/// that is not HTTP.
pub fn resolve_redirect(base: &str, location: &str) -> Option<String> {
    let location = location.trim();
    if location.is_empty() {
        return None;
    }

    let base = base_url(base)?;
    let target = if location.contains("://") {
        location.to_string()
    } else if let Some(rest) = location.strip_prefix("//") {
        format!("{}://{rest}", base.scheme)
    } else if location.starts_with('/') {
        format!("{}://{}{location}", base.scheme, base.authority)
    } else {
        // A relative address is resolved against the directory of the address
        // that was asked, which is the base without its last segment.
        let directory = match base.path.rfind('/') {
            Some(index) => &base.path[..=index],
            None => "/",
        };
        format!("{}://{}{directory}{location}", base.scheme, base.authority)
    };

    let resolved = base_url(&target)?;
    if resolved.scheme != "http" && resolved.scheme != "https" {
        return None;
    }

    // The launcher of Java resolves an address with `WebURL`, which removes the
    // segments `.` and `..` the way a browser does, so a server may name its
    // API with an address relative to the directory it answers from.
    let address = format!(
        "{}://{}{}",
        resolved.scheme,
        resolved.authority,
        remove_dot_segments(&resolved.path)
    );

    Some(encode_location(&address))
}

/// Returns a path with the segments `.` and `..` resolved.
///
/// A path without such a segment is returned as it is, including the repeated
/// slashes a server may write; a `..` that names no segment above the root is
/// dropped, because an address cannot go above its host.
fn remove_dot_segments(path: &str) -> String {
    let (path, suffix) = match path.find(['?', '#']) {
        Some(index) => (&path[..index], &path[index..]),
        None => (path, ""),
    };

    if !path
        .split('/')
        .any(|segment| segment == "." || segment == "..")
    {
        return format!("{path}{suffix}");
    }

    let absolute = path.starts_with('/');
    let mut segments: Vec<&str> = Vec::new();
    let mut trailing_slash = false;

    for segment in path.split('/') {
        match segment {
            "" | "." => trailing_slash = true,
            ".." => {
                segments.pop();
                trailing_slash = true;
            }
            other => {
                segments.push(other);
                trailing_slash = false;
            }
        }
    }

    let mut resolved = String::new();
    if absolute {
        resolved.push('/');
    }
    resolved.push_str(&segments.join("/"));
    if trailing_slash && !resolved.ends_with('/') {
        resolved.push('/');
    }
    resolved.push_str(suffix);
    resolved
}

/// Whether the code of an answer tells the launcher to ask another address.
///
/// The launcher of Java follows the codes from 300 to 308, but not 304, which
/// tells the launcher the file did not change, and not 306, which no server
/// uses.
pub fn is_redirect_status(status: u16) -> bool {
    (300..=308).contains(&status) && status != 304 && status != 306
}

/// The parts of an address the launcher resolves redirects with.
#[derive(Debug, Clone, PartialEq, Eq)]
struct BaseUrl {
    /// The protocol of the address, which is `http` or `https`.
    scheme: String,
    /// The host and the port of the address, as the server writes them.
    authority: String,
    /// The path of the address, which begins with a slash.
    path: String,
}

/// Writes the address the way the launcher wrote it.
impl fmt::Display for BaseUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}://{}{}",
            self.scheme, self.authority, self.path
        )
    }
}

/// Reads an address into the parts the launcher resolves redirects with.
///
/// An address the launcher cannot read, such as one without a host, is refused.
fn base_url(url: &str) -> Option<BaseUrl> {
    let (scheme, rest) = url.split_once("://")?;
    if scheme.is_empty() || rest.is_empty() {
        return None;
    }

    // The authority ends at the first slash, the question mark or the hash, so
    // a host written with a port and a path that begins with a slash are told
    // apart.
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..end];
    let path = &rest[end..];

    if authority.is_empty() {
        return None;
    }

    Some(BaseUrl {
        scheme: scheme.to_ascii_lowercase(),
        authority: authority.to_string(),
        // A path that holds no slash is a path of the root, so the directory
        // of the address is the root as well.
        path: if path.is_empty() {
            "/".to_string()
        } else {
            path.to_string()
        },
    })
}

/// Encodes the characters of an address that are not allowed in one.
///
/// Some servers answer with a `Location` that holds a space or a character of
/// another language, which the programs of the network refuse. The launcher of
/// Java encodes those characters the way curl does: the characters of the query
/// are written as they are, the space of a query becomes a plus, and everything
/// else becomes the percent-encoded form of the character.
pub fn encode_location(location: &str) -> String {
    let mut encoded = String::with_capacity(location.len() + 8);
    let mut in_query = false;

    for character in location.chars() {
        match character {
            '?' => {
                in_query = true;
                encoded.push('?');
            }
            ' ' if in_query => encoded.push('+'),
            character if needs_encoding(character, in_query) => {
                let mut buffer = [0u8; 4];
                for byte in character.encode_utf8(&mut buffer).as_bytes() {
                    encoded.push('%');
                    encoded.push_str(&format!("{byte:02X}"));
                }
            }
            character => encoded.push(character),
        }
    }

    encoded
}

/// Whether a character of an address has to be encoded.
///
/// The characters that carry a meaning in an address, such as the separators of
/// the path and the query, are left alone, as are the letters and the digits;
/// everything else, and every character that is not ASCII, is encoded.
fn needs_encoding(character: char, in_query: bool) -> bool {
    if character.is_ascii_alphanumeric() {
        return false;
    }

    if character.is_ascii() {
        // The characters curl leaves alone are the ones that carry a meaning in
        // an address, such as the separators of the path and the query.
        const ALLOWED: &str = "-_.~:/?#[]@!$&'()*+,;=";
        !ALLOWED.contains(character)
    } else {
        // The brackets of a query are not encoded, because some servers use
        // them to name a file.
        let _ = in_query;
        true
    }
}

/// Returns the name of the launcher and its version, for the log and the
/// headers.
pub fn user_agent() -> &'static str {
    USER_AGENT
}

/// Returns the version of the launcher, for the log and the headers.
pub fn launcher_version() -> &'static str {
    metadata::VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_user_agent_names_the_launcher() {
        assert!(USER_AGENT.starts_with("HMCL/"), "{USER_AGENT}");
        assert!(USER_AGENT.contains(launcher_version()));
    }

    #[test]
    fn a_direct_proxy_is_not_written() {
        assert_eq!(ProxySetting::Direct.to_url(), None);
        assert_eq!(ProxySetting::System.to_url(), None);
    }

    #[test]
    fn an_http_proxy_is_written_with_its_port() {
        let setting = ProxySetting::Custom {
            scheme: "http",
            host: "127.0.0.1".to_string(),
            port: 8080,
            user: None,
            password: None,
        };
        assert_eq!(setting.to_url(), Some("http://127.0.0.1:8080".to_string()));
    }

    #[test]
    fn a_socks_proxy_is_written_with_its_credentials() {
        let setting = ProxySetting::Custom {
            scheme: "socks5",
            host: "proxy.example.com".to_string(),
            port: 1080,
            user: Some("alice".to_string()),
            password: Some("s3cret".to_string()),
        };
        assert_eq!(
            setting.to_url(),
            Some("socks5://alice:s3cret@proxy.example.com:1080".to_string())
        );
    }

    #[test]
    fn the_settings_of_the_proxy_are_read() {
        use crate::setting::launcher::ProxyType;

        let address = ProxyAddress::Custom {
            proxy_type: ProxyType::Http,
            host: "127.0.0.1".to_string(),
            port: 7890,
        };
        let credentials = ProxyCredentials {
            user: "alice".to_string(),
            password: "s3cret".to_string(),
        };

        let setting = proxy_setting(&address, Some(&credentials));
        assert_eq!(
            setting,
            ProxySetting::Custom {
                scheme: "http",
                host: "127.0.0.1".to_string(),
                port: 7890,
                user: Some("alice".to_string()),
                password: Some("s3cret".to_string()),
            }
        );
        assert_eq!(
            setting.to_url(),
            Some("http://alice:s3cret@127.0.0.1:7890".to_string())
        );
    }

    #[test]
    fn a_socks_proxy_of_the_settings_is_written_as_socks5() {
        use crate::setting::launcher::ProxyType;

        let address = ProxyAddress::Custom {
            proxy_type: ProxyType::Socks,
            host: "127.0.0.1".to_string(),
            port: 1080,
        };

        assert_eq!(
            proxy_setting(&address, None).to_url(),
            Some("socks5://127.0.0.1:1080".to_string())
        );
    }

    #[test]
    fn an_absolute_location_is_kept() {
        assert_eq!(
            resolve_redirect("https://example.com/a/b", "https://other.com/c"),
            Some("https://other.com/c".to_string())
        );
    }

    #[test]
    fn a_relative_location_is_resolved_against_the_directory() {
        assert_eq!(
            resolve_redirect("https://example.com/a/b/c", "d"),
            Some("https://example.com/a/b/d".to_string())
        );
    }

    #[test]
    fn a_root_location_keeps_the_host() {
        assert_eq!(
            resolve_redirect("https://example.com/a/b/c", "/d/e"),
            Some("https://example.com/d/e".to_string())
        );
    }

    #[test]
    fn a_location_without_a_scheme_keeps_the_scheme() {
        assert_eq!(
            resolve_redirect("https://example.com/a/b", "//other.com/c"),
            Some("https://other.com/c".to_string())
        );
    }

    #[test]
    fn an_empty_location_is_refused() {
        assert_eq!(resolve_redirect("https://example.com/a", ""), None);
        assert_eq!(resolve_redirect("https://example.com/a", "   "), None);
    }

    #[test]
    fn a_location_of_another_scheme_is_refused() {
        assert_eq!(
            resolve_redirect("https://example.com/a", "ftp://example.com/b"),
            None
        );
        assert_eq!(
            resolve_redirect("https://example.com/a", "file:///etc/passwd"),
            None
        );
    }

    #[test]
    fn an_address_without_a_host_is_refused() {
        assert_eq!(resolve_redirect("https://", "/a"), None);
        assert_eq!(resolve_redirect("not an address", "/a"), None);
    }

    #[test]
    fn a_location_with_a_space_is_encoded() {
        assert_eq!(
            resolve_redirect("https://example.com/a", "/my file.jar"),
            Some("https://example.com/my%20file.jar".to_string())
        );
    }

    #[test]
    fn a_space_of_a_query_becomes_a_plus() {
        assert_eq!(
            resolve_redirect("https://example.com/a?b=c", "/search?q=a b"),
            Some("https://example.com/search?q=a+b".to_string())
        );
    }

    #[test]
    fn a_character_of_another_language_is_encoded() {
        assert_eq!(
            resolve_redirect("https://example.com/a", "/模组.jar"),
            Some("https://example.com/%E6%A8%A1%E7%BB%84.jar".to_string())
        );
    }

    #[test]
    fn the_dot_segments_of_a_location_are_resolved() {
        // A server may name a page relative to the directory it answers from,
        // which a browser resolves before it asks.
        assert_eq!(
            resolve_redirect("https://example.com/page/", "../api/yggdrasil"),
            Some("https://example.com/api/yggdrasil".to_string())
        );
        assert_eq!(
            resolve_redirect("https://example.com/a/b/", "./c"),
            Some("https://example.com/a/b/c".to_string())
        );
        // A `..` above the root cannot go further up than the host.
        assert_eq!(
            resolve_redirect("https://example.com/a/", "../../b"),
            Some("https://example.com/b".to_string())
        );
        // The segments of a query are not segments of the path.
        assert_eq!(
            resolve_redirect("https://example.com/a/", "../b?c=../d"),
            Some("https://example.com/b?c=../d".to_string())
        );
        // A path without such a segment is kept as it is, including the
        // repeated slashes a server may write.
        assert_eq!(
            resolve_redirect("https://example.com/a//b", "c"),
            Some("https://example.com/a//c".to_string())
        );
    }

    #[test]
    fn the_codes_of_a_redirect_are_read() {
        assert!(is_redirect_status(301));
        assert!(is_redirect_status(302));
        assert!(is_redirect_status(303));
        assert!(is_redirect_status(307));
        assert!(is_redirect_status(308));

        assert!(!is_redirect_status(200));
        assert!(!is_redirect_status(304));
        assert!(!is_redirect_status(306));
        assert!(!is_redirect_status(404));
    }

    #[test]
    fn the_default_headers_name_the_language_and_the_launcher() {
        let headers = default_headers("zh-CN");
        assert_eq!(
            headers,
            vec![
                ("Accept-Language", "zh-CN".to_string()),
                ("User-Agent", USER_AGENT.to_string()),
            ]
        );
    }
}
