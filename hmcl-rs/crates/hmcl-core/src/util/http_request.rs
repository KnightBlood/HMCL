//! The requests the launcher sends to the servers of the accounts.
//!
//! The download of a file is a `GET` of an address that carries no body, so the
//! tasks of [`crate::download`] ask a server through the small
//! [`crate::download::Fetcher`] trait. The login of an account is not like
//! that: it posts a form or a JSON body to the server of the accounts, reads a
//! JSON answer, and tells a status that is not a success from a body that says
//! what went wrong. This module holds that request and that answer, and the
//! order of the fields of a form, so the login behaves the way the launcher of
//! Java behaves without the core of the launcher opening a connection.
//!
//! The program that runs the launcher names a [`HttpClient`]; the core only
//! writes the request and reads the answer it gets.

use std::fmt;

use serde::de::DeserializeOwned;

/// The method of a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    /// Asks a server for a body.
    Get,
    /// Sends a body to a server and asks it to keep the change.
    Post,
    /// Sends a body to a server, replacing what the server holds.
    Put,
    /// Asks a server to remove what it holds.
    Delete,
}

impl HttpMethod {
    /// Returns the method the way a request writes it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
        }
    }
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// The type of a JSON body.
pub const CONTENT_TYPE_JSON: &str = "application/json; charset=utf-8";

/// The type of a body that holds the fields of a form.
pub const CONTENT_TYPE_FORM: &str = "application/x-www-form-urlencoded; charset=utf-8";

/// A request to a server.
///
/// The request is written by the core of the launcher and sent by the program
/// that runs it. The headers of the launcher, such as the user agent and the
/// language of the player, are added by the client and not by this request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    /// The address the request is sent to.
    url: String,
    /// The method of the request.
    method: HttpMethod,
    /// The headers of the request, in the order they are written.
    headers: Vec<(String, String)>,
    /// The body of the request, if the request carries one.
    body: Option<Vec<u8>>,
    /// Whether a status that is not a success is read as an answer.
    ignore_status: bool,
    /// The statuses that are read as an answer even though they are not a
    /// success.
    tolerated_statuses: Vec<u16>,
    /// The number of the attempts the request makes.
    attempts: u32,
}

impl HttpRequest {
    /// Creates a request without headers and without a body.
    pub fn new(url: impl Into<String>, method: HttpMethod) -> Self {
        Self {
            url: url.into(),
            method,
            headers: Vec::new(),
            body: None,
            ignore_status: false,
            tolerated_statuses: Vec::new(),
            attempts: 1,
        }
    }

    /// Creates a request that asks a server for a body.
    pub fn get(url: impl Into<String>) -> Self {
        Self::new(url, HttpMethod::Get)
    }

    /// Creates a request that sends a body to a server.
    pub fn post(url: impl Into<String>) -> Self {
        Self::new(url, HttpMethod::Post)
    }

    /// Adds a header, replacing the value of a header of the same name.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        let name = name.into();
        match self
            .headers
            .iter_mut()
            .find(|(existing, _)| existing.eq_ignore_ascii_case(&name))
        {
            Some((_, existing)) => *existing = value.into(),
            None => self.headers.push((name, value.into())),
        }
        self
    }

    /// Tells the server the type of the answer the launcher reads.
    pub fn accept(self, content_type: impl Into<String>) -> Self {
        self.header("Accept", content_type)
    }

    /// Names the token the request proves itself with.
    ///
    /// The value is written the way the server expects it, such as
    /// `Bearer <token>` for the servers of the game.
    pub fn authorization(self, value: impl Into<String>) -> Self {
        self.header("Authorization", value)
    }

    /// Sends a body of JSON.
    ///
    /// The value is written the way the launcher of Java writes it, which is
    /// the form of the fields of the value in the order the type declares
    /// them.
    pub fn json(self, value: &impl serde::Serialize) -> Self {
        self.raw_body(
            serde_json::to_vec(value).unwrap_or_default(),
            CONTENT_TYPE_JSON,
        )
    }

    /// Sends a body of JSON that is already written.
    pub fn json_text(self, text: impl Into<String>) -> Self {
        self.raw_body(text.into().into_bytes(), CONTENT_TYPE_JSON)
    }

    /// Sends the fields of a form.
    ///
    /// The fields are written in the order they are given, and a field without
    /// a value is left out, which is the way the launcher of Java writes a form.
    pub fn form(self, fields: &[(&str, Option<&str>)]) -> Self {
        self.raw_body(form_body(fields).into_bytes(), CONTENT_TYPE_FORM)
    }

    /// Sends a body with a type the caller names.
    pub fn raw_body(mut self, body: Vec<u8>, content_type: &str) -> Self {
        self.body = Some(body);
        let length = self.body.as_ref().map_or(0, Vec::len).to_string();
        self.headers.retain(|(name, _)| {
            !name.eq_ignore_ascii_case("Content-Type")
                && !name.eq_ignore_ascii_case("Content-Length")
        });
        self.headers
            .push(("Content-Type".to_string(), content_type.to_string()));
        self.headers.push(("Content-Length".to_string(), length));
        self
    }

    /// Reads a status that is not a success as an answer.
    ///
    /// The servers of the accounts answer a refused login with the status `400`
    /// and a body that says why, so the body is read and the login reports the
    /// reason of the server instead of a bare status.
    pub fn ignore_status(mut self) -> Self {
        self.ignore_status = true;
        self
    }

    /// Reads one status that is not a success as an answer.
    ///
    /// The server of the tokens of Xbox answers a child account without a
    /// family with the status `401` and a body that names the reason; the other
    /// statuses are still failures.
    pub fn tolerate_status(mut self, status: u16) -> Self {
        self.tolerated_statuses.push(status);
        self
    }

    /// Sets the number of the attempts the request makes.
    ///
    /// The number is the count of the attempts and not the count of the
    /// retries, which is the way the launcher of Java counts them: a request of
    /// five attempts is sent at most five times.
    pub fn attempts(mut self, attempts: u32) -> Self {
        self.attempts = attempts.max(1);
        self
    }

    /// Returns the address the request is sent to.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Returns the method of the request.
    pub const fn method(&self) -> HttpMethod {
        self.method
    }

    /// Returns the headers of the request, in the order they are written.
    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }

    /// Returns the body of the request.
    pub fn body(&self) -> Option<&[u8]> {
        self.body.as_deref()
    }

    /// Returns whether a status that is not a success is read as an answer.
    pub const fn ignores_status(&self) -> bool {
        self.ignore_status
    }

    /// Returns whether a status is read as an answer even though it is not a
    /// success.
    pub fn tolerates_status(&self, status: u16) -> bool {
        self.tolerated_statuses.contains(&status)
    }

    /// Returns the number of the attempts the request makes.
    pub const fn attempt_count(&self) -> u32 {
        self.attempts
    }
}

/// The answer of a server.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HttpResponse {
    /// The address the answer came from, after the redirects.
    pub url: String,
    /// The code of the answer, such as `200` or `400`.
    pub status: u16,
    /// The body of the answer, read as text.
    pub body: String,
    /// The headers of the answer, with the names as the server wrote them.
    ///
    /// Most requests of the launcher read the body only; the headers are kept
    /// because a server of the authlib-injector names its real address in one
    /// of them, and the launcher has to ask that address instead.
    pub headers: Vec<(String, String)>,
}

impl HttpResponse {
    /// Creates an answer.
    pub fn new(status: u16, body: impl Into<String>) -> Self {
        Self {
            url: String::new(),
            status,
            body: body.into(),
            headers: Vec::new(),
        }
    }

    /// Returns whether the server answered with a success.
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// Returns the value of a header of the answer.
    ///
    /// The name is compared without regard to case, which is the way a server
    /// may write it.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header, _)| header.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// Returns the answer with a header the server wrote.
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Returns the answer with the address it came from.
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = url.into();
        self
    }
}

/// The way the program of the launcher sends a request.
///
/// The implementation adds the headers of the launcher, follows the redirects
/// the way the launcher of Java follows them, and returns the body of the
/// answer even when the status is not a success, so the login can report the
/// reason of the server. Only a failure of the connection is an error.
pub trait HttpClient {
    /// Sends a request and reads the answer.
    fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, String>;
}

/// The failure of a request of the launcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestFailure {
    /// The connection broke or the server could not be reached.
    Network {
        /// The address the launcher asked.
        url: String,
        /// The message the client wrote.
        message: String,
    },
    /// The server answered with a status that is not a success.
    Status {
        /// The address the launcher asked.
        url: String,
        /// The code of the answer.
        status: u16,
        /// The body of the answer, when the server sent one.
        body: Option<String>,
    },
}

impl RequestFailure {
    /// Returns the code of the answer, when the server answered at all.
    pub const fn status(&self) -> Option<u16> {
        match self {
            Self::Network { .. } => None,
            Self::Status { status, .. } => Some(*status),
        }
    }
}

impl fmt::Display for RequestFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network { url, message } => write!(formatter, "{url}: {message}"),
            Self::Status { url, status, .. } => {
                write!(
                    formatter,
                    "{url}: the server answered with the status {status}"
                )
            }
        }
    }
}

impl std::error::Error for RequestFailure {}

/// Sends a request and returns the body of the answer.
///
/// The request is sent again when the connection breaks, up to the number of
/// the attempts the request names; a status that is not a success is reported
/// at once, because a server that refused the request answers the same way
/// when it is asked again. The launcher of Java retries that status as well,
/// which only costs the time of the attempts.
pub fn send(
    client: &dyn HttpClient,
    request: &HttpRequest,
) -> Result<HttpResponse, RequestFailure> {
    let mut failure = None;

    for _ in 0..request.attempt_count() {
        match client.execute(request) {
            Ok(response)
                if response.is_success()
                    || request.ignores_status()
                    || request.tolerates_status(response.status) =>
            {
                return Ok(response);
            }
            Ok(response) => {
                // A refusal is reported at once: the server answered, so asking
                // again costs the time of the attempts without changing the
                // answer.
                return Err(RequestFailure::Status {
                    url: request.url().to_string(),
                    status: response.status,
                    body: if response.body.trim().is_empty() {
                        None
                    } else {
                        Some(response.body)
                    },
                });
            }
            Err(message) => {
                failure = Some(RequestFailure::Network {
                    url: request.url().to_string(),
                    message,
                });
            }
        }
    }

    Err(failure.unwrap_or_else(|| RequestFailure::Network {
        url: request.url().to_string(),
        message: "the request was not sent".to_string(),
    }))
}

/// Sends a request and reads the body of the answer as a value.
///
/// The body is read as the value even when the status is not a success, which
/// is what a login needs: the server of the accounts names the reason of a
/// refused login in the body.
pub fn send_json<T: DeserializeOwned>(
    client: &dyn HttpClient,
    request: &HttpRequest,
) -> Result<T, RequestFailure> {
    let response = send(client, request)?;
    serde_json::from_str(&response.body).map_err(|error| RequestFailure::Network {
        url: request.url().to_string(),
        message: format!("the answer is not the JSON the launcher reads: {error}"),
    })
}

/// Returns the text of a field of a form, written the way Java writes it.
///
/// The letters and the digits are written as they are, and so are the four
/// characters `.`, `-`, `*` and `_`; a space becomes a `+`; everything else,
/// and every character that is not ASCII, becomes the percent-encoded form of
/// its bytes, with the digits in upper case. This is the encoding of
/// `URLEncoder.encode(text, UTF_8)`, which the launcher of Java writes its forms
/// with.
pub fn encode_form_component(text: &str) -> String {
    let mut encoded = String::with_capacity(text.len());

    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' | b'*' | b'_' => {
                encoded.push(char::from(byte));
            }
            b' ' => encoded.push('+'),
            byte => {
                encoded.push('%');
                encoded.push(char::from(HEX[(byte >> 4) as usize]));
                encoded.push(char::from(HEX[(byte & 0x0f) as usize]));
            }
        }
    }

    encoded
}

/// Returns the text of a form component, decoded.
///
/// This is the reading of `URLDecoder.decode(text, UTF_8)`: a plus stands for a
/// space, `%XX` stands for the byte it names, and the bytes of a character of
/// another language are written one after the other and are read as one
/// character. A `%` that does not name a byte is kept as it is, so the text is
/// decoded as far as it can be instead of the whole value being lost.
pub fn decode_form_component(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'+' => {
                decoded.push(b' ');
                index += 1;
            }
            b'%' => {
                let high = bytes.get(index + 1).and_then(|byte| hex_value(*byte));
                let low = bytes.get(index + 2).and_then(|byte| hex_value(*byte));
                match (high, low) {
                    (Some(high), Some(low)) => {
                        decoded.push(high * 16 + low);
                        index += 3;
                    }
                    _ => {
                        decoded.push(b'%');
                        index += 1;
                    }
                }
            }
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }

    // The text of a form is written in UTF-8, so a sequence of bytes that is
    // not one is shown the way it is instead of the whole value being lost.
    String::from_utf8_lossy(&decoded).into_owned()
}

/// Returns the value of a digit of the hexadecimal form of a byte.
fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// The digits the percent-encoded form is written with.
const HEX: &[u8; 16] = b"0123456789ABCDEF";

/// Returns the body of a form.
///
/// A field without a value is left out, which is the way the launcher of Java
/// writes a form: a value the launcher does not know is not sent as an empty
/// one.
pub fn form_body(fields: &[(&str, Option<&str>)]) -> String {
    let mut body = String::new();
    for (name, value) in fields {
        let Some(value) = value else {
            continue;
        };
        if !body.is_empty() {
            body.push('&');
        }
        body.push_str(&encode_form_component(name));
        body.push('=');
        body.push_str(&encode_form_component(value));
    }
    body
}

/// Returns an address with the fields of a query.
///
/// The fields are written in the order they are given and a field without a
/// value is left out, which is the way `NetworkUtils.withQuery` writes the query
/// of the address of the authorization page.
pub fn with_query(base: &str, fields: &[(&str, Option<&str>)]) -> String {
    let query = form_body(fields);
    if query.is_empty() {
        return base.to_string();
    }

    let mut url = String::with_capacity(base.len() + query.len() + 1);
    url.push_str(base);
    if !base.is_empty() {
        // A base that already holds a query keeps its own fields, which is the
        // case of the address of a server of the game.
        url.push(if base.contains('?') { '&' } else { '?' });
    }
    url.push_str(&query);
    url
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// A client that answers with the answers it is given and keeps the
    /// requests it was asked.
    struct FakeClient {
        /// The answers, in the order they are returned.
        answers: RefCell<Vec<Result<HttpResponse, String>>>,
        /// The requests the client was asked, in the order they were asked.
        asked: RefCell<Vec<HttpRequest>>,
    }

    impl FakeClient {
        fn new(answers: Vec<Result<HttpResponse, String>>) -> Self {
            Self {
                answers: RefCell::new(answers),
                asked: RefCell::new(Vec::new()),
            }
        }
    }

    impl HttpClient for FakeClient {
        fn execute(&self, request: &HttpRequest) -> Result<HttpResponse, String> {
            self.asked.borrow_mut().push(request.clone());
            let mut answers = self.answers.borrow_mut();
            if answers.is_empty() {
                return Err("the client was asked too often".to_string());
            }
            answers.remove(0)
        }
    }

    #[test]
    fn the_fields_of_a_form_are_written_the_way_java_writes_them() {
        // The encoding of `URLEncoder.encode(text, UTF_8)`: a space becomes a
        // plus, the four characters of the alphabet are kept, and the rest is
        // percent-encoded with upper-case digits.
        assert_eq!(
            encode_form_component("XboxLive.signin offline_access"),
            "XboxLive.signin+offline_access"
        );
        assert_eq!(encode_form_component("a-b_c*d.e"), "a-b_c*d.e");
        assert_eq!(encode_form_component("a+b"), "a%2Bb");
        assert_eq!(encode_form_component("a=b&c"), "a%3Db%26c");
        assert_eq!(encode_form_component("é"), "%C3%A9");
        assert_eq!(encode_form_component("~"), "%7E");
        assert_eq!(encode_form_component(""), "");
    }

    #[test]
    fn a_form_is_written_in_the_order_of_the_fields_and_skips_the_empty_ones() {
        let body = form_body(&[
            ("client_id", Some("id")),
            ("code", None),
            ("grant_type", Some("authorization_code")),
        ]);

        assert_eq!(body, "client_id=id&grant_type=authorization_code");
        assert_eq!(form_body(&[]), "");
    }

    #[test]
    fn the_query_of_an_address_keeps_the_fields_of_the_address() {
        assert_eq!(
            with_query(
                "https://example.com/a",
                &[("scope", Some("a b")), ("state", Some("x"))]
            ),
            "https://example.com/a?scope=a+b&state=x"
        );
        assert_eq!(
            with_query("https://example.com/a?b=c", &[("d", Some("e"))]),
            "https://example.com/a?b=c&d=e"
        );
        assert_eq!(
            with_query("https://example.com/a", &[("b", None)]),
            "https://example.com/a"
        );
    }

    #[test]
    fn a_json_body_names_its_type_and_its_length() {
        let request = HttpRequest::post("https://example.com/a").json(&serde_json::json!({"a": 1}));

        assert_eq!(request.body(), Some(&b"{\"a\":1}"[..]));
        assert_eq!(
            request
                .headers()
                .iter()
                .find(|(name, _)| name == "Content-Type")
                .map(|(_, value)| value.as_str()),
            Some(CONTENT_TYPE_JSON)
        );
        assert_eq!(
            request
                .headers()
                .iter()
                .find(|(name, _)| name == "Content-Length")
                .map(|(_, value)| value.as_str()),
            Some("7")
        );
    }

    #[test]
    fn a_header_is_written_once() {
        let request = HttpRequest::get("https://example.com/a")
            .header("Accept", "text/plain")
            .accept("application/json");

        assert_eq!(
            request.headers(),
            &[("Accept".to_string(), "application/json".to_string())]
        );
    }

    #[test]
    fn a_status_that_is_not_a_success_is_a_failure() {
        let client = FakeClient::new(vec![Ok(HttpResponse::new(404, "gone"))]);
        let failure = send(&client, &HttpRequest::get("https://example.com/a"))
            .expect_err("the status should be a failure");

        assert_eq!(failure.status(), Some(404));
        assert!(failure.to_string().contains("404"));
    }

    #[test]
    fn a_refused_login_is_read_as_an_answer() {
        let client = FakeClient::new(vec![Ok(HttpResponse::new(400, "{\"error\":\"invalid\"}"))]);
        let response = send(
            &client,
            &HttpRequest::post("https://example.com/a").ignore_status(),
        )
        .expect("the body should be read");

        assert_eq!(response.status, 400);
        assert_eq!(response.body, "{\"error\":\"invalid\"}");
    }

    #[test]
    fn one_status_is_read_as_an_answer() {
        let client = FakeClient::new(vec![Ok(HttpResponse::new(401, "{\"XErr\":2148916238}"))]);
        let response = send(
            &client,
            &HttpRequest::post("https://example.com/a").tolerate_status(401),
        )
        .expect("the body should be read");

        assert_eq!(response.status, 401);
    }

    #[test]
    fn a_connection_that_breaks_is_asked_again() {
        let client = FakeClient::new(vec![
            Err("the connection broke".to_string()),
            Ok(HttpResponse::new(200, "ok")),
        ]);
        let response = send(
            &client,
            &HttpRequest::get("https://example.com/a").attempts(5),
        )
        .expect("the request should be asked again");

        assert_eq!(response.body, "ok");
        assert_eq!(client.asked.borrow().len(), 2);
    }

    #[test]
    fn every_attempt_is_made_before_the_failure_is_reported() {
        let client = FakeClient::new(vec![
            Err("the connection broke".to_string()),
            Err("the connection broke".to_string()),
            Err("the connection broke".to_string()),
        ]);
        let failure = send(
            &client,
            &HttpRequest::get("https://example.com/a").attempts(3),
        )
        .expect_err("the failure should be reported");

        assert_eq!(client.asked.borrow().len(), 3);
        assert_eq!(failure.status(), None);
    }

    #[test]
    fn the_attempts_are_counted_as_the_launcher_of_java_counts_them() {
        // A request of one attempt is sent once, which is the default.
        let client = FakeClient::new(vec![Err("the connection broke".to_string())]);
        let _ = send(&client, &HttpRequest::get("https://example.com/a"));
        assert_eq!(client.asked.borrow().len(), 1);

        // A number below one is read as one attempt, so a request is sent.
        assert_eq!(
            HttpRequest::get("https://example.com/a")
                .attempts(0)
                .attempt_count(),
            1
        );
    }

    #[test]
    fn an_answer_is_read_as_a_value() {
        let client = FakeClient::new(vec![Ok(HttpResponse::new(200, "{\"a\":1}"))]);
        let value: serde_json::Value =
            send_json(&client, &HttpRequest::get("https://example.com/a"))
                .expect("the value should be read");

        assert_eq!(value["a"], 1);
    }

    #[test]
    fn the_headers_of_an_answer_are_read_without_regard_to_case() {
        let response = HttpResponse::new(200, "")
            .with_header(
                "X-Authlib-Injector-Api-Location",
                "https://example.com/api/yggdrasil/",
            )
            .with_url("https://example.com/");

        assert_eq!(
            response.header("x-authlib-injector-api-location"),
            Some("https://example.com/api/yggdrasil/")
        );
        assert_eq!(response.header("Location"), None);
        assert_eq!(response.url, "https://example.com/");
        // The answer a client reads with no header keeps an empty list, which is
        // what the requests of the login expect.
        assert!(HttpResponse::new(200, "ok").headers.is_empty());
    }

    #[test]
    fn a_form_component_is_decoded_the_way_java_decodes_it() {
        assert_eq!(decode_form_component("a+b"), "a b");
        assert_eq!(decode_form_component("a%20b"), "a b");
        assert_eq!(decode_form_component("%2F%2f"), "//");
        // The bytes of a character of another language are decoded together.
        assert_eq!(decode_form_component("%E4%B8%AD%E6%96%87"), "中文");
        // A percent sign that names no byte is kept, so the rest of the value
        // is read instead of the whole answer being lost.
        assert_eq!(decode_form_component("100%"), "100%");
        assert_eq!(decode_form_component("%zz"), "%zz");
        assert_eq!(decode_form_component("%4"), "%4");
        // A byte that is not the start of a character is replaced rather than
        // refused.
        assert_eq!(decode_form_component("%FF"), "\u{FFFD}");
    }

    #[test]
    fn an_answer_that_is_not_json_is_a_failure() {
        let client = FakeClient::new(vec![Ok(HttpResponse::new(200, "<html>"))]);
        let failure =
            send_json::<serde_json::Value>(&client, &HttpRequest::get("https://example.com/a"))
                .expect_err("the answer should be refused");

        assert!(failure.to_string().contains("JSON"));
    }
}
