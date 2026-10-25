//! The address of a server of the game.
//!
//! The game joins a server with its address, which is written as the name of
//! the host and, when it is not the usual one, the port the game reads. The
//! launcher reads the address the player wrote to tell the game which server
//! to join, and to tell the player when the address is not one the game can
//! read.

use std::fmt;

/// The number of the port when the address does not name one.
///
/// The game reads the usual port when the address does not name one, so the
/// launcher keeps the address as it was written.
pub const UNKNOWN_PORT: i32 = -1;

/// The largest number a port can be.
const MAX_PORT: i32 = 0xFFFF;

/// The error of reading an address of a server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerAddressError {
    /// The address that could not be read.
    address: String,
}

impl ServerAddressError {
    /// Returns the address that could not be read.
    pub fn address(&self) -> &str {
        &self.address
    }
}

impl fmt::Display for ServerAddressError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "Invalid server address: {}", self.address)
    }
}

impl std::error::Error for ServerAddressError {}

/// The address of a server of the game.
///
/// The address is the name of the host and the port the game reads. A host of
/// the kind of IPv6 is written between brackets, because the numbers of such
/// a host hold colons of their own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerAddress {
    /// The name of the host, or the numbers of the host of the kind of IPv6.
    host: String,
    /// The port the game reads, or [`UNKNOWN_PORT`] when the address does not
    /// name one.
    port: i32,
}

impl ServerAddress {
    /// Returns an address of a host, without a port.
    pub fn of_host(host: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port: UNKNOWN_PORT,
        }
    }

    /// Returns an address of a host and a port.
    pub fn of_host_and_port(host: impl Into<String>, port: i32) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }

    /// Returns the name of the host, or the numbers of the host of the kind
    /// of IPv6.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Returns the port the game reads, or [`UNKNOWN_PORT`] when the address
    /// does not name one.
    pub fn port(&self) -> i32 {
        self.port
    }

    /// Returns whether the address names a port.
    pub fn has_port(&self) -> bool {
        self.port != UNKNOWN_PORT
    }

    /// Reads an address the player wrote.
    ///
    /// The port is the numbers that follow the last colon of the address, and
    /// a host of the kind of IPv6 is written between brackets so its own
    /// colons are not read as the port.
    ///
    /// # Errors
    ///
    /// Returns an error when the address names a port that is not a number
    /// between zero and 65535, or when the brackets of a host of the kind of
    /// IPv6 are not closed before the port.
    pub fn parse(address: &str) -> Result<Self, ServerAddressError> {
        let invalid = || ServerAddressError {
            address: address.to_string(),
        };

        if !address.starts_with('[') {
            // The address of a host that is not of the kind of IPv6 holds at
            // most one colon, which begins the port.
            match address.find(':') {
                Some(colon) => {
                    if colon == address.len() - 1 {
                        return Err(invalid());
                    }

                    let port = parse_port(&address[colon + 1..]).ok_or_else(invalid)?;
                    Ok(Self::of_host_and_port(&address[..colon], port))
                }
                None => Ok(Self::of_host(address)),
            }
        } else {
            // The host of the kind of IPv6 is written between brackets, and
            // the port follows the bracket that closes it.
            let Some(colon) = address.find(':') else {
                return Err(invalid());
            };
            let Some(close) = address.rfind(']') else {
                return Err(invalid());
            };
            if close < colon {
                return Err(invalid());
            }

            let host = &address[1..close];
            if close == address.len() - 1 {
                return Ok(Self::of_host(host));
            }

            // The port is written as `]:port`, so a character must follow the
            // bracket that closes the host.
            if address.len() < close + 3 || address.as_bytes()[close + 1] != b':' {
                return Err(invalid());
            }

            let port = parse_port(&address[close + 2..]).ok_or_else(invalid)?;
            Ok(Self::of_host_and_port(host, port))
        }
    }

    /// Writes the address the way the game reads it.
    ///
    /// A host of the kind of IPv6 is written between brackets, because its own
    /// colons would be read as the port otherwise.
    pub fn to_address_string(&self) -> String {
        if self.host.contains(':') {
            if self.has_port() {
                format!("[{}]:{}", self.host, self.port)
            } else {
                format!("[{}]", self.host)
            }
        } else if self.has_port() {
            format!("{}:{}", self.host, self.port)
        } else {
            self.host.clone()
        }
    }
}

impl fmt::Display for ServerAddress {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "ServerAddress[host='{}', port={}]",
            self.host, self.port
        )
    }
}

impl From<&str> for ServerAddress {
    /// Reads an address, and keeps it as the host when it cannot be read.
    fn from(address: &str) -> Self {
        Self::parse(address).unwrap_or_else(|_| Self::of_host(address))
    }
}

/// Reads the numbers of a port.
///
/// Returns `None` when the text is not a number between zero and 65535.
fn parse_port(text: &str) -> Option<i32> {
    let port: i32 = text.parse().ok()?;
    if (0..=MAX_PORT).contains(&port) {
        Some(port)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_address_without_a_port_is_read() {
        let address = ServerAddress::parse("example.org").expect("the address should be read");
        assert_eq!(address.host(), "example.org");
        assert_eq!(address.port(), UNKNOWN_PORT);
        assert!(!address.has_port());
    }

    #[test]
    fn an_address_with_a_port_is_read() {
        let address =
            ServerAddress::parse("example.org:25565").expect("the address should be read");
        assert_eq!(address.host(), "example.org");
        assert_eq!(address.port(), 25565);
        assert!(address.has_port());
    }

    #[test]
    fn an_address_of_a_host_of_the_kind_of_ipv6_is_read() {
        let address =
            ServerAddress::parse("[2001:db8::1]:25565").expect("the address should be read");
        assert_eq!(address.host(), "2001:db8::1");
        assert_eq!(address.port(), 25565);

        // A host of the kind of IPv6 without a port keeps its own colons.
        let address = ServerAddress::parse("[2001:db8::1]").expect("the address should be read");
        assert_eq!(address.host(), "2001:db8::1");
        assert!(!address.has_port());
    }

    #[test]
    fn an_address_that_names_no_port_after_the_colon_is_refused() {
        assert!(ServerAddress::parse("example.org:").is_err());
        assert!(ServerAddress::parse("example.org:not-a-port").is_err());
        assert!(ServerAddress::parse("example.org:-1").is_err());
        assert!(ServerAddress::parse("example.org:65536").is_err());
    }

    #[test]
    fn an_address_of_a_host_of_the_kind_of_ipv6_without_a_bracket_is_refused() {
        // The brackets of the host are not closed before the port.
        assert!(ServerAddress::parse("[2001:db8::1:25565").is_err());
        // The character after the bracket that closes the host is not a colon.
        assert!(ServerAddress::parse("[2001:db8::1]25565").is_err());
        // The port is not written at all.
        assert!(ServerAddress::parse("[2001:db8::1]:").is_err());
        assert!(ServerAddress::parse("[2001:db8::1]:not-a-port").is_err());
    }

    #[test]
    fn an_address_is_written_the_way_the_game_reads_it() {
        assert_eq!(
            ServerAddress::of_host("example.org").to_address_string(),
            "example.org"
        );
        assert_eq!(
            ServerAddress::of_host_and_port("example.org", 25565).to_address_string(),
            "example.org:25565"
        );
        // A host of the kind of IPv6 is written between brackets, because its
        // own colons would be read as the port otherwise.
        assert_eq!(
            ServerAddress::of_host("2001:db8::1").to_address_string(),
            "[2001:db8::1]"
        );
        assert_eq!(
            ServerAddress::of_host_and_port("2001:db8::1", 25565).to_address_string(),
            "[2001:db8::1]:25565"
        );
    }

    #[test]
    fn an_address_that_cannot_be_read_is_kept_as_the_host() {
        let address = ServerAddress::from("not a port:at all");
        assert_eq!(address.host(), "not a port:at all");
        assert!(!address.has_port());
    }

    #[test]
    fn the_address_is_written_with_its_host_and_its_port() {
        assert_eq!(
            ServerAddress::of_host_and_port("example.org", 25565).to_string(),
            "ServerAddress[host='example.org', port=25565]"
        );
    }
}
