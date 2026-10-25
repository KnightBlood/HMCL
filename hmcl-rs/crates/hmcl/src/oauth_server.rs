//! The page of the login of Microsoft.
//!
//! The login of an account of Microsoft sends the player to a page of
//! Microsoft and the page sends the code of the login back to the launcher.
//! The launcher of Java holds a small server of its own, on the loopback
//! address of the machine, that answers the page with a text that tells the
//! player the login is done. This module is that server: it listens on the
//! ports the Java launcher listens on, so the address of the redirect of the
//! application of the launcher does not change, and it reads the code out of
//! the answer of the page.
//!
//! The server is created with the state and the verifier of the proof key of
//! the login (RFC 6749 and RFC 7636), which the launcher sends with the page
//! and checks when the page answers:
//!
//! * an answer without a code stops the login with
//!   `Failed to authenticate: missing authorization code`;
//! * an answer without the state stops the login with
//!   `Failed to authenticate: missing state`;
//! * an answer with another state stops the login with
//!   `Failed to authenticate: invalid state`;
//! * the server of the page is stopped a second after it answers, and the
//!   login ends when the page is dropped before the player logged in, with
//!   `OAuth server is closing`.
//!
//! The page is shown by the browser of the player, so it cannot read the
//! resources of the launcher; its colors and texts are written into the
//! document before it is sent, the way the launcher of Java does it.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

use hmcl_core::auth::{AuthenticationError, GrantFlow, OAuthCallback, OAuthSession};
use hmcl_core::i18n::I18n;
use hmcl_core::util::http;
use hmcl_core::util::http_request::decode_form_component;
use hmcl_core::util::logging;
use hmcl_core::util::operating_system::OsType;
use hmcl_core::util::{base64, random};

/// The ports the server of the page asks for, in order.
///
/// The ports are the ones the launcher of Java listens on, because the address
/// of the redirect of the application of the launcher names one of them. A
/// port another program holds is skipped, so a launcher still logs a player in
/// while another program of the launcher is open.
const PORTS: [u16; 5] = [29111, 29112, 29113, 29114, 29115];

/// The path of the page that answers with the code.
const AUTH_RESPONSE_PATH: &str = "/auth-response";

/// The document the page of the login shows.
///
/// The document is the one of the launcher of Java, so the two launchers show
/// the player the same page.
const PAGE: &str = include_str!("../ui/microsoft_auth.html");

/// The number of bytes of the state of a login.
///
/// The state is written in base 64 without the padding, so the state of a
/// login is 43 characters long.
const STATE_BYTES: usize = 32;

/// The number of bytes of the verifier of the proof key of a login.
///
/// RFC 7636 allows a verifier of 43 to 128 characters, and the challenge of a
/// verifier of 64 bytes is 86 characters long, which is the length the
/// launcher of Java uses.
const VERIFIER_BYTES: usize = 64;

/// The longest request line and header the server reads.
///
/// The requests of a page of a browser are short; a request that is longer is
/// not one of the page and is dropped, so a program of the machine cannot make
/// the launcher hold a long line in memory.
const MAX_LINE_BYTES: usize = 8 * 1024;

/// The longest body the server reads.
///
/// The answer of the page is a form of a few hundred bytes, so a longer body
/// is refused instead of read.
const MAX_BODY_BYTES: usize = 64 * 1024;

/// The time the server keeps the connection of the page after it answered.
///
/// The page of the browser needs the answer before the server goes away, and
/// the player may look at the page for a moment, so the server keeps the
/// answer alive the way the launcher of Java does.
const STOP_DELAY: Duration = Duration::from_secs(1);

/// The time the server waits for the rest of a request it refused.
const DISCARD_TIMEOUT: Duration = Duration::from_millis(200);

/// The time the server waits between two checks for a connection.
///
/// The server checks the flag that stops it between two connections, so a
/// server that is closed does not hold the thread that closed it.
const ACCEPT_INTERVAL: Duration = Duration::from_millis(20);

/// The programs that open the page of the login, in the order they are tried.
///
/// The programs are the ones the launcher of Java tries, so a player of a
/// distribution that installs any of them gets the page of the login.
const LINUX_BROWSERS: [&str; 7] = [
    "xdg-open",
    "google-chrome",
    "firefox",
    "microsoft-edge",
    "opera",
    "konqueror",
    "mozilla",
];

/// The answer of the page, which the login waits for.
///
/// The answer is written by the thread of the server and read by the thread of
/// the login, so the two are told apart by the lock: the login sleeps until the
/// server has written the answer.
#[derive(Default)]
struct Completion {
    /// The answer of the page, once the page answered.
    answer: Mutex<Option<Result<String, AuthenticationError>>>,
    /// Wakes the login when the page answers.
    answered: Condvar,
}

impl Completion {
    /// Writes the answer of the page, unless the page answered before.
    ///
    /// A page that answers twice, or a launcher that closes the server while
    /// the page answers, keeps the first answer: the first answer is the one
    /// the login waits for.
    fn complete(&self, answer: Result<String, AuthenticationError>) {
        let mut slot = self
            .answer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if slot.is_none() {
            *slot = Some(answer);
            self.answered.notify_all();
        }
    }

    /// Waits until the page answers and returns the answer.
    fn wait(&self) -> Result<String, AuthenticationError> {
        let mut slot = self
            .answer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        loop {
            if let Some(answer) = slot.as_ref() {
                return answer.clone();
            }
            slot = self
                .answered
                .wait(slot)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
    }
}

/// The texts and the colors of the page of the login.
///
/// The page is built once for each answer of the server, because the language
/// and the theme of the launcher may change while the launcher runs.
#[derive(Clone)]
pub struct Page {
    /// The texts of the launcher in the language of the player.
    i18n: I18n,
    /// The language of the page, written the way a browser reads it, such as
    /// `zh-Hans`.
    language: String,
    /// Whether the page is shown in the dark theme.
    dark: bool,
}

impl Page {
    /// Creates the texts and the colors of the page.
    ///
    /// `language` is the language the browser reads, such as `zh-Hans`, and
    /// `dark` tells which palette the page is drawn with.
    pub fn new(i18n: I18n, language: impl Into<String>, dark: bool) -> Self {
        Self {
            i18n,
            language: language.into(),
            dark,
        }
    }
    /// Returns the document of the page with the texts and the colors in it.
    ///
    /// The placeholders of the document are replaced as they are, without
    /// escaping the texts: the texts are the texts of the launcher, which hold
    /// no character of the document, and the launcher of Java replaces them the
    /// same way.
    fn render(&self) -> String {
        PAGE.replace("%style%", &crate::resources::page_style_sheet(self.dark))
            .replace("%lang%", &self.language)
            .replace("%success%", &self.i18n.i18n("message.success"))
            .replace("%ok%", &self.i18n.i18n("button.ok"))
            .replace(
                "%close_page%",
                &self.i18n.i18n("account.methods.microsoft.close_page"),
            )
    }
}

/// Returns the value of a name of a query string, decoded.
///
/// The parts of the query are separated by `&` and the name is separated from
/// the value by `=`. A part without a value is returned as an empty value, and
/// a part that cannot be decoded is skipped, so a page that writes a character
/// the launcher cannot read does not stop the login.
fn query_value(query: &str, name: &str) -> Option<String> {
    for part in query.split('&') {
        if part.is_empty() {
            continue;
        }
        let (key, value) = match part.split_once('=') {
            Some((key, value)) => (key, value),
            None => (part, ""),
        };
        if decode_form_component(key) == name {
            return Some(decode_form_component(value));
        }
    }
    None
}

/// Returns the program that opens the page in the browser of the player.
///
/// The program and its arguments are the ones the launcher of Java runs: the
/// handler of the protocol of Windows, the program of macOS, and the first
/// browser of [`LINUX_BROWSERS`] the path of the player names.
fn browser_command(
    system: OsType,
    mut find: impl FnMut(&str) -> Option<PathBuf>,
    url: &str,
) -> Option<(PathBuf, Vec<String>)> {
    if system.is_windows() {
        return Some((
            PathBuf::from("rundll32.exe"),
            vec!["url.dll,FileProtocolHandler".to_string(), url.to_string()],
        ));
    }

    if system.is_macos() {
        return Some((PathBuf::from("open"), vec![url.to_string()]));
    }

    LINUX_BROWSERS
        .iter()
        .find_map(|browser| find(browser))
        .map(|command| (command, vec![url.to_string()]))
}

/// Returns the path of a program the path of the player names.
///
/// The names of the programs of Windows end with `.exe`, so a name that names
/// no program is tried with the extension as well.
fn find_in_path(program: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let names: Vec<String> = if cfg!(windows) {
        vec![program.to_string(), format!("{program}.exe")]
    } else {
        vec![program.to_string()]
    };

    std::env::split_paths(&path)
        .flat_map(|directory| names.iter().map(move |name| directory.join(name)))
        .find(|candidate| candidate.is_file())
}

/// The message of a login whose page is closed before the player logged in.
const CLOSING_MESSAGE: &str = "OAuth server is closing";

/// The server the page of the login answers.
///
/// A server is created with the state and the verifier of the proof key of the
/// login and listens on the loopback address of the machine. The server is
/// stopped when the page answered, when the launcher closes the login, and
/// when the login is dropped, so the launcher holds no port after a login.
pub struct LocalServer {
    /// The port the server listens on.
    port: u16,
    /// The state the page is expected to answer with.
    state: String,
    /// The verifier of the proof key of the login.
    code_verifier: String,
    /// The answer of the page.
    completion: Arc<Completion>,
    /// Whether the server is to stop listening.
    stopped: Arc<AtomicBool>,
}

impl LocalServer {
    /// Starts a server on the first port of [`PORTS`] that is free.
    ///
    /// The server waits for the page on a thread of its own, so the login that
    /// started it keeps waiting for the answer. A machine whose ports are all
    /// held by other programs stops the login with the failure of the last
    /// port, which is the way the launcher of Java reports it.
    pub fn start(page: Page) -> Result<Self, AuthenticationError> {
        Self::start_on(&PORTS, page)
    }

    /// Starts a server on the first free port of a list.
    ///
    /// The port zero asks the system for a port that is free, which the tests
    /// use so two servers of two tests do not compete for the port of the
    /// launcher of the player.
    fn start_on(ports: &[u16], page: Page) -> Result<Self, AuthenticationError> {
        let state = base64::encode_url_safe_unpadded(&random::bytes::<STATE_BYTES>());
        let code_verifier = base64::encode_url_safe_unpadded(&random::bytes::<VERIFIER_BYTES>());

        let mut last_failure = None;
        for &port in ports {
            match TcpListener::bind((Ipv4Addr::LOCALHOST, port)) {
                Ok(listener) => {
                    return Ok(Self::listen(listener, state, code_verifier, page));
                }
                Err(error) => {
                    logging::warn(&format!(
                        "The page of the login cannot listen on the port {port}: {error}"
                    ));
                    last_failure = Some(error);
                }
            }
        }

        Err(match last_failure {
            Some(error) => AuthenticationError::disconnect(format!(
                "Cannot start the server of the login: {error}"
            )),
            None => AuthenticationError::disconnect("Cannot start the server of the login"),
        })
    }

    /// Creates a server that listens on a listener that is already bound.
    ///
    /// The port of the server is the port of the listener, which is the port
    /// the system chose when the launcher asked for the port zero.
    fn listen(listener: TcpListener, state: String, code_verifier: String, page: Page) -> Self {
        let port = listener
            .local_addr()
            .map(|address| address.port())
            .unwrap_or_default();
        let completion = Arc::new(Completion::default());
        let stopped = Arc::new(AtomicBool::new(false));

        let server = Self {
            port,
            state,
            code_verifier,
            completion: Arc::clone(&completion),
            stopped: Arc::clone(&stopped),
        };

        // The listener is set to not block, so the thread notices the flag that
        // stops it even when no page opens it; a listener that cannot be set is
        // used as it blocked, because a page still answers within a second.
        let _ = listener.set_nonblocking(true);

        let page = Arc::new(page);
        let expected_state = server.state.clone();
        thread::spawn(move || {
            serve(&listener, &completion, &stopped, &page, &expected_state);
            // The server answered the page and is of no use any more, so the
            // port is released before the login ends.
            drop(listener);
        });

        server
    }

    /// Returns the port the server listens on.
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// Stops the server and releases the port.
    ///
    /// A login that is still waiting for the page ends with the message of the
    /// launcher of Java, so a page that is closed while the launcher waits for
    /// it does not leave the login waiting forever.
    pub fn close(&self) {
        self.completion
            .complete(Err(AuthenticationError::aborted(CLOSING_MESSAGE)));
        self.stopped.store(true, Ordering::SeqCst);
    }
}

impl Drop for LocalServer {
    /// Stops the server when the login is done with it.
    fn drop(&mut self) {
        self.close();
    }
}

impl OAuthSession for LocalServer {
    /// Returns the state the page is expected to answer with.
    fn state(&self) -> String {
        self.state.clone()
    }

    /// Returns the verifier of the proof key of the login.
    fn code_verifier(&self) -> String {
        self.code_verifier.clone()
    }

    /// Returns the address the page answers with the code.
    fn redirect_uri(&self) -> String {
        // The address is the one of the launcher of Java with `localhost`
        // written as the name of the machine, because the address is the one
        // registered for the application of the launcher.
        format!("http://localhost:{}{AUTH_RESPONSE_PATH}", self.port())
    }

    /// Waits until the page answers with the code.
    fn wait_for_code(&self) -> Result<String, AuthenticationError> {
        self.completion.wait()
    }
}

/// Waits for the pages of the server and answers them.
///
/// The server answers the page at `?AUTH_RESPONSE_PATH` and refuses everything
/// else with the answer of a missing page, which is what the launcher of Java
/// does. The server stops when the page answered, a second later, or as soon as
/// the flag that closes it is set.
fn serve(
    listener: &TcpListener,
    completion: &Completion,
    stopped: &AtomicBool,
    page: &Page,
    expected_state: &str,
) {
    while !stopped.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                // A page that answers ends the wait of the login, so the server
                // is of no use after the answer; the answer itself is written
                // before the server goes away.
                if handle(stream, completion, page, expected_state).is_some() {
                    thread::sleep(STOP_DELAY);
                    return;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(ACCEPT_INTERVAL);
            }
            Err(error) => {
                // A connection that broke before it was accepted is not a
                // reason to stop the server: the page of the login may still
                // come.
                logging::warn(&format!("The page of the login was not read: {error}"));
            }
        }
    }
}

/// Reads one request, answers it and returns whether the page of the login
/// answered.
///
/// The answer of the page holds the code of the login, so the login is
/// completed here and not by the caller; the caller only stops the server when
/// the answer is done.
fn handle(
    mut stream: TcpStream,
    completion: &Completion,
    page: &Page,
    expected_state: &str,
) -> Option<()> {
    // The answer of the page is short and the browser waits for it, so the
    // connection is written as soon as the answer is known.
    let _ = stream.set_nodelay(true);

    let mut reader = BufReader::new(match stream.try_clone() {
        Ok(stream) => stream,
        Err(error) => {
            logging::warn(&format!("The page of the login was not read: {error}"));
            return None;
        }
    });

    let request = match read_request(&mut reader) {
        Ok(request) => request,
        Err(answer) => {
            write_answer(&mut stream, answer, "");
            // The request was refused before it was read to the end, so the
            // rest of it is still in the connection. The rest is read so the
            // connection closes in an orderly way instead of being reset,
            // which would drop the answer on the way to the browser.
            discard(&mut reader);
            return None;
        }
    };

    if request.path != AUTH_RESPONSE_PATH {
        write_answer(&mut stream, Answer::NotFound, "");
        return None;
    }

    // The launcher of Java reads the code out of the query string of the
    // address of the page, so a form the page sends is read to the end but not
    // used: only the query string is decoded.
    if request.method == Method::Other {
        write_answer(&mut stream, Answer::NotFound, "");
        return None;
    }
    let query = request.query.as_str();

    let code = query_value(query, "code");
    let state = query_value(query, "state");

    let answer = match code {
        Some(code) => match state {
            Some(state) if state == expected_state => {
                completion.complete(Ok(code));
                Ok(())
            }
            Some(_) => {
                logging::warn("Failed to authenticate: invalid state in parameters");
                Err("Failed to authenticate: invalid state")
            }
            None => {
                logging::warn("Failed to authenticate: missing state in parameters");
                Err("Failed to authenticate: missing state")
            }
        },
        None => {
            logging::warn("Failed to authenticate: missing authorization code in parameters");
            Err("Failed to authenticate: missing authorization code")
        }
    };

    if let Err(message) = answer {
        completion.complete(Err(AuthenticationError::aborted(message)));
    }

    let document = page.render();
    write_answer(&mut stream, Answer::Ok, &document);
    Some(())
}

/// The method of a request of the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Method {
    /// A page the browser asks for.
    Get,
    /// A form the page sends.
    Post,
    /// A method the server does not know.
    Other,
}

/// A request of the page.
#[derive(Debug, Clone)]
struct Request {
    /// The method of the request.
    method: Method,
    /// The path of the request, without the query string.
    path: String,
    /// The query string of the request, without the `?`.
    query: String,
}

/// The answer the server writes for a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Answer {
    /// The page of the login.
    Ok,
    /// A path or a method the server does not answer.
    NotFound,
    /// A request the server cannot read.
    BadRequest,
}

impl Answer {
    /// Returns the code and the reason of the answer.
    const fn status(self) -> (&'static str, &'static str) {
        match self {
            Self::Ok => ("200 OK", "text/html; charset=UTF-8"),
            Self::NotFound => ("404 Not Found", "text/html"),
            Self::BadRequest => ("400 Bad Request", "text/html"),
        }
    }
}

/// Reads the head of a request and its body.
///
/// The head is read line by line until the empty line that ends it. The body is
/// read when the head names its length, because the page sends a form in the
/// body and not in the query string. A line or a body that is longer than
/// [`MAX_LINE_BYTES`] or [`MAX_BODY_BYTES`] is refused, so a program of the
/// machine cannot make the launcher hold a long line in memory.
fn read_request(reader: &mut BufReader<TcpStream>) -> Result<Request, Answer> {
    let mut request_line = String::new();
    read_line(reader, &mut request_line)?;

    let mut parts = request_line.split_whitespace();
    let method = match parts.next() {
        Some("GET") => Method::Get,
        Some("POST") => Method::Post,
        Some(_) => Method::Other,
        None => return Err(Answer::BadRequest),
    };
    let target = parts.next().ok_or(Answer::BadRequest)?;

    let (path, query) = match target.split_once('?') {
        Some((path, query)) => (path, query),
        None => (target, ""),
    };

    let mut length = 0usize;
    loop {
        let mut line = String::new();
        read_line(reader, &mut line)?;
        if line.is_empty() {
            break;
        }

        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                length = value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| Answer::BadRequest)?;
            }
        }
    }

    if length > MAX_BODY_BYTES {
        return Err(Answer::BadRequest);
    }

    let mut body = vec![0u8; length];
    if length > 0 {
        reader
            .read_exact(&mut body)
            .map_err(|_| Answer::BadRequest)?;
    }

    Ok(Request {
        method,
        path: percent_path(path),
        query: query.to_string(),
    })
}

/// Reads and drops what is left of a request that was refused.
///
/// The connection is closed by the server after the answer, and a connection
/// that is closed with unread data is reset by the system of the machine, which
/// can make the browser of the player lose the answer it is waiting for. The
/// wait for more data is short, so a request that is not finished does not hold
/// the thread of the server.
fn discard(reader: &mut BufReader<TcpStream>) {
    let _ = reader.get_ref().set_read_timeout(Some(DISCARD_TIMEOUT));

    let mut buffer = [0u8; 1024];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(_) => continue,
            // The timeout ends the read, and any other failure means the
            // connection is gone.
            Err(_) => break,
        }
    }
}

/// Reads one line of a request without the line break.
///
/// A line longer than [`MAX_LINE_BYTES`] is refused, so a request that never
/// ends its line does not fill the memory of the launcher.
fn read_line(reader: &mut BufReader<TcpStream>, line: &mut String) -> Result<(), Answer> {
    let mut bytes = Vec::new();
    let read = reader
        .by_ref()
        .take(MAX_LINE_BYTES as u64)
        .read_until(b'\n', &mut bytes);

    match read {
        Ok(0) => Err(Answer::BadRequest),
        Ok(_) => {
            if !bytes.ends_with(b"\n") {
                // The line was cut by the limit before its end, so the rest of
                // the line is still in the stream and cannot be read as the
                // line of a header.
                return Err(Answer::BadRequest);
            }
            while bytes
                .last()
                .is_some_and(|byte| *byte == b'\n' || *byte == b'\r')
            {
                bytes.pop();
            }
            *line = String::from_utf8_lossy(&bytes).into_owned();
            Ok(())
        }
        Err(_) => Err(Answer::BadRequest),
    }
}

/// Returns the path of a request with the bytes of another language decoded.
///
/// A browser encodes a character that is not a letter of the address, so the
/// path of the page is compared once the characters are read.
fn percent_path(path: &str) -> String {
    decode_form_component(path)
}

/// Writes an answer and the document of the page.
///
/// The answer names the length of the document, so the browser shows the page
/// without waiting for the server to close the connection.
fn write_answer(stream: &mut TcpStream, answer: Answer, document: &str) {
    let (status, content_type) = answer.status();
    let head = format!(
        "HTTP/1.1 {status}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         Cache-Control: no-store\r\n\
         Connection: close\r\n\r\n",
        document.len()
    );

    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(document.as_bytes());
    let _ = stream.flush();
}

/// The name of the variable that holds the identifier of the application of
/// the launcher.
///
/// The launcher of Java reads the identifier from the attribute of the same
/// name of its jar, which the build writes from the variable of the build. A
/// program of Rust has no jar, so the identifier is read from the variable of
/// the program when it runs, and the build may write it into the program when
/// it is compiled.
const CLIENT_ID_VARIABLE: &str = "HMCL_MICROSOFT_AUTH_ID";

/// Returns the identifier of the application of the launcher.
///
/// The identifier is the one the build was given, unless the program of the
/// player names another one, which is what the launcher of Java allows through
/// its system property. A build without an identifier returns an empty text,
/// which refuses the login with
/// [`AuthenticationError::MicrosoftNotSupported`].
pub fn client_id() -> String {
    if let Ok(id) = std::env::var(CLIENT_ID_VARIABLE) {
        if !id.trim().is_empty() {
            return id;
        }
    }

    option_env!("HMCL_MICROSOFT_AUTH_ID")
        .unwrap_or("")
        .to_string()
}

/// The page of the login of the desktop launcher.
///
/// The launcher holds the local server of the page and opens the browser of the
/// player. The code of the device flow has no window to be shown in yet, so it
/// is written into the log, which is where a player who reports a login finds
/// it.
pub struct DesktopOAuthCallback {
    /// The identifier of the application of the launcher.
    client_id: String,
    /// The texts and the colors of the page, which the page of the server uses.
    page: Page,
    /// The address of the page that was opened last, which the launcher of Java
    /// keeps as well.
    last_opened_url: Mutex<Option<String>>,
}

impl DesktopOAuthCallback {
    /// Creates the page of the login of the launcher.
    pub fn new(client_id: String, page: Page) -> Self {
        Self {
            client_id,
            page,
            last_opened_url: Mutex::new(None),
        }
    }

    /// Returns the address of the page that was opened last, if the launcher
    /// opened one.
    ///
    /// The launcher of Java keeps the address in a field, so a page that cannot
    /// be opened can still be opened by hand. The launcher of the desktop shows
    /// the address in a window that does not exist yet, so only a test reads
    /// it.
    #[cfg(test)]
    pub fn last_opened_url(&self) -> Option<String> {
        self.last_opened_url
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

impl OAuthCallback for DesktopOAuthCallback {
    /// Starts the local server that receives the code of the page.
    ///
    /// A build without the identifier of its application cannot log a player
    /// in, so the login is refused before the server holds a port.
    fn start_session(&self) -> Result<Box<dyn OAuthSession>, AuthenticationError> {
        if self.client_id.trim().is_empty() {
            return Err(AuthenticationError::MicrosoftNotSupported);
        }

        Ok(Box::new(LocalServer::start(self.page.clone())?))
    }

    /// Shows the player the code and the address of the device flow.
    ///
    /// The launcher has no window that can show the code yet, so the code is
    /// written into the log; the browser of the player is opened by
    /// [`Self::open_browser`], which the login calls as well.
    fn grant_device_code(&self, user_code: &str, verification_uri: &str) {
        logging::info(&format!(
            "Log in at {verification_uri} with the code {user_code}"
        ));
    }

    /// Tells the player the login of the device flow is done.
    fn login_completed_device_code(&self) {
        logging::info("The login of the device flow is done");
    }

    /// Opens the page of Microsoft in the browser of the player.
    fn open_browser(&self, _flow: GrantFlow, url: &str) -> Result<(), String> {
        *self
            .last_opened_url
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(url.to_string());

        let encoded = http::encode_location(url);
        let (program, arguments) = browser_command(OsType::current(), find_in_path, &encoded)
            .ok_or_else(|| format!("No program that opens {url} was found"))?;

        std::process::Command::new(&program)
            .args(&arguments)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("Cannot open {}: {error}", program.display()))
    }

    /// Returns the identifier of the application of the launcher.
    fn client_id(&self) -> String {
        self.client_id.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hmcl_core::i18n::Locale;
    use std::net::TcpStream;

    /// Returns the texts of the launcher of the tests.
    fn texts() -> I18n {
        I18n::new(Locale::English)
    }

    /// Returns the page of the tests.
    fn page() -> Page {
        Page::new(texts(), "en", false)
    }

    /// Starts a server of the tests on a port the system chose.
    ///
    /// The ports of the launcher are the ones of the player, and two tests run
    /// at the same time, so a test asks for a free port instead.
    fn server() -> LocalServer {
        LocalServer::start_on(&[0], page()).expect("the server should start")
    }

    /// Sends a request to the server of the tests and returns the answer.
    ///
    /// The answer is read to the end, because the server names the length of
    /// the document and closes the connection after it. A connection the host
    /// resets while the request is sent still holds the bytes of the answer,
    /// so the reset is not an error of the test.
    fn ask(port: u16, request: &str) -> String {
        let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port))
            .expect("the server of the page should be listening");
        stream
            .write_all(request.as_bytes())
            .expect("the request should be sent");

        let mut answer = Vec::new();
        let _ = stream.read_to_end(&mut answer);
        String::from_utf8_lossy(&answer).into_owned()
    }

    /// Sends the answer of the page to the server of the tests.
    fn answer(port: u16, query: &str) -> String {
        ask(
            port,
            &format!("GET {AUTH_RESPONSE_PATH}?{query} HTTP/1.1\r\nHost: localhost\r\n\r\n"),
        )
    }

    #[test]
    fn a_query_is_read_name_by_name() {
        assert_eq!(
            query_value("code=abc&state=xyz", "code").as_deref(),
            Some("abc")
        );
        assert_eq!(
            query_value("code=abc&state=xyz", "state").as_deref(),
            Some("xyz")
        );
        assert_eq!(query_value("code=abc", "state"), None);
        assert_eq!(query_value("", "code"), None);
        // A name without a value is a name of an empty value, not a missing
        // name: the page may send `state=` to tell that it has no state.
        assert_eq!(query_value("state=", "state").as_deref(), Some(""));
        assert_eq!(query_value("state", "state").as_deref(), Some(""));
        // A part without a name is skipped instead of ending the search.
        assert_eq!(query_value("&&code=abc", "code").as_deref(), Some("abc"));
    }

    #[test]
    fn windows_opens_the_page_with_the_handler_of_the_protocol() {
        let (program, arguments) = browser_command(OsType::Windows, |_| None, "https://x/")
            .expect("Windows should have a handler of the protocol");

        assert_eq!(program, PathBuf::from("rundll32.exe"));
        assert_eq!(arguments, vec!["url.dll,FileProtocolHandler", "https://x/"]);
    }

    #[test]
    fn macos_opens_the_page_with_open() {
        let (program, arguments) = browser_command(OsType::MacOs, |_| None, "https://x/")
            .expect("macOS should have a program that opens a page");

        assert_eq!(program, PathBuf::from("open"));
        assert_eq!(arguments, vec!["https://x/"]);
    }

    #[test]
    fn linux_opens_the_page_with_the_first_browser_of_the_path() {
        let mut asked = Vec::new();
        let (program, arguments) = browser_command(
            OsType::Linux,
            |name| {
                asked.push(name.to_string());
                // The first two programs are not installed, so the third one is
                // the one the page is opened with.
                if name == LINUX_BROWSERS[2] {
                    Some(PathBuf::from("/usr/bin").join(name))
                } else {
                    None
                }
            },
            "https://x/",
        )
        .expect("a browser of the path should be found");

        assert_eq!(program, PathBuf::from("/usr/bin").join(LINUX_BROWSERS[2]));
        assert_eq!(arguments, vec!["https://x/"]);
        assert_eq!(&asked, &LINUX_BROWSERS[..3]);
    }

    #[test]
    fn linux_without_a_known_browser_opens_no_page() {
        assert!(browser_command(OsType::Linux, |_| None, "https://x/").is_none());
        // The launcher of Java reports an operating system it does not know as
        // the systems that are not Windows or macOS, so the browsers are tried.
        assert!(browser_command(OsType::Unknown, |_| None, "https://x/").is_none());
    }

    #[test]
    fn the_page_of_the_login_holds_the_texts_and_the_colors_of_the_launcher() {
        let document = page().render();

        assert!(!document.contains("%lang%"), "{document}");
        assert!(!document.contains("%style%"), "{document}");
        assert!(!document.contains("%success%"), "{document}");
        assert!(!document.contains("%ok%"), "{document}");
        assert!(!document.contains("%close_page%"), "{document}");

        assert!(document.contains(r#"<html lang="en">"#), "{document}");
        assert!(document.contains("--monet-surface"), "{document}");
        assert!(document.contains("Operation successfully completed"));
        assert!(document.contains(">OK<"));
    }

    #[test]
    fn the_page_of_the_login_follows_the_language_of_the_launcher() {
        let document = Page::new(texts(), "zh-Hans", true).render();

        assert!(document.contains(r#"<html lang="zh-Hans">"#));
        // The dark theme names the colors of the dark palette of the window.
        assert!(document.contains(crate::resources::page_style_sheet(true).as_str()));
        assert!(!document.contains(crate::resources::page_style_sheet(false).as_str()));
    }

    #[test]
    fn the_state_and_the_verifier_of_a_login_are_long_enough_for_its_rules() {
        let server = server();

        // RFC 6749 asks for a state that cannot be guessed, and RFC 7636 asks
        // for a verifier of 43 to 128 characters; the launcher of Java writes
        // 32 and 64 bytes as base 64 without padding.
        assert_eq!(server.state().len(), 43);
        assert_eq!(server.code_verifier().len(), 86);
        assert_ne!(server.state(), server.code_verifier());
        assert_eq!(
            server.redirect_uri(),
            format!("http://localhost:{}/auth-response", server.port())
        );
    }

    #[test]
    fn two_logins_do_not_share_their_state() {
        let first = server();
        let second = server();

        assert_ne!(first.state(), second.state());
        assert_ne!(first.code_verifier(), second.code_verifier());
    }

    #[test]
    fn the_page_answers_with_the_code_of_the_login() {
        let server = server();
        let state = server.state();

        let answered = answer(server.port(), &format!("code=the-code&state={state}"));

        assert!(answered.starts_with("HTTP/1.1 200 OK"), "{answered}");
        assert!(answered.contains("Operation successfully completed"));
        assert_eq!(
            server
                .wait_for_code()
                .expect("the page answers with a code"),
            "the-code"
        );
    }

    #[test]
    fn the_code_of_the_page_is_read_from_the_query_encoded() {
        let server = server();
        let state = server.state();

        answer(server.port(), &format!("code=a%2Fb+c&state={state}"));

        assert_eq!(
            server
                .wait_for_code()
                .expect("the page answers with a code"),
            "a/b c"
        );
    }

    #[test]
    fn the_page_without_a_code_stops_the_login() {
        let server = server();
        let state = server.state();

        answer(server.port(), &format!("state={state}"));

        let error = server
            .wait_for_code()
            .expect_err("a page without a code cannot log in");
        assert_eq!(
            error.to_string(),
            "Failed to authenticate: missing authorization code"
        );
    }

    #[test]
    fn the_page_without_a_state_stops_the_login() {
        let server = server();

        answer(server.port(), "code=the-code");

        let error = server
            .wait_for_code()
            .expect_err("a page without a state cannot log in");
        assert_eq!(error.to_string(), "Failed to authenticate: missing state");
    }

    #[test]
    fn the_page_with_another_state_stops_the_login() {
        let server = server();

        let answered = answer(server.port(), "code=the-code&state=another");

        // The page of the launcher is shown even so, because the player may
        // look at it after the login failed, which is what the launcher of Java
        // does.
        assert!(answered.starts_with("HTTP/1.1 200 OK"), "{answered}");
        let error = server
            .wait_for_code()
            .expect_err("a page of another login cannot log in");
        assert_eq!(error.to_string(), "Failed to authenticate: invalid state");
    }

    #[test]
    fn the_server_answers_no_page_but_the_one_of_the_login() {
        let server = server();

        let answered = ask(
            server.port(),
            "GET /other HTTP/1.1\r\nHost: localhost\r\n\r\n",
        );

        assert!(answered.starts_with("HTTP/1.1 404 Not Found"), "{answered}");
        // A request of another path does not end the login, so the page may
        // still answer.
        answer(server.port(), &format!("code=c&state={}", server.state()));
        assert_eq!(server.wait_for_code().expect("the page answers"), "c");
    }

    #[test]
    fn a_method_the_page_is_not_shown_with_answers_no_page() {
        let server = server();
        let state = server.state();

        let answered = ask(
            server.port(),
            &format!(
                "PUT {AUTH_RESPONSE_PATH}?code=c&state={state} HTTP/1.1\r\n\
                 Host: localhost\r\nContent-Length: 0\r\n\r\n"
            ),
        );

        assert!(answered.starts_with("HTTP/1.1 404 Not Found"), "{answered}");
    }

    #[test]
    fn the_page_of_the_form_is_shown_and_its_body_is_read() {
        let server = server();
        let state = server.state();
        let body = format!("code=the-code&state={state}");

        let answered = ask(
            server.port(),
            &format!(
                "POST {AUTH_RESPONSE_PATH}?code=the-code&state={state} HTTP/1.1\r\n\
                 Host: localhost\r\n\
                 Content-Type: application/x-www-form-urlencoded\r\n\
                 Content-Length: {}\r\n\r\n{body}",
                body.len()
            ),
        );

        assert!(answered.starts_with("HTTP/1.1 200 OK"), "{answered}");
        assert_eq!(
            server
                .wait_for_code()
                .expect("the page answers with a code"),
            "the-code"
        );
    }

    #[test]
    fn a_request_the_server_cannot_read_is_refused() {
        let server = server();

        // The head of the request is cut by the limit of a line, so the server
        // cannot read it as a request.
        let long = "a".repeat(MAX_LINE_BYTES + 1);
        let answered = ask(server.port(), &format!("GET /{long}\r\n"));

        assert!(
            answered.starts_with("HTTP/1.1 400 Bad Request"),
            "{answered}"
        );
    }

    #[test]
    fn a_server_that_is_closed_stops_the_login_of_the_page() {
        let server = server();

        server.close();

        let error = server
            .wait_for_code()
            .expect_err("a closed server cannot log in");
        assert_eq!(error.to_string(), CLOSING_MESSAGE);
    }

    #[test]
    fn a_login_without_the_identifier_of_the_application_is_refused() {
        let callback = DesktopOAuthCallback::new(String::new(), page());

        match callback.start_session() {
            Err(AuthenticationError::MicrosoftNotSupported) => {}
            Ok(_) => panic!("a launcher without an application cannot log in"),
            Err(error) => panic!("another error was reported: {error}"),
        }
    }

    #[test]
    fn a_login_with_the_identifier_of_the_application_starts_the_page() {
        let callback = DesktopOAuthCallback::new("the-application".to_string(), page());

        let session = callback
            .start_session()
            .expect("the launcher with an application logs in");

        assert_eq!(callback.client_id(), "the-application");
        assert!(session.redirect_uri().starts_with("http://localhost:"));
    }

    #[test]
    fn the_callback_keeps_the_address_of_the_page_it_opened() {
        let callback = DesktopOAuthCallback::new("the-application".to_string(), page());

        assert_eq!(callback.last_opened_url(), None);
        // The address is kept before the browser is opened, so a machine
        // without a program that opens a page still shows the address the
        // player may open by hand. The address is not a real one: the test
        // only asks whether the callback keeps the one it was told to open.
        let _ = callback.open_browser(GrantFlow::Device, "https://example.invalid/not-an-endpoint");
        assert_eq!(
            callback.last_opened_url().as_deref(),
            Some("https://example.invalid/not-an-endpoint")
        );
    }

    #[test]
    fn the_identifier_of_the_application_of_the_build_is_read() {
        // The variable of the program overrides the identifier the build wrote,
        // which is the way the launcher of Java reads its system property.
        let previous = std::env::var(CLIENT_ID_VARIABLE).ok();
        std::env::set_var(CLIENT_ID_VARIABLE, "from-the-program");
        assert_eq!(client_id(), "from-the-program");

        match previous {
            Some(value) => std::env::set_var(CLIENT_ID_VARIABLE, value),
            None => std::env::remove_var(CLIENT_ID_VARIABLE),
        }
    }
}
