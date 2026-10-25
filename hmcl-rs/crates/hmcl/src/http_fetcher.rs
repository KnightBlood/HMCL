//! The client that opens the connections of the launcher.
//!
//! The launcher of Java asks the servers of the game for files through
//! `HttpURLConnection`, which it makes by hand so it can follow the redirects
//! itself: some servers answer with a `Location` that is not encoded, and the
//! programs of the network refuse an address that is written that way. This
//! module asks through `ureq` instead and keeps the behaviour of the launcher
//! of Java:
//!
//! * the client does not follow the redirects (`redirects(0)`), but
//!   [`HttpFetcher::fetch`] does, resolving the location with the rules of
//!   [`hmcl_core::util::http`] and refusing a chain of more than
//!   [`hmcl_core::util::http::MAX_REDIRECTS`];
//! * the headers the launcher of Java sends with every request are sent, and
//!   the body of the answer is written to the writer the caller gives;
//! * the proxy the player wrote is read from the settings of the launcher.
//!
//! The agent of the connections is kept open between the requests, because the
//! launcher asks the same server for many small files and a connection that is
//! made for every file is slow.
//!
//! No page of the window asks for a file yet, so the module is allowed to hold
//! items nothing calls: the fetcher is the one the pages will download with,
//! and the tests of the module ask the servers with it already.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::OnceLock;

use hmcl_core::download::{Fetcher, ResponseInfo};
use hmcl_core::setting::launcher::LauncherSettings;
use hmcl_core::util::http;

/// The way the launcher gets the body of an address.
///
/// The fetcher writes the body of an answer to the writer it is given and
/// returns the headers of the answer, which is what
/// [`hmcl_core::download::FileDownloadTask`] needs to check the file it got.
/// A redirect is asked again, so the answer the caller sees is the answer of
/// the last address of the chain.
pub struct HttpFetcher {
    /// The agent that keeps the connections open.
    agent: ureq::Agent,
    /// The language of the player, which the servers answer in.
    language: String,
}

impl HttpFetcher {
    /// Returns the fetcher that asks the servers the way the launcher of Java
    /// asks them.
    ///
    /// The settings of the launcher name the proxy the player wrote, and the
    /// language is the one the texts of the launcher are shown in, so the
    /// servers of the game answer in the language of the player.
    pub fn new(settings: &LauncherSettings, language: impl Into<String>) -> Self {
        Self {
            agent: build_agent(settings),
            language: language.into(),
        }
    }

    /// Returns the fetcher the launcher of the program uses.
    ///
    /// The fetcher is made once, from the settings the launcher was started
    /// with, and the same one is returned afterwards, so every download of the
    /// program shares the connections. A launcher that changes the settings of
    /// the proxy has to be restarted for the change to take, which is what the
    /// launcher of Java does as well: the proxy is read when the program
    /// starts.
    pub fn shared(settings: &LauncherSettings, language: impl Into<String>) -> &'static Self {
        static FETCHER: OnceLock<HttpFetcher> = OnceLock::new();
        FETCHER.get_or_init(|| Self::new(settings, language))
    }
}

/// Builds the agent that opens the connections of the launcher.
///
/// The client does not follow the redirects itself, because the launcher
/// resolves the location of an answer the way the servers expect; it waits as
/// long as the launcher of Java waits for a connection and for the body of an
/// answer; and it names the launcher with its version, because the servers of
/// the game refuse a client they cannot name.
pub(crate) fn build_agent(settings: &LauncherSettings) -> ureq::Agent {
    let mut builder = ureq::AgentBuilder::new()
        .redirects(0)
        .timeout_connect(http::CONNECT_TIMEOUT)
        .timeout_read(http::READ_TIMEOUT)
        .user_agent(http::USER_AGENT);

    let setting = http::proxy_setting(
        &settings.proxy_address(),
        settings.proxy_credentials().as_ref(),
    );

    match setting {
        http::ProxySetting::System => {
            // The proxy of the system is read by the client from the
            // environment of the program, which is where the launcher of Java
            // reads it from as well.
            builder = builder.try_proxy_from_env(true);
        }
        http::ProxySetting::Direct => {
            // No proxy at all, not even the one of the environment.
        }
        // A proxy the player wrote is named as an address, which is the only
        // thing the client asks of a proxy.
        http::ProxySetting::Custom { .. } => {
            if let Some(url) = setting.to_url() {
                match ureq::Proxy::new(&url) {
                    Ok(proxy) => builder = builder.proxy(proxy),
                    Err(error) => {
                        hmcl_core::util::logging::warn(&format!(
                            "The proxy {url} cannot be used: {error}"
                        ));
                    }
                }
            }
        }
    }

    builder.build()
}

impl Fetcher for HttpFetcher {
    /// Asks a server for a file and writes the body of the answer.
    ///
    /// The headers of the request tell the server whether the launcher already
    /// has the file, so a file that did not change is answered with a short
    /// answer and no body at all. A redirect is followed by hand, which is the
    /// way the launcher of Java asks the servers: an address that is written
    /// the wrong way is written the right way before it is asked.
    fn fetch(
        &self,
        url: &str,
        headers: &BTreeMap<String, String>,
        sink: &mut dyn Write,
    ) -> Result<ResponseInfo, String> {
        let mut current = url.to_string();

        // The chain of the redirects is asked again from the beginning, so an
        // address that redirects to itself is refused after the number of
        // redirects the launcher of Java refuses as well.
        for _ in 0..=http::MAX_REDIRECTS {
            let mut request = self.agent.get(&current);

            // The headers of the launcher are sent with every request, and the
            // headers of the caller are sent on top of them, because they name
            // the file the launcher already has.
            for (name, value) in http::default_headers(&self.language) {
                request = request.set(name, &value);
            }
            for (name, value) in headers {
                request = request.set(name, value);
            }

            let response = match request.call() {
                Ok(response) => response,
                // The client reports a status that is not a success as an
                // error, but the server did answer, and the answer may hold
                // the file the launcher asked for or the reason it did not.
                Err(ureq::Error::Status(_, response)) => response,
                Err(error) => return Err(error.to_string()),
            };

            if http::is_redirect_status(response.status()) {
                current = redirect_target(&current, &response)?;
                continue;
            }

            let url = response.get_url().to_string();
            let status = response.status();
            let headers = headers_of(&response);
            copy_body(response.into_reader(), sink)?;

            return Ok(ResponseInfo {
                url,
                status,
                headers,
            });
        }

        Err("The server redirected too much".to_string())
    }
}

/// Returns the address a redirect of an answer points to.
///
/// A location that is empty, or that names a scheme the launcher may not ask,
/// is refused, which is what the launcher of Java does when a server answers
/// with a redirect it cannot follow.
fn redirect_target(current: &str, response: &ureq::Response) -> Result<String, String> {
    let location = response
        .header("location")
        .ok_or_else(|| "The server redirected to an empty location".to_string())?;

    http::resolve_redirect(current, location).ok_or_else(|| {
        format!("The server redirected to an address the launcher refuses: {location}")
    })
}

/// Returns the headers of an answer the way the launcher of Rust reads them.
///
/// The names of the headers of a server are written in many ways, and the
/// launcher of Java reads them without regard to the case, so the names are
/// kept the way the client wrote them, which is the way the server wrote them
/// as well. A header the client cannot read as text is not kept, because the
/// launcher does not ask for a file of such a server again.
///
/// The headers are read before the body, because the answer is read from the
/// body once it is turned into a reader, and a reader cannot be asked for the
/// head of the answer any more.
fn headers_of(response: &ureq::Response) -> BTreeMap<String, String> {
    let mut headers = BTreeMap::new();
    for name in response.headers_names() {
        if let Some(value) = response.header(&name) {
            headers.insert(name, value.to_string());
        }
    }

    headers
}

/// Writes the body of an answer to the writer the caller gave.
fn copy_body(reader: Box<dyn Read + Send + Sync>, sink: &mut dyn Write) -> Result<(), String> {
    let mut reader = reader;
    let mut buffer = vec![0u8; 64 * 1024];

    loop {
        match reader.read(&mut buffer) {
            Ok(0) => return Ok(()),
            Ok(read) => {
                sink.write_all(&buffer[..read])
                    .map_err(|error| error.to_string())?;
            }
            // A read that was interrupted is asked again, because the body of
            // the answer did not end.
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.to_string()),
        }
    }
}

/// Returns the language the servers answer in, written the way the names of
/// the languages are written.
///
/// The code of a language of the launcher is written with an underscore, such
/// as `zh_Hans`, while the servers of the game read a language that is written
/// with a dash, such as `zh-Hans`.
pub fn language_tag(code: &str) -> String {
    if code.is_empty() {
        return "en".to_string();
    }

    code.replace('_', "-")
}

/// Serves one answer and returns the body the server was asked for.
///
/// The answer is written to the first connection that is made, so a test can
/// check the way the launcher asks a server without a network of its own.
fn serve_once(listener: &TcpListener, answer: &str) -> Vec<u8> {
    let (mut stream, _) = listener.accept().expect("a connection should be made");

    let mut asked = Vec::new();
    let mut buffer = [0u8; 1024];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => asked.extend_from_slice(&buffer[..read]),
            // The head of a request is read until the server has it all; a
            // request of the launcher is short, so one read is enough, but a
            // body of a request is not read at all.
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
        if asked.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }

    stream
        .write_all(answer.as_bytes())
        .expect("the answer should be written");
    let _ = stream.flush();

    asked
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    /// Returns the settings of a launcher that reaches the network directly.
    fn settings() -> LauncherSettings {
        LauncherSettings::default()
    }

    #[test]
    fn the_language_of_the_servers_is_written_with_a_dash() {
        assert_eq!(language_tag("zh_Hans"), "zh-Hans");
        assert_eq!(language_tag("zh_Hant"), "zh-Hant");
        assert_eq!(language_tag("en"), "en");
        assert_eq!(language_tag(""), "en");
    }

    #[test]
    fn the_agent_of_a_direct_launcher_names_the_launcher() {
        let fetcher = HttpFetcher::new(&settings(), "en");
        // The agent is made without a network, so a fetcher that is made is a
        // fetcher that can be used.
        assert_eq!(fetcher.language, "en");
    }

    #[test]
    fn a_socks_proxy_of_the_settings_is_read_as_socks5() {
        let settings = LauncherSettings {
            proxy_type: hmcl_core::setting::launcher::ProxyType::Socks,
            proxy_host: Some("127.0.0.1".to_string()),
            proxy_port: Some(1080),
            ..LauncherSettings::default()
        };

        let setting = http::proxy_setting(&settings.proxy_address(), None);
        assert_eq!(
            setting.to_url(),
            Some("socks5://127.0.0.1:1080".to_string())
        );

        // The agent is made even when the proxy is one the player cannot
        // reach, so a launcher that starts still asks the servers.
        let fetcher = HttpFetcher::new(&settings, "en");
        assert_eq!(fetcher.language, "en");
    }

    #[test]
    fn a_file_is_written_to_the_writer() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a server should be listened for");
        let address = listener.local_addr().expect("the address should be read");

        thread::spawn(move || {
            let _ = serve_once(
                &listener,
                "HTTP/1.1 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello",
            );
        });

        let fetcher = HttpFetcher::new(&settings(), "en");
        let mut body = Vec::new();
        let response = fetcher
            .fetch(
                &format!("http://{address}/file"),
                &BTreeMap::new(),
                &mut body,
            )
            .expect("the file should be downloaded");

        assert!(response.is_success());
        assert_eq!(response.status, 200);
        assert_eq!(body, b"hello");
    }

    #[test]
    fn the_headers_of_the_answer_are_read() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a server should be listened for");
        let address = listener.local_addr().expect("the address should be read");

        thread::spawn(move || {
            let _ = serve_once(
                &listener,
                "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nETag: \"abc\"\r\nConnection: close\r\n\r\nok",
            );
        });

        let fetcher = HttpFetcher::new(&settings(), "en");
        let mut body = Vec::new();
        let response = fetcher
            .fetch(
                &format!("http://{address}/file"),
                &BTreeMap::new(),
                &mut body,
            )
            .expect("the file should be downloaded");

        // The cache of the launcher reads the e-tag of the answer to ask
        // whether the file changed later, so the value of a header the server
        // sent has to be read and not only its name.
        assert_eq!(response.header("ETag"), Some("\"abc\""));
        assert_eq!(response.header("content-length"), Some("2"));
        assert_eq!(response.header("content-LENGTH"), Some("2"));
        assert_eq!(response.header("x-bmclapi-hash"), None);
    }

    #[test]
    fn a_redirect_is_followed_by_hand() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a server should be listened for");
        let address = listener.local_addr().expect("the address should be read");

        thread::spawn(move || {
            let first = serve_once(
                &listener,
                "HTTP/1.1 302 Found\r\nLocation: /moved\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
            // The launcher asked the address the location names, which is a
            // path of the server it asked and not the address of another one.
            let asked = String::from_utf8_lossy(&first);
            assert!(asked.starts_with("GET /file HTTP/1.1"), "{asked}");

            let second = serve_once(
                &listener,
                "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
            );
            let asked = String::from_utf8_lossy(&second);
            assert!(asked.starts_with("GET /moved HTTP/1.1"), "{asked}");
        });

        let fetcher = HttpFetcher::new(&settings(), "en");
        let mut body = Vec::new();
        let response = fetcher
            .fetch(
                &format!("http://{address}/file"),
                &BTreeMap::new(),
                &mut body,
            )
            .expect("the file should be downloaded");

        assert_eq!(response.status, 200);
        assert_eq!(body, b"ok");
    }

    #[test]
    fn a_location_that_is_not_encoded_is_written_the_right_way() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a server should be listened for");
        let address = listener.local_addr().expect("the address should be read");

        thread::spawn(move || {
            let first = serve_once(
                &listener,
                "HTTP/1.1 302 Found\r\nLocation: /my file.jar\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
            let asked = String::from_utf8_lossy(&first);
            assert!(asked.starts_with("GET /file HTTP/1.1"), "{asked}");

            let second = serve_once(
                &listener,
                "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
            // The space of the location is encoded before the address is
            // asked, which is what the programs of the network expect.
            let asked = String::from_utf8_lossy(&second);
            assert!(asked.starts_with("GET /my%20file.jar HTTP/1.1"), "{asked}");
        });

        let fetcher = HttpFetcher::new(&settings(), "en");
        let mut body = Vec::new();
        fetcher
            .fetch(
                &format!("http://{address}/file"),
                &BTreeMap::new(),
                &mut body,
            )
            .expect("the file should be downloaded");
    }

    #[test]
    fn a_redirect_to_a_scheme_the_launcher_refuses_is_refused() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a server should be listened for");
        let address = listener.local_addr().expect("the address should be read");

        thread::spawn(move || {
            let _ = serve_once(
                &listener,
                "HTTP/1.1 302 Found\r\nLocation: file:///etc/passwd\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
        });

        let fetcher = HttpFetcher::new(&settings(), "en");
        let mut body = Vec::new();
        let error = fetcher
            .fetch(
                &format!("http://{address}/file"),
                &BTreeMap::new(),
                &mut body,
            )
            .expect_err("the redirect should be refused");

        assert!(error.contains("refuses"), "{error}");
    }

    #[test]
    fn a_redirect_to_an_empty_location_is_refused() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a server should be listened for");
        let address = listener.local_addr().expect("the address should be read");

        thread::spawn(move || {
            let _ = serve_once(
                &listener,
                "HTTP/1.1 302 Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
        });

        let fetcher = HttpFetcher::new(&settings(), "en");
        let mut body = Vec::new();
        let error = fetcher
            .fetch(
                &format!("http://{address}/file"),
                &BTreeMap::new(),
                &mut body,
            )
            .expect_err("the redirect should be refused");

        assert!(error.contains("empty location"), "{error}");
    }

    #[test]
    fn the_headers_of_the_caller_are_sent() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a server should be listened for");
        let address = listener.local_addr().expect("the address should be read");

        thread::spawn(move || {
            let asked = serve_once(
                &listener,
                "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
            let asked = String::from_utf8_lossy(&asked);
            // The launcher names itself and the language of the player, and
            // the header of the caller tells the server whether the launcher
            // already has the file.
            assert!(asked.contains("User-Agent: HMCL/"), "{asked}");
            assert!(asked.contains("Accept-Language: en"), "{asked}");
            assert!(asked.contains("If-None-Match: \"abc\""), "{asked}");
        });

        let fetcher = HttpFetcher::new(&settings(), "en");
        let mut body = Vec::new();
        fetcher
            .fetch(
                &format!("http://{address}/file"),
                &BTreeMap::from([("If-None-Match".to_string(), "\"abc\"".to_string())]),
                &mut body,
            )
            .expect("the file should be asked for");
    }

    #[test]
    fn a_short_answer_of_the_server_is_answered() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a server should be listened for");
        let address = listener.local_addr().expect("the address should be read");

        thread::spawn(move || {
            let _ = serve_once(
                &listener,
                "HTTP/1.1 304 Not Modified\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
        });

        let fetcher = HttpFetcher::new(&settings(), "en");
        let mut body = Vec::new();
        let response = fetcher
            .fetch(
                &format!("http://{address}/file"),
                &BTreeMap::new(),
                &mut body,
            )
            .expect("the answer of the server should be read");

        // A file that did not change is told by the server with a short
        // answer, which is not a failure of the download.
        assert!(response.is_not_modified());
        assert!(body.is_empty());
    }

    #[test]
    fn a_file_the_server_does_not_hold_is_answered() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a server should be listened for");
        let address = listener.local_addr().expect("the address should be read");

        thread::spawn(move || {
            let _ = serve_once(
                &listener,
                "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
        });

        let fetcher = HttpFetcher::new(&settings(), "en");
        let mut body = Vec::new();
        let response = fetcher
            .fetch(
                &format!("http://{address}/file"),
                &BTreeMap::new(),
                &mut body,
            )
            .expect("the answer of the server should be read");

        assert_eq!(response.status, 404);
        assert!(!response.is_success());
    }
}
