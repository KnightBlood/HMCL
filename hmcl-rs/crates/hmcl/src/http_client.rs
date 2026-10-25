//! The client the login of an account sends its requests with.
//!
//! The core of the launcher writes the requests of a login and reads the
//! answers, but it opens no connection of its own: it asks a
//! [`HttpClient`] of the program that runs it. This module is that client for
//! the desktop launcher, and it keeps the behaviour of the launcher of Java:
//!
//! * the headers the launcher of Java sends with every request are sent, so
//!   the servers answer in the language of the player;
//! * a redirect is followed by hand with the rules of
//!   [`hmcl_core::util::http`], because some servers answer with a `Location`
//!   that is not encoded and the programs of the network refuse such an
//!   address;
//! * a status that is not a success is returned as an answer rather than as a
//!   failure, because the login tells the reason of the server from the body of
//!   the answer.
//!
//! The agent of the connections is the one the downloads of the launcher use,
//! so a player who logs in does not make a second connection to the servers.

use hmcl_core::setting::launcher::LauncherSettings;
use hmcl_core::util::http;
use hmcl_core::util::http_request::{HttpClient, HttpRequest, HttpResponse};

/// The client the launcher sends the requests of a login with.
pub struct UreqHttpClient {
    /// The agent that keeps the connections open.
    agent: ureq::Agent,
    /// The language of the player, which the servers answer in.
    language: String,
}

impl UreqHttpClient {
    /// Creates a client that sends the requests the way the launcher of Java
    /// sends them.
    ///
    /// The settings name the proxy of the player, and the language is the one
    /// the texts of the launcher are shown in.
    pub fn new(settings: &LauncherSettings, language: impl Into<String>) -> Self {
        Self {
            agent: super::http_fetcher::build_agent(settings),
            language: language.into(),
        }
    }

    /// Sends the request once and reads the answer.
    ///
    /// The headers of the launcher are sent before the headers of the request,
    /// so a request that names a language of its own is answered in that
    /// language. The headers of the answer are kept with the answer, because a
    /// server of the authlib-injector names its real address in one of them.
    fn send_once(&self, url: &str, request: &HttpRequest) -> Result<HttpResponse, String> {
        let mut sent = self.agent.request(request.method().as_str(), url);

        for (name, value) in http::default_headers(&self.language) {
            sent = sent.set(name, &value);
        }
        for (name, value) in request.headers() {
            sent = sent.set(name, value);
        }

        let response = match request.body() {
            Some(body) => sent.send_bytes(body),
            None => sent.call(),
        };

        // The client reports a status that is not a success as an error, but
        // the server did answer, and the body of the answer is the reason it
        // refused the request.
        let response = match response {
            Ok(response) => response,
            Err(ureq::Error::Status(_, response)) => response,
            Err(error) => return Err(error.to_string()),
        };

        let url = response.get_url().to_string();
        let status = response.status();
        // The headers are read before the body, because the answer is read
        // from the body once it is turned into a reader.
        let headers = response
            .headers_names()
            .into_iter()
            .filter_map(|name| {
                let value = response.header(&name)?.to_string();
                Some((name, value))
            })
            .collect();
        let body = read_text(response.into_reader())?;

        Ok(HttpResponse {
            url,
            status,
            body,
            headers,
        })
    }
}

impl HttpClient for UreqHttpClient {
    /// Sends a request and reads the answer of the server.
    ///
    /// The redirects are followed by hand, up to
    /// [`http::MAX_REDIRECTS`], and the address of the answer is the address of
    /// the last request of the chain. A redirect to a scheme the launcher may
    /// not ask, such as a file of the machine of the player, is a failure.
    fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, String> {
        let mut current = request.url().to_string();

        for _ in 0..=http::MAX_REDIRECTS {
            let response = self.send_once(&current, request)?;

            if !http::is_redirect_status(response.status) {
                return Ok(response);
            }

            let location = response.header("location").map(str::to_string);
            let location =
                location.ok_or_else(|| format!("The server redirected {current} to no address"))?;
            current = http::resolve_redirect(&current, &location).ok_or_else(|| {
                format!("The server redirected to an address the launcher refuses: {location}")
            })?;
        }

        Err(format!("The server redirected {current} too often"))
    }
}

/// Reads the body of an answer as text.
///
/// The servers of the accounts answer in UTF-8, so a body that is not valid
/// UTF-8 is read with the replacement character rather than refused: the login
/// only needs the text of the reason of a refusal.
fn read_text(reader: Box<dyn std::io::Read + Send + Sync>) -> Result<String, String> {
    let mut reader = reader;
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut reader, &mut bytes).map_err(|error| error.to_string())?;

    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Returns the method of a request the way a test writes it.
#[cfg(test)]
fn method_of(asked: &str) -> &str {
    asked.split_whitespace().next().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;
    use std::thread;

    /// Reads one request of the client and answers it.
    ///
    /// The head of the request is read first, because the body is sent after
    /// the head and the length of the body is named in it. The answer is
    /// written once the whole request is read, so a client that waits for the
    /// answer of a body it did not finish writing cannot deadlock the test.
    fn serve(listener: &TcpListener, answer: &str) -> Vec<u8> {
        let (mut stream, _) = listener.accept().expect("a connection should be made");

        let mut asked = Vec::new();
        let mut buffer = [0u8; 4096];
        loop {
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => asked.extend_from_slice(&buffer[..read]),
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }

            let Some(head_end) = position(&asked, b"\r\n\r\n") else {
                continue;
            };
            let head = String::from_utf8_lossy(&asked[..head_end]).to_string();
            let length = head
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    if name.eq_ignore_ascii_case("content-length") {
                        value.trim().parse::<usize>().ok()
                    } else {
                        None
                    }
                })
                .unwrap_or(0);

            if asked.len() >= head_end + 4 + length {
                break;
            }
        }

        stream
            .write_all(answer.as_bytes())
            .expect("the answer should be written");
        let _ = stream.flush();

        asked
    }

    /// Returns the position of a sequence inside another one.
    fn position(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }

    /// Returns the settings of a launcher that reaches the network directly.
    fn settings() -> LauncherSettings {
        LauncherSettings::default()
    }

    #[test]
    fn a_body_of_a_login_is_sent_with_the_method_of_the_request() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a server should be listened for");
        let address = listener.local_addr().expect("the address should be read");

        thread::spawn(move || {
            let asked = serve(
                &listener,
                "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
            );
            let asked = String::from_utf8_lossy(&asked).to_string();

            assert_eq!(method_of(&asked), "POST");
            assert!(
                asked.contains("Content-Type: application/json; charset=utf-8"),
                "{asked}"
            );
            assert!(asked.contains("User-Agent: HMCL/"), "{asked}");
            assert!(asked.ends_with(r#"{"a":1}"#), "{asked}");
        });

        let client = UreqHttpClient::new(&settings(), "en");
        let request = HttpRequest::post(format!("http://{address}/token")).json_text(r#"{"a":1}"#);

        let response = client
            .execute(&request)
            .expect("the request should be sent");
        assert_eq!(response.status, 200);
        assert_eq!(response.body, "ok");
        assert!(response.url.starts_with("http://127.0.0.1:"));
    }

    #[test]
    fn the_headers_of_an_answer_are_kept_with_it() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a server should be listened for");
        let address = listener.local_addr().expect("the address should be read");

        thread::spawn(move || {
            serve(
                &listener,
                "HTTP/1.1 200 OK\r\nX-Authlib-Injector-API-Location: https://example.com/api/yggdrasil/\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
            );
        });

        let client = UreqHttpClient::new(&settings(), "en");
        let response = client
            .execute(&HttpRequest::get(format!("http://{address}/")))
            .expect("the request should be sent");

        // The name of the header is read without regard to case, because the
        // server writes it in its own case.
        assert_eq!(
            response.header("x-authlib-injector-api-location"),
            Some("https://example.com/api/yggdrasil/")
        );
    }

    #[test]
    fn a_status_that_is_not_a_success_is_returned_with_its_body() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a server should be listened for");
        let address = listener.local_addr().expect("the address should be read");

        thread::spawn(move || {
            serve(
                &listener,
                "HTTP/1.1 400 Bad Request\r\nContent-Length: 25\r\nConnection: close\r\n\r\n{\"error\":\"invalid_grant\"}\r\n",
            );
        });

        let client = UreqHttpClient::new(&settings(), "en");
        let response = client
            .execute(&HttpRequest::post(format!("http://{address}/token")))
            .expect("the answer should be read");

        // The login tells the reason of the refusal from the body, so a status
        // that is not a success is an answer and not a failure.
        assert_eq!(response.status, 400);
        assert!(response.body.contains("invalid_grant"), "{}", response.body);
    }

    #[test]
    fn a_redirect_of_the_servers_is_followed() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a server should be listened for");
        let address = listener.local_addr().expect("the address should be read");

        thread::spawn(move || {
            let asked = serve(
                &listener,
                "HTTP/1.1 302 Found\r\nLocation: /moved\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
            assert_eq!(method_of(&String::from_utf8_lossy(&asked)), "GET");

            let asked = serve(
                &listener,
                "HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok",
            );
            let asked = String::from_utf8_lossy(&asked);
            assert!(asked.starts_with("GET /moved HTTP/1.1"), "{asked}");
        });

        let client = UreqHttpClient::new(&settings(), "en");
        let response = client
            .execute(&HttpRequest::get(format!("http://{address}/login")))
            .expect("the redirect should be followed");

        assert_eq!(response.status, 200);
        assert!(response.url.ends_with("/moved"), "{}", response.url);
    }

    #[test]
    fn a_redirect_to_a_file_of_the_machine_is_refused() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a server should be listened for");
        let address = listener.local_addr().expect("the address should be read");

        thread::spawn(move || {
            serve(
                &listener,
                "HTTP/1.1 302 Found\r\nLocation: file:///etc/passwd\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
        });

        let client = UreqHttpClient::new(&settings(), "en");
        let error = client
            .execute(&HttpRequest::get(format!("http://{address}/login")))
            .expect_err("the redirect should be refused");

        assert!(error.contains("refuses"), "{error}");
    }

    #[test]
    fn the_method_of_a_request_is_written_the_way_the_server_reads_it() {
        use hmcl_core::util::http_request::HttpMethod;

        assert_eq!(HttpMethod::Get.as_str(), "GET");
        assert_eq!(HttpMethod::Post.as_str(), "POST");
        assert_eq!(HttpMethod::Put.as_str(), "PUT");
        assert_eq!(HttpMethod::Delete.as_str(), "DELETE");
    }
}
