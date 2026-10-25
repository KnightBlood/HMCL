//! The authentication servers of the authlib-injector the player uses.
//!
//! The player may name a server of the authlib-injector the launcher signs
//! the game with; the launcher keeps the list of the servers the player uses
//! in `config/authlib-injector-servers.json` and the metadata it read from
//! every server in `cache/authlib-injector-server-metadata.json`, which is
//! the way the launcher of Java keeps them.
//!
//! The metadata of a server is read from the server over the network; the
//! files here keep the list and the metadata the launcher already has, so the
//! launcher starts even when the network is not there.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::schema::{Schema, SchemaVersion};
use crate::error::Error;
use crate::util::datetime::unix_millis;
use crate::util::http_request::{send, HttpClient, HttpRequest};
use crate::util::logging;

/// The address of the LittleSkin API the launcher puts into a new list.
pub const LITTLE_SKIN_URL: &str = "https://littleskin.cn/api/yggdrasil/";

/// The name of the file that holds the list of the servers.
pub const AUTHLIB_INJECTOR_SERVERS_FILE: &str = "authlib-injector-servers.json";

/// The name of the file that holds the metadata of the servers.
pub const AUTHLIB_INJECTOR_METADATA_FILE: &str = "authlib-injector-server-metadata.json";

/// The schema of the file that holds the list of the servers.
pub const AUTHLIB_INJECTOR_SERVERS_SCHEMA: &str =
    "https://schemas.glavo.site/hmcl/authlib-injector-servers/1.0.0";

/// The schema of the file that holds the metadata of the servers.
pub const AUTHLIB_INJECTOR_METADATA_SCHEMA: &str =
    "https://schemas.glavo.site/hmcl/authlib-injector-server-metadata/1.0.0";

/// Returns the schema of the file that holds the list of the servers, which
/// the deserializer of the list reads when the file names none.
fn default_servers_schema() -> String {
    AUTHLIB_INJECTOR_SERVERS_SCHEMA.to_string()
}

/// Returns the schema of the file that holds the metadata of the servers,
/// which the deserializer of the cache reads when the file names none.
fn default_metadata_schema() -> String {
    AUTHLIB_INJECTOR_METADATA_SCHEMA.to_string()
}

/// Returns the address the pages of the launcher show for a server URL.
///
/// The launcher of Java shows the host of an HTTPS server, with its port when
/// the port is not the one of HTTPS and without the path of the API of
/// Yggdrasil; a URL it cannot read is shown as it is written.
pub fn display_host_url(url: &str) -> String {
    let Some(scheme_end) = url.find("://") else {
        return url.to_string();
    };

    let scheme = &url[..scheme_end];
    if scheme != "https" {
        return url.to_string();
    }

    let rest = &url[scheme_end + 3..];
    let (authority, path) = match rest.split_once('/') {
        Some((authority, path)) => (authority, Some(path)),
        None => (rest, None),
    };

    let host = match authority.rsplit_once(':') {
        Some((host, port))
            if port.is_empty() || port.chars().all(|digit| digit.is_ascii_digit()) =>
        {
            host
        }
        _ => authority,
    };
    let port = authority
        .rsplit_once(':')
        .and_then(|(_, port)| port.parse::<u16>().ok());

    let mut shown = host.to_string();
    if port.is_none_or(|port| port != 443) {
        if let Some(port) = port {
            shown.push_str(&format!(":{port}"));
        }
    }

    if path == Some("api/yggdrasil/") {
        return shown;
    }
    if let Some(path) = path {
        shown.push('/');
        shown.push_str(path);
    }
    shown.shrink_to_fit();
    shown
}

/// Adds the scheme of HTTPS to a URL that names none, and puts a slash at its
/// end.
///
/// The player writes the address of a server as a domain, which is HTTPS; the
/// launcher of Java turns such an address into an HTTPS URL, and keeps the
/// slash at the end, because the endpoint of Yggdrasil is under it.
pub fn normalize_url(url: &str) -> String {
    let with_scheme = add_https_if_missing(url.trim());

    if with_scheme.ends_with('/') {
        with_scheme
    } else {
        format!("{with_scheme}/")
    }
}

/// Adds the scheme of HTTPS to a URL that names no scheme.
///
/// A URL that names a scheme, such as `http://example.com`, is kept as it is; a
/// URL that starts with two slashes takes the scheme of HTTPS, and a URL that
/// names no scheme at all is read as the host of an HTTPS URL. This is the
/// reading of `NetworkUtils.addHttpsIfMissing`, which the launcher of Java
/// applies before it asks a server the player named.
fn add_https_if_missing(url: &str) -> String {
    if has_scheme(url) {
        url.to_string()
    } else if url.starts_with("//") {
        format!("https:{url}")
    } else {
        format!("https://{url}")
    }
}

/// Returns whether a URL starts with a scheme, such as `https://`.
///
/// A scheme begins with a letter and continues with letters, digits and the
/// three characters `+`, `.` and `-`, which is the form the launcher of Java
/// looks for.
fn has_scheme(url: &str) -> bool {
    let Some((scheme, _)) = url.split_once("://") else {
        return false;
    };

    let mut characters = scheme.chars();
    match characters.next() {
        Some(first) if first.is_ascii_alphabetic() => {}
        _ => return false,
    }

    characters
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '+' | '.' | '-'))
}

/// The header a server of the authlib-injector names the address of its API
/// in.
///
/// The player often names the page of a server rather than the endpoint of
/// Yggdrasil, so the server answers with the address the launcher has to ask
/// instead.
pub const API_LOCATION_HEADER: &str = "x-authlib-injector-api-location";

/// Returns whether two addresses name the same server, ignoring the slash at
/// the end.
///
/// The launcher of Java compares the addresses this way, because a server may
/// answer with the address of its API without the slash the launcher wrote.
fn url_equals_ignore_slash(first: &str, second: &str) -> bool {
    fn with_slash(url: &str) -> String {
        if url.ends_with('/') {
            url.to_string()
        } else {
            format!("{url}/")
        }
    }

    with_slash(first) == with_slash(second)
}

/// Returns the address of a server of the authlib-injector a dropped text
/// names.
///
/// The page of a server may hand the launcher a link of the form
/// `authlib-injector:yggdrasil-server:<encoded address>`, which the player
/// drops onto the window. The address is form-encoded, so the characters that
/// are not letters are read the way a browser wrote them; a text that is not
/// such a link names no server.
pub fn parse_dropped_url(text: &str) -> Option<String> {
    /// The scheme of a link that names a server of the authlib-injector.
    const SCHEME: &str = "authlib-injector";
    /// The part of the link that names a server of Yggdrasil.
    const PATH_YGGDRASIL_SERVER: &str = "yggdrasil-server";

    // Java splits the text on every colon and reads it only when it is made of
    // exactly three parts, so a text that holds a colon inside the address is
    // refused as well.
    let parts: Vec<&str> = text.split(':').collect();
    if parts.len() != 3 || parts[0] != SCHEME || parts[1] != PATH_YGGDRASIL_SERVER {
        return None;
    }

    Some(crate::util::http_request::decode_form_component(parts[2]))
}

/// Returns the text of a JSON value the way Java reads a primitive.
///
/// A string is its text, a number is written as it is, and a boolean is
/// written `true` or `false`; an array or an object is not a primitive and
/// returns nothing, which is what `JsonPrimitive.getAsString` refuses.
fn primitive_text(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(text) => Some(text.clone()),
        serde_json::Value::Number(number) => Some(number.to_string()),
        serde_json::Value::Bool(boolean) => Some(boolean.to_string()),
        serde_json::Value::Null | serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            None
        }
    }
}

/// Returns the boolean of a JSON value the way Java reads a primitive.
///
/// Only the text `true`, in any case, is true, which is what
/// `Boolean.parseBoolean` does; every other primitive, and every value that is
/// not a primitive, is false.
fn primitive_boolean(value: &serde_json::Value) -> Option<bool> {
    primitive_text(value).map(|text| text.eq_ignore_ascii_case("true"))
}

/// An authentication server of the authlib-injector.
///
/// The server is named by its address, which is the one the player wrote,
/// normalized by [`normalize_url`]. The metadata the server tells is kept as
/// the text the server answered with, and the parts the launcher shows are
/// read out of it.
///
/// Two servers are the same server when they have the same address, which is
/// what the launcher of Java compares as well; the metadata a server answered
/// with is not a part of its identity, because the launcher may read it again
/// at any time.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthlibInjectorServer {
    /// The address of the endpoint of Yggdrasil of the server.
    pub url: String,
    /// The text of the metadata the server answered with.
    #[serde(
        rename = "metadataResponse",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    metadata_response: Option<String>,
    /// The moment the metadata was read, in milliseconds since the epoch.
    #[serde(
        rename = "metadataTimestamp",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    metadata_timestamp: Option<u64>,
    /// The parts of the metadata the pages of the launcher show, which the
    /// launcher reads out of the text again when it starts, so the text is
    /// the only thing the file keeps.
    #[serde(skip)]
    metadata: Option<AuthlibInjectorMetadata>,
    /// Whether the metadata was read from the server during this run.
    ///
    /// The launcher reads the metadata once and keeps it, so a server that is
    /// asked again during the same run answers the text the launcher already
    /// holds. The flag is not stored, because a metadata the launcher read in
    /// an earlier run is not fresh any more.
    #[serde(skip)]
    metadata_refreshed: bool,
}

impl PartialEq for AuthlibInjectorServer {
    /// Compares the servers by their address.
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
    }
}

impl Eq for AuthlibInjectorServer {}

impl fmt::Display for AuthlibInjectorServer {
    /// Writes the address of the server, and its name when it told one.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.metadata.as_ref().and_then(|meta| meta.name.as_deref()) {
            Some(name) => write!(formatter, "{} ({name})", self.url),
            None => formatter.write_str(&self.url),
        }
    }
}

impl AuthlibInjectorServer {
    /// Creates a server that the launcher has not read the metadata of yet.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: normalize_url(&url.into()),
            metadata_response: None,
            metadata_timestamp: None,
            metadata: None,
            metadata_refreshed: false,
        }
    }

    /// Finds the server the player named and reads its metadata.
    ///
    /// The player usually names the page of a server rather than the endpoint
    /// of Yggdrasil, so the server names the address of its API in the header
    /// [`API_LOCATION_HEADER`]. The header is resolved against the address the
    /// server answered from, and the address it names is asked instead, unless
    /// it is the address that was asked, with or without the slash at its end.
    ///
    /// The metadata is read from the answer of the last request, which is the
    /// way the launcher of Java reads it; a server that cannot be reached, or
    /// that answers with a status that is not a success, is reported as a
    /// failure of the network, and an answer the launcher cannot read as
    /// malformed metadata.
    pub fn locate_server(client: &dyn HttpClient, url: &str) -> Result<Self, AuthlibInjectorError> {
        let requested = normalize_url(url);

        let response = send(
            client,
            &HttpRequest::get(requested.clone())
                .ignore_status()
                .attempts(5),
        )
        .map_err(|failure| AuthlibInjectorError::Network(failure.to_string()))?;

        // The address of the API is resolved against the address the answer
        // came from, which is the address of the last redirect.
        let base = if response.url.is_empty() {
            requested.clone()
        } else {
            response.url.clone()
        };
        // The header is read before the body is taken out of the answer.
        let location = response.header(API_LOCATION_HEADER).map(str::to_string);

        let mut server_url = requested.clone();
        let mut body = response.body;

        if let Some(location) = location {
            if let Some(api_location) = crate::util::http::resolve_redirect(&base, &location) {
                if !url_equals_ignore_slash(&requested, &api_location) {
                    let answer = send(
                        client,
                        &HttpRequest::get(api_location.clone())
                            .ignore_status()
                            .attempts(5),
                    )
                    .map_err(|failure| AuthlibInjectorError::Network(failure.to_string()))?;
                    server_url = api_location;
                    body = answer.body;
                }
            }
        }

        let mut server = Self::new(server_url);
        server.refresh_metadata_from(body)?;
        Ok(server)
    }

    /// Returns the address of the server.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Returns the text of the metadata the server answered with.
    pub fn metadata_response(&self) -> Option<&str> {
        self.metadata_response.as_deref()
    }

    /// Returns the moment the metadata was read, in milliseconds since the
    /// epoch, or `None` when the launcher has not read it yet.
    pub const fn metadata_timestamp(&self) -> Option<u64> {
        self.metadata_timestamp
    }

    /// Reads the metadata of the server from the text the server answered
    /// with.
    ///
    /// The launcher writes the text as it was answered, so a value the
    /// launcher does not know is kept. The name, the links of the page of the
    /// server and whether the server lets a player sign in without an address
    /// of a mailbox are the parts the pages of the launcher show.
    pub fn set_metadata_response(
        &mut self,
        metadata_response: impl Into<String>,
        metadata_timestamp: u64,
    ) -> Result<(), AuthlibInjectorError> {
        let metadata_response = metadata_response.into();

        let response: serde_json::Value = if metadata_response.trim().is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_str(&metadata_response)
                .map_err(|error| AuthlibInjectorError::MalformedMetadata(error.to_string()))?
        };

        // The launcher of Java reads the text as an object and refuses an
        // answer that names nothing, because a server that answered nothing
        // cannot be shown to the player.
        if response.is_null() {
            return Err(AuthlibInjectorError::MalformedMetadata(
                "Metadata response is empty".to_string(),
            ));
        }

        let meta = response.get("meta");

        let name = meta
            .and_then(|meta| meta.get("serverName"))
            .and_then(primitive_text);

        let links = meta
            .and_then(|meta| meta.get("links"))
            .and_then(serde_json::Value::as_object)
            .map(|link_object| {
                link_object
                    .iter()
                    .filter_map(|(name, value)| {
                        primitive_text(value).map(|value| (name.clone(), value))
                    })
                    .collect::<BTreeMap<String, String>>()
            })
            .unwrap_or_default();

        let non_email_login = meta
            .and_then(|meta| meta.get("feature.non_email_login"))
            .and_then(primitive_boolean)
            .unwrap_or(false);

        self.metadata_response = Some(metadata_response);
        self.metadata_timestamp = Some(metadata_timestamp);
        self.metadata = Some(AuthlibInjectorMetadata {
            name,
            links,
            non_email_login,
        });
        Ok(())
    }

    /// Returns the metadata of the server, asking the server for it when the
    /// launcher does not hold a fresh one.
    ///
    /// The metadata is read once during a run of the launcher; a caller that
    /// invalidated the cache with [`Self::invalidate_metadata_cache`] gets the
    /// text the server answers now.
    pub fn fetch_metadata_response(
        &mut self,
        client: &dyn HttpClient,
    ) -> Result<String, AuthlibInjectorError> {
        if self.metadata_response.is_none() || !self.metadata_refreshed {
            self.refresh_metadata(client)?;
        }

        // The refreshment stored the text, so the text is there.
        Ok(self.metadata_response.clone().unwrap_or_default())
    }

    /// Reads the metadata of the server over the network.
    ///
    /// The metadata is read from the address of the server, which is the
    /// endpoint of Yggdrasil the server answers on. A server that answers
    /// something the launcher cannot read is reported as such, with the text
    /// the server wrote, because the player may have named the address of
    /// another page.
    pub fn refresh_metadata(
        &mut self,
        client: &dyn HttpClient,
    ) -> Result<(), AuthlibInjectorError> {
        let response = send(
            client,
            &HttpRequest::get(self.url.clone())
                .ignore_status()
                .attempts(5),
        )
        .map_err(|failure| AuthlibInjectorError::Network(failure.to_string()))?;

        self.refresh_metadata_from(response.body)
    }

    /// Reads the metadata of the server from the text an answer held.
    ///
    /// The text is the one a server answered with, whether the answer was a
    /// success or not, which is what the launcher of Java reads: a server that
    /// refuses the address of its API writes the reason in the body, and the
    /// reason is shown to the player as the text of the answer.
    fn refresh_metadata_from(&mut self, text: String) -> Result<(), AuthlibInjectorError> {
        // The launcher of Java reports an answer it cannot read with the text
        // the server wrote, because the player may have named the address of
        // another page; the reason of the parser is not shown.
        self.set_metadata_response(text.clone(), unix_millis() as u64)
            .map_err(|_| {
                AuthlibInjectorError::MalformedMetadata(format!("Malformed response\n{text}"))
            })?;

        self.metadata_refreshed = true;
        logging::info(&format!(
            "authlib-injector server metadata refreshed: {}",
            self.url
        ));
        Ok(())
    }

    /// Restores a metadata response the launcher read earlier.
    ///
    /// The text is read the way [`Self::set_metadata_response`] reads it, but
    /// the metadata is not marked as fresh: the launcher of Java reads the
    /// metadata of a server again before it starts the game with it.
    pub fn restore_metadata_cache(
        &mut self,
        metadata_response: impl Into<String>,
        metadata_timestamp: u64,
    ) -> Result<(), AuthlibInjectorError> {
        self.set_metadata_response(metadata_response, metadata_timestamp)
    }

    /// Marks the metadata the launcher holds as stale.
    ///
    /// The text is kept, so a server that cannot be reached still shows the
    /// name it told; the next call of [`Self::fetch_metadata_response`] asks
    /// the server again.
    pub const fn invalidate_metadata_cache(&mut self) {
        self.metadata_refreshed = false;
    }

    /// Returns the name of the server, or its address when it names none.
    pub fn display_name(&self) -> String {
        self.metadata
            .as_ref()
            .and_then(|meta| meta.name.clone())
            .unwrap_or_else(|| self.url.clone())
    }

    /// Returns whether the server lets a player sign in without an address
    /// of a mailbox.
    pub fn non_email_login(&self) -> bool {
        self.metadata
            .as_ref()
            .is_some_and(|meta| meta.non_email_login)
    }

    /// Returns the links the server tells, sorted by their name.
    pub fn links(&self) -> &BTreeMap<String, String> {
        self.metadata
            .as_ref()
            .map(|meta| &meta.links)
            .unwrap_or(&EMPTY_LINKS)
    }
}

/// The metadata of a server the launcher shows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuthlibInjectorMetadata {
    /// The name of the server.
    name: Option<String>,
    /// The links of the page of the server, sorted by their name.
    links: BTreeMap<String, String>,
    /// Whether the server lets a player sign in without an address of a
    /// mailbox.
    non_email_login: bool,
}

/// The empty list of the links of a server.
static EMPTY_LINKS: BTreeMap<String, String> = BTreeMap::new();

/// The list of the servers the player uses.
///
/// The list is what the pages of the accounts show; the launcher reads it
/// from `config/authlib-injector-servers.json` and writes it back, so the
/// player finds the servers it named again the next time it starts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthlibInjectorServerList {
    /// The schema of the file, which the launcher compares with the one it
    /// knows.
    #[serde(rename = "$schema", default = "default_servers_schema")]
    schema: String,
    /// The servers the player named, in the order the player added them.
    #[serde(default)]
    servers: Vec<AuthlibInjectorServer>,
}

impl AuthlibInjectorServerList {
    /// Creates a list that names no server.
    ///
    /// The launcher starts with a list that names LittleSkin, which
    /// [`AuthlibInjectorServerList::new`] creates; this one is for the
    /// migration of the settings of an older launcher, which names the
    /// servers itself.
    pub fn empty() -> Self {
        Self {
            schema: AUTHLIB_INJECTOR_SERVERS_SCHEMA.to_string(),
            servers: Vec::new(),
        }
    }

    /// Creates the list the launcher starts with, which names LittleSkin.
    pub fn new() -> Self {
        let mut list = Self::empty();
        list.add_little_skin_if_absent();
        list
    }

    /// Returns the schema of the file.
    pub fn schema(&self) -> &str {
        &self.schema
    }

    /// Returns the servers the player named, in the order they were added.
    pub fn servers(&self) -> &[AuthlibInjectorServer] {
        &self.servers
    }

    /// Returns the server of an address, if the player named it.
    pub fn get(&self, url: &str) -> Option<&AuthlibInjectorServer> {
        let url = normalize_url(url);
        self.servers.iter().find(|server| server.url == url)
    }

    /// Adds a server, and returns whether the player added it.
    ///
    /// A server the player already named is not added a second time; the
    /// address the player wrote is normalized first, so the same server is
    /// found it names it another way.
    pub fn add(&mut self, url: &str) -> bool {
        self.add_server(AuthlibInjectorServer::new(url))
    }

    /// Adds a server the launcher already read the metadata of, and returns
    /// whether the player added it.
    ///
    /// A server the list names again is replaced, so the list holds the
    /// metadata the server answered with the last time it was asked; this is
    /// what the launcher does when a server that could not be reached before
    /// answers at last.
    pub fn add_server(&mut self, server: AuthlibInjectorServer) -> bool {
        match self
            .servers
            .iter()
            .position(|named| named.url == server.url)
        {
            Some(index) => {
                self.servers[index] = server;
                false
            }
            None => {
                self.servers.push(server);
                true
            }
        }
    }

    /// Returns the server of an address, naming it when nobody named it yet.
    ///
    /// This is what the launcher of Java gives the accounts: an account the
    /// launcher read from its settings names the address of its server, and
    /// the launcher either finds that server in the list or names it now, so
    /// an account whose server the player removed still signs in.
    pub fn get_or_add(&mut self, url: &str) -> AuthlibInjectorServer {
        let url = normalize_url(url);
        if let Some(server) = self.servers.iter().find(|server| server.url == url) {
            return server.clone();
        }

        let server = AuthlibInjectorServer::new(url);
        self.servers.push(server.clone());
        server
    }

    /// Returns the entry of a server, so the metadata of it can be written
    /// back.
    ///
    /// The metadata the cache of the launcher holds is the only memory of what
    /// a server answered with, and the list the player reads shows the entries
    /// of the cache; putting the metadata back into the entry of the list is
    /// what the launcher of Java does when it binds its cache to the list.
    pub fn server_mut(&mut self, url: &str) -> Option<&mut AuthlibInjectorServer> {
        let url = normalize_url(url);
        self.servers.iter_mut().find(|server| server.url == url)
    }

    /// Names LittleSkin when the player has not named it yet.
    pub fn add_little_skin_if_absent(&mut self) {
        let _ = self.add(LITTLE_SKIN_URL);
    }

    /// Removes a server, and returns whether the player removed one.
    pub fn remove(&mut self, url: &str) -> bool {
        let url = normalize_url(url);
        let Some(index) = self.servers.iter().position(|server| server.url == url) else {
            return false;
        };
        self.servers.remove(index);
        true
    }
}

impl Default for AuthlibInjectorServerList {
    fn default() -> Self {
        Self::new()
    }
}

impl super::schema::SchemaSettings for AuthlibInjectorServerList {
    fn schema(&self) -> &str {
        &self.schema
    }

    fn set_schema(&mut self, schema: &str) {
        self.schema = schema.to_string();
    }
}

impl AuthlibInjectorServerList {
    /// Reads the list from `config/authlib-injector-servers.json` of the
    /// directory of the launcher.
    pub fn load(user_home: &std::path::Path) -> Result<super::storage::SettingsFile<Self>, Error> {
        super::storage::SettingsFile::load_with_schema(
            authlib_injector_servers_path(user_home),
            &AuthlibInjectorServerList::schema_of(),
        )
    }

    /// Returns the schema the launcher asks for.
    pub fn schema_of() -> Schema {
        Schema::new("authlib-injector-servers", SchemaVersion::new(1, 0, 0))
    }
}

/// The metadata the launcher read from every server.
///
/// The launcher keeps the metadata apart from the list the player edits, so
/// an update of a server does not ask the player to name it again. The cache
/// is read from `cache/authlib-injector-server-metadata.json`, which the
/// launcher writes whenever it read the metadata of a server.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthlibInjectorServerMetadataCache {
    /// The schema of the file, which the launcher compares with the one it
    /// knows.
    #[serde(rename = "$schema", default = "default_metadata_schema")]
    schema: String,
    /// The metadata, in the order the servers were named.
    #[serde(default)]
    servers: Vec<AuthlibInjectorServer>,
}

impl AuthlibInjectorServerMetadataCache {
    /// Creates the cache of no server.
    pub fn new() -> Self {
        Self {
            schema: AUTHLIB_INJECTOR_METADATA_SCHEMA.to_string(),
            servers: Vec::new(),
        }
    }

    /// Returns the schema of the file.
    pub fn schema(&self) -> &str {
        &self.schema
    }

    /// Returns the cache entries, in the order the servers were named.
    pub fn entries(&self) -> &[AuthlibInjectorServer] {
        &self.servers
    }

    /// Restores the metadata of a server from the cache.
    ///
    /// A cache entry whose metadata the launcher cannot read is dropped, so
    /// the next time the launcher starts it asks the server for its metadata
    /// again; this is the way the launcher of Java treats a cache it wrote and
    /// the server answered the cache was stale.
    pub fn restore(&mut self, url: &str) -> Option<AuthlibInjectorServer> {
        let url = normalize_url(url);
        let index = self.index_of(&url)?;

        let entry = &self.servers[index];
        let Some(response) = entry
            .metadata_response()
            .filter(|response| !response.is_empty())
        else {
            self.servers.remove(index);
            return None;
        };

        let mut restored = AuthlibInjectorServer::new(&entry.url);
        match restored
            .set_metadata_response(response, entry.metadata_timestamp().unwrap_or_default())
        {
            Ok(()) => Some(restored),
            Err(_) => {
                self.servers.remove(index);
                None
            }
        }
    }

    /// Keeps the metadata the launcher read from a server.
    pub fn store(&mut self, server: &AuthlibInjectorServer) {
        let Some(response) = server.metadata_response().map(str::to_string) else {
            return;
        };

        let entry = AuthlibInjectorServer {
            url: server.url.clone(),
            metadata_response: Some(response),
            metadata_timestamp: server.metadata_timestamp(),
            // The cache keeps the text of the metadata only: the parts the
            // pages show are read out of the text again when the launcher
            // restores the entry.
            metadata: None,
            metadata_refreshed: false,
        };

        match self.index_of(&entry.url) {
            Some(index) => self.servers[index] = entry,
            None => self.servers.push(entry),
        }
    }

    /// Drops the metadata of a server the player removed.
    pub fn remove(&mut self, url: &str) -> bool {
        let Some(index) = self.index_of(&normalize_url(url)) else {
            return false;
        };
        self.servers.remove(index);
        true
    }

    /// Returns the position of the cache entry of an address.
    fn index_of(&self, url: &str) -> Option<usize> {
        self.servers.iter().position(|entry| entry.url == url)
    }
}

impl AuthlibInjectorServerMetadataCache {
    /// Reads the cache from `cache/authlib-injector-server-metadata.json` of
    /// the directory of the launcher.
    pub fn load(user_home: &std::path::Path) -> Result<super::storage::SettingsFile<Self>, Error> {
        super::storage::SettingsFile::load_with_schema(
            authlib_injector_metadata_path(user_home),
            &Self::schema_of(),
        )
    }

    /// Returns the schema the launcher asks for.
    pub fn schema_of() -> Schema {
        Schema::new(
            "authlib-injector-server-metadata",
            SchemaVersion::new(1, 0, 0),
        )
    }
}

impl super::schema::SchemaSettings for AuthlibInjectorServerMetadataCache {
    fn schema(&self) -> &str {
        &self.schema
    }

    fn set_schema(&mut self, schema: &str) {
        self.schema = schema.to_string();
    }
}

/// Returns the path of the file that holds the list of the servers.
pub fn authlib_injector_servers_path(user_home: &Path) -> PathBuf {
    super::config_directory(user_home).join(AUTHLIB_INJECTOR_SERVERS_FILE)
}

/// Returns the path of the file that holds the metadata of the servers.
pub fn authlib_injector_metadata_path(user_home: &Path) -> PathBuf {
    super::cache_directory(user_home).join(AUTHLIB_INJECTOR_METADATA_FILE)
}

/// A failure the launcher had with a server of the authlib-injector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthlibInjectorError {
    /// The server could not be reached, or it answered with a status that is
    /// not a success.
    Network(String),
    /// The metadata the server answered with is not JSON the launcher reads.
    MalformedMetadata(String),
}

impl fmt::Display for AuthlibInjectorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(message) => {
                write!(
                    formatter,
                    "the server of the authlib-injector could not be read: {message}"
                )
            }
            Self::MalformedMetadata(message) => {
                write!(
                    formatter,
                    "the metadata of the server is malformed: {message}"
                )
            }
        }
    }
}

impl std::error::Error for AuthlibInjectorError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::http_request::HttpResponse;

    #[test]
    fn the_address_of_a_server_pages_show_is_the_host() {
        assert_eq!(
            display_host_url("https://example.com/api/yggdrasil/"),
            "example.com"
        );
        assert_eq!(
            display_host_url("https://example.com:8443/api/yggdrasil/"),
            "example.com:8443"
        );
        assert_eq!(
            display_host_url("https://example.com/some/path"),
            "example.com/some/path"
        );
        assert_eq!(
            display_host_url("http://example.com/api/yggdrasil/"),
            "http://example.com/api/yggdrasil/"
        );
        assert_eq!(display_host_url("not a url"), "not a url");
    }

    #[test]
    fn the_address_the_player_wrote_is_normalized() {
        assert_eq!(normalize_url("example.com"), "https://example.com/");
        assert_eq!(normalize_url("//example.com"), "https://example.com/");
        assert_eq!(normalize_url("http://example.com"), "http://example.com/");
        assert_eq!(normalize_url("example.com/"), "https://example.com/");
        assert_eq!(
            normalize_url("example.com/api/"),
            "https://example.com/api/"
        );
    }

    #[test]
    fn the_default_list_names_littleskin() {
        let mut list = AuthlibInjectorServerList::new();

        assert_eq!(list.servers().len(), 1);
        assert_eq!(list.servers()[0].url(), LITTLE_SKIN_URL);
        // The launcher does not add a server twice.
        list.add_little_skin_if_absent();
        assert_eq!(list.servers().len(), 1);
    }

    #[test]
    fn servers_are_found_under_another_name_of_the_same_address() {
        let mut list = AuthlibInjectorServerList::new();

        // The address names the server the list names already, so no server
        // is added a second time.
        assert!(!list.add("littleskin.cn/api/yggdrasil"));
        assert_eq!(list.servers().len(), 1);

        assert!(list.remove("https://littleskin.cn/api/yggdrasil"));
        assert!(list.servers().is_empty());
        assert!(!list.remove("https://littleskin.cn/api/yggdrasil"));
    }

    #[test]
    fn a_server_the_list_names_again_replaces_the_entry_of_the_address() {
        let mut list = AuthlibInjectorServerList::new();

        let mut server = AuthlibInjectorServer::new("example.com/api/yggdrasil");
        server
            .set_metadata_response(metadata_text("Example"), 1000)
            .expect("the metadata should be read");

        assert!(list.add_server(server));
        assert_eq!(list.servers().len(), 2);
        assert_eq!(list.servers()[1].display_name(), "Example");

        // The launcher asks the same server again, so the entry of its
        // address is replaced rather than named a second time.
        let mut again = AuthlibInjectorServer::new("example.com/api/yggdrasil");
        again
            .set_metadata_response(metadata_text("Asked again"), 2000)
            .expect("the metadata should be read");
        assert!(!list.add_server(again));
        assert_eq!(list.servers().len(), 2);
        assert_eq!(list.servers()[1].display_name(), "Asked again");
    }

    #[test]
    fn the_server_of_an_account_names_its_server_when_it_is_not_there() {
        let mut list = AuthlibInjectorServerList::new();

        // An account that names LittleSkin finds the server the list names.
        let found = list.get_or_add(LITTLE_SKIN_URL);
        assert_eq!(found.url(), LITTLE_SKIN_URL);
        assert_eq!(list.servers().len(), 1);

        // A server the player removed is named again, which is what keeps an
        // account whose server the list no longer holds able to sign in.
        let created = list.get_or_add("example.com/api/yggdrasil");
        assert_eq!(created.url(), "https://example.com/api/yggdrasil/");
        assert_eq!(list.servers().len(), 2);
        assert_eq!(
            list.get("example.com/api/yggdrasil")
                .map(|server| server.url()),
            Some("https://example.com/api/yggdrasil/")
        );
    }

    #[test]
    fn the_entry_of_a_server_is_reachable_for_the_metadata_of_the_cache() {
        let mut list = AuthlibInjectorServerList::new();

        // The cache of the metadata is the only memory of what a server
        // answered with; the entry of the list is where it is written back.
        list.add("example.com/api/yggdrasil");
        let mut restored = AuthlibInjectorServer::new("example.com/api/yggdrasil");
        restored
            .set_metadata_response(metadata_text("Example"), 1000)
            .expect("the metadata should be read");

        let entry = list
            .server_mut("example.com/api/yggdrasil")
            .expect("the server should be in the list");
        let _ = entry.restore_metadata_cache(
            restored
                .metadata_response()
                .expect("the metadata should be there"),
            restored.metadata_timestamp().unwrap_or_default(),
        );

        // The server no server was asked for, but the pages of the launcher
        // show the name it told the last time.
        assert_eq!(list.servers()[1].display_name(), "Example");
        assert_eq!(list.servers()[1].metadata_timestamp(), Some(1000));

        // A server the list does not name is not found.
        assert!(list.server_mut("other.example.com").is_none());
    }

    /// Returns the text of the metadata of a server the tests read.
    fn metadata_text(server_name: &str) -> String {
        format!(
            r#"{{"meta": {{"serverName": "{server_name}", "links": {{}}, "feature.non_email_login": true}}}}"#
        )
    }

    #[test]
    fn the_metadata_of_a_server_is_read() {
        let mut server = AuthlibInjectorServer::new("example.com/api/yggdrasil");

        server
            .set_metadata_response(metadata_text("Example"), 1000)
            .expect("the metadata should be read");

        assert_eq!(server.display_name(), "Example");
        assert!(server.non_email_login());
        assert_eq!(server.metadata_timestamp(), Some(1000));
        assert_eq!(server.links().len(), 0);
    }

    #[test]
    fn the_metadata_of_a_server_that_is_not_json_is_refused() {
        let mut server = AuthlibInjectorServer::new("example.com");
        let error = server.set_metadata_response("not json", 1000);
        assert!(error.is_err());
        assert_eq!(server.display_name(), "https://example.com/");
        assert_eq!(server.metadata_response(), None);
    }

    #[test]
    fn the_list_is_kept_in_the_config_directory() {
        let user_home = Path::new("/home/user/.hmcl");
        let path = authlib_injector_servers_path(user_home);
        assert!(
            path.ends_with("config/authlib-injector-servers.json"),
            "{path:?}"
        );
    }

    #[test]
    fn the_metadata_is_kept_in_the_cache_directory() {
        let user_home = Path::new("/home/user/.hmcl");
        let path = authlib_injector_metadata_path(user_home);
        assert!(
            path.ends_with("cache/authlib-injector-server-metadata.json"),
            "{path:?}"
        );
    }

    #[test]
    fn the_metadata_of_a_server_is_kept_and_restored() {
        let mut cache = AuthlibInjectorServerMetadataCache::new();
        let mut server = AuthlibInjectorServer::new("example.com");
        server
            .set_metadata_response(metadata_text("Example"), 1000)
            .expect("the metadata should be read");

        cache.store(&server);
        assert_eq!(cache.entries().len(), 1);

        // The launcher restores the metadata without asking the server again.
        let restored = cache
            .restore("example.com")
            .expect("the cache should be read");
        assert_eq!(restored.display_name(), "Example");
        assert_eq!(restored.metadata_timestamp(), Some(1000));

        // A server the player removed has no cache left.
        assert!(cache.remove("example.com"));
        assert!(cache.restore("example.com").is_none());
        assert!(!cache.remove("example.com"));
    }

    #[test]
    fn a_cache_entry_with_no_metadata_is_dropped() {
        let mut cache = AuthlibInjectorServerMetadataCache::new();
        cache.store(&AuthlibInjectorServer::new("example.com"));

        // The empty metadata is not restored and the entry is dropped.
        assert!(cache.restore("example.com").is_none());
        assert!(cache.entries().is_empty());
    }

    #[test]
    fn a_cache_entry_that_is_not_json_is_dropped() {
        let mut cache = AuthlibInjectorServerMetadataCache::new();
        let mut server = AuthlibInjectorServer::new("example.com");
        // The launcher keeps the text it was answered with, even when it is
        // not JSON the launcher reads.
        server.metadata_response = Some("not json".to_string());
        server.metadata_timestamp = Some(1000);
        cache.store(&server);

        assert!(cache.restore("example.com").is_none());
        assert!(cache.entries().is_empty());
    }

    #[test]
    fn the_files_the_list_and_the_cache_are_kept_in_use_the_schema() {
        let json = serde_json::to_value(AuthlibInjectorServerList::new())
            .expect("the list should be written");
        assert_eq!(json["$schema"], AUTHLIB_INJECTOR_SERVERS_SCHEMA);

        let json = serde_json::to_value(AuthlibInjectorServerMetadataCache::new())
            .expect("the cache should be written");
        assert_eq!(json["$schema"], AUTHLIB_INJECTOR_METADATA_SCHEMA);
        assert_eq!(json["servers"], serde_json::json!([]));
    }

    /// A client that answers with the answers it is given and keeps the
    /// requests it was asked.
    struct FakeClient {
        /// The answers, in the order they are returned.
        answers: std::cell::RefCell<Vec<Result<HttpResponse, String>>>,
        /// The addresses the client was asked, in the order they were asked.
        asked: std::cell::RefCell<Vec<String>>,
    }

    impl FakeClient {
        fn new(answers: Vec<Result<HttpResponse, String>>) -> Self {
            Self {
                answers: std::cell::RefCell::new(answers),
                asked: std::cell::RefCell::new(Vec::new()),
            }
        }
    }

    impl HttpClient for FakeClient {
        fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, String> {
            self.asked.borrow_mut().push(request.url().to_string());
            let mut answers = self.answers.borrow_mut();
            if answers.is_empty() {
                return Err("the client was asked too often".to_string());
            }
            answers.remove(0)
        }
    }

    #[test]
    fn a_link_of_a_server_is_read_from_the_text_the_player_dropped() {
        assert_eq!(
            parse_dropped_url(
                "authlib-injector:yggdrasil-server:https%3A%2F%2Fexample.com%2Fapi%2Fyggdrasil"
            ),
            Some("https://example.com/api/yggdrasil".to_string())
        );
        // A space of the address is written as a plus, which is read back as a
        // space.
        assert_eq!(
            parse_dropped_url("authlib-injector:yggdrasil-server:a+b"),
            Some("a b".to_string())
        );
        // The text has to be made of exactly three parts, so a text that is
        // not a link of a server names no server.
        assert_eq!(parse_dropped_url(""), None);
        assert_eq!(parse_dropped_url("https://example.com"), None);
        assert_eq!(
            parse_dropped_url("authlib-injector:yggdrasil-server:https://example.com"),
            None
        );
        assert_eq!(
            parse_dropped_url("authlib-injector:other-server:example.com"),
            None
        );
        assert_eq!(
            parse_dropped_url("other:yggdrasil-server:example.com"),
            None
        );
    }

    #[test]
    fn the_text_of_the_metadata_is_kept_as_the_server_wrote_it() {
        let mut server = AuthlibInjectorServer::new("example.com");
        // The text holds a value the launcher does not read and a formatting
        // the launcher does not write, both of which are kept.
        let text = r#"{ "meta": { "serverName": "Example" }, "unknown": [1, 2] }"#;
        server
            .set_metadata_response(text, 1000)
            .expect("the metadata should be read");

        assert_eq!(server.metadata_response(), Some(text));
    }

    #[test]
    fn the_metadata_that_is_empty_is_refused() {
        let mut server = AuthlibInjectorServer::new("example.com");

        // A server that answered nothing, or the text `null`, names nothing
        // the pages of the launcher may show.
        for text in ["", "  ", "null"] {
            let error = server
                .set_metadata_response(text, 1000)
                .expect_err("the metadata should be refused");
            assert_eq!(
                error,
                AuthlibInjectorError::MalformedMetadata("Metadata response is empty".to_string())
            );
        }

        assert_eq!(server.metadata_response(), None);
        assert_eq!(server.display_name(), "https://example.com/");
    }

    #[test]
    fn the_values_of_the_metadata_are_read_the_way_java_reads_them() {
        let mut server = AuthlibInjectorServer::new("example.com");
        // Java reads a primitive as text, so a number names a server as well;
        // a link that is not a primitive is skipped, and only the text `true`
        // turns a feature on.
        server
            .set_metadata_response(
                r#"{
                    "meta": {
                        "serverName": 42,
                        "links": {"homepage": "https://example.com", "other": [1]},
                        "feature.non_email_login": "TRUE"
                    }
                }"#,
                1000,
            )
            .expect("the metadata should be read");

        assert_eq!(server.display_name(), "42");
        assert_eq!(server.links().len(), 1);
        assert_eq!(
            server.links().get("homepage").map(String::as_str),
            Some("https://example.com")
        );
        assert!(server.non_email_login());
    }

    #[test]
    fn the_metadata_of_a_server_is_read_from_the_server() {
        let client = FakeClient::new(vec![Ok(HttpResponse::new(200, metadata_text("Example")))]);
        let mut server = AuthlibInjectorServer::new("example.com");

        let text = server
            .fetch_metadata_response(&client)
            .expect("the metadata should be read");

        assert_eq!(text, metadata_text("Example"));
        assert_eq!(server.display_name(), "Example");
        assert_eq!(
            client.asked.borrow().as_slice(),
            ["https://example.com/".to_string()]
        );
    }

    #[test]
    fn the_metadata_that_was_read_is_not_read_again_during_a_run() {
        let client = FakeClient::new(vec![Ok(HttpResponse::new(200, metadata_text("Example")))]);
        let mut server = AuthlibInjectorServer::new("example.com");

        server
            .fetch_metadata_response(&client)
            .expect("the metadata should be read");
        server
            .fetch_metadata_response(&client)
            .expect("the metadata should be kept");

        // The server is asked once, and the second call is answered from the
        // text the launcher holds.
        assert_eq!(client.asked.borrow().len(), 1);
    }

    #[test]
    fn a_stale_metadata_is_read_again() {
        let client = FakeClient::new(vec![
            Ok(HttpResponse::new(200, metadata_text("Before"))),
            Ok(HttpResponse::new(200, metadata_text("After"))),
        ]);
        let mut server = AuthlibInjectorServer::new("example.com");

        server
            .fetch_metadata_response(&client)
            .expect("the metadata should be read");
        server.invalidate_metadata_cache();

        // The text the launcher held is kept until the server answers again,
        // so the pages still show the name of the server.
        assert_eq!(server.display_name(), "Before");

        server
            .fetch_metadata_response(&client)
            .expect("the metadata should be read again");
        assert_eq!(server.display_name(), "After");
        assert_eq!(client.asked.borrow().len(), 2);
    }

    #[test]
    fn a_server_is_found_from_the_page_the_player_named() {
        // The player names the page of the server, which answers with the
        // address of its API; the launcher asks that address and reads the
        // metadata it answers with.
        let client = FakeClient::new(vec![
            Ok(
                HttpResponse::new(200, "<html>the page of the server</html>")
                    .with_url("https://example.com/")
                    .with_header(API_LOCATION_HEADER, "https://example.com/api/yggdrasil/"),
            ),
            Ok(HttpResponse::new(200, metadata_text("Example"))),
        ]);

        let server = AuthlibInjectorServer::locate_server(&client, "example.com")
            .expect("the server should be found");

        assert_eq!(server.url(), "https://example.com/api/yggdrasil/");
        assert_eq!(server.display_name(), "Example");
        assert_eq!(
            client.asked.borrow().as_slice(),
            [
                "https://example.com/".to_string(),
                "https://example.com/api/yggdrasil/".to_string()
            ]
        );
    }

    #[test]
    fn an_api_location_that_names_the_asked_address_is_not_asked_again() {
        // The server names its own address, which the launcher asked already;
        // the metadata is read from the answer of the one request.
        let client = FakeClient::new(vec![Ok(HttpResponse::new(200, metadata_text("Example"))
            .with_header(API_LOCATION_HEADER, "https://example.com"))]);

        let server = AuthlibInjectorServer::locate_server(&client, "example.com")
            .expect("the server should be found");

        assert_eq!(server.url(), "https://example.com/");
        assert_eq!(client.asked.borrow().len(), 1);
    }

    #[test]
    fn an_api_location_is_resolved_against_the_address_of_the_answer() {
        // The address of the API is relative, so it is resolved against the
        // address the answer came from, which is the address of the last
        // redirect rather than the one the launcher asked.
        let client = FakeClient::new(vec![
            Ok(HttpResponse::new(200, "<html></html>")
                .with_url("https://example.com/page/")
                .with_header(API_LOCATION_HEADER, "../api/yggdrasil")),
            Ok(HttpResponse::new(200, metadata_text("Example"))),
        ]);

        let server = AuthlibInjectorServer::locate_server(&client, "example.com")
            .expect("the server should be found");

        assert_eq!(server.url(), "https://example.com/api/yggdrasil/");
    }

    #[test]
    fn a_server_that_cannot_be_reached_is_a_failure_of_the_network() {
        let client = FakeClient::new(vec![Err("the connection broke".to_string())]);

        let error = AuthlibInjectorServer::locate_server(&client, "example.com")
            .expect_err("the server should not be found");

        assert!(
            matches!(error, AuthlibInjectorError::Network(_)),
            "{error:?}"
        );
    }

    #[test]
    fn a_metadata_the_server_wrote_wrongly_is_reported_with_its_text() {
        let client = FakeClient::new(vec![Ok(HttpResponse::new(200, "not json"))]);
        let mut server = AuthlibInjectorServer::new("example.com");

        let error = server
            .fetch_metadata_response(&client)
            .expect_err("the metadata should be refused");

        // The player may have named the address of another page, so the text
        // the server wrote is shown.
        assert_eq!(
            error,
            AuthlibInjectorError::MalformedMetadata("Malformed response\nnot json".to_string())
        );
        // The launcher did not keep the answer, so the next call asks again.
        assert_eq!(server.metadata_response(), None);
    }
}
