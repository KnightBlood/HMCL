//! The forms the launcher sends to a server.
//!
//! A request of the launcher sends a body of JSON or a body of a form whose
//! fields are texts. The upload of a skin is neither: it sends a file next to a
//! field, which is a form of several parts, each part with its own headers.
//! This module writes such a body, the way the launcher of Java writes it, so a
//! server reads the same parts from both launchers.
//!
//! The body is written into memory before it is sent, because a part of a file
//! is short and the length of the body is told to the server before it is sent.

use crate::util::datetime::unix_millis;

/// The end of a line of the body of a form of several parts.
const ENDL: &[u8] = b"\r\n";

/// The type of a body of a form of several parts, before the boundary.
const CONTENT_TYPE_MULTIPART: &str = "multipart/form-data; boundary=";

/// The body of a form of several parts.
///
/// The boundary is the text that tells the parts apart; it must not appear in
/// the body. The launcher writes a boundary out of the moment the body is
/// written, which is what the launcher of Java does, so two bodies of one
/// launcher differ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultipartForm {
    /// The text that tells the parts apart.
    boundary: String,
    /// The bytes of the body written so far.
    body: Vec<u8>,
}

impl MultipartForm {
    /// Creates a form with a boundary made of the current moment.
    pub fn new() -> Self {
        Self::with_boundary(format!("*****{}*****", unix_millis()))
    }

    /// Creates a form with the boundary the caller names.
    ///
    /// The boundary is written as it is given, so a caller that writes it
    /// itself must not use a text the body of a part holds.
    pub fn with_boundary(boundary: impl Into<String>) -> Self {
        Self {
            boundary: boundary.into(),
            body: Vec::new(),
        }
    }

    /// Returns the boundary of the form.
    pub fn boundary(&self) -> &str {
        &self.boundary
    }

    /// Returns the type of the body, which names the boundary.
    pub fn content_type(&self) -> String {
        format!("{CONTENT_TYPE_MULTIPART}{}", self.boundary)
    }

    /// Adds a part that holds a text.
    pub fn field(mut self, name: &str, value: &str) -> Self {
        self.line(&format!("--{}", self.boundary));
        self.line(&format!("Content-Disposition: form-data; name=\"{name}\""));
        self.line("");
        self.line(value);
        self
    }

    /// Adds a part that holds a file.
    ///
    /// The name is the name the file has on the machine of the player, which
    /// the server shows; the type is the type of the file, such as `image/png`.
    pub fn file(mut self, name: &str, file_name: &str, content_type: &str, data: &[u8]) -> Self {
        self.line(&format!("--{}", self.boundary));
        self.line(&format!(
            "Content-Disposition: form-data; name=\"{name}\"; filename=\"{file_name}\""
        ));
        self.line(&format!("Content-Type: {content_type}"));
        self.line("");
        self.body.extend_from_slice(data);
        self.body.extend_from_slice(ENDL);
        self
    }

    /// Returns the body of the form, with its last boundary.
    pub fn finish(mut self) -> Vec<u8> {
        self.line(&format!("--{}--", self.boundary));
        self.body
    }

    /// Writes a line of the body, with the end of the line.
    fn line(&mut self, text: &str) {
        self.body.extend_from_slice(text.as_bytes());
        self.body.extend_from_slice(ENDL);
    }
}

impl Default for MultipartForm {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_form_writes_its_parts_in_the_order_they_are_added() {
        let body = MultipartForm::with_boundary("the-boundary")
            .field("model", "slim")
            .file("file", "skin.png", "image/png", b"the-skin")
            .finish();

        let text = String::from_utf8_lossy(&body);
        assert_eq!(
            text,
            "--the-boundary\r\n\
             Content-Disposition: form-data; name=\"model\"\r\n\
             \r\n\
             slim\r\n\
             --the-boundary\r\n\
             Content-Disposition: form-data; name=\"file\"; filename=\"skin.png\"\r\n\
             Content-Type: image/png\r\n\
             \r\n\
             the-skin\r\n\
             --the-boundary--\r\n"
        );
    }

    #[test]
    fn the_type_of_the_body_names_the_boundary() {
        let form = MultipartForm::with_boundary("the-boundary");
        assert_eq!(
            form.content_type(),
            "multipart/form-data; boundary=the-boundary"
        );
        assert_eq!(form.boundary(), "the-boundary");
    }

    #[test]
    fn the_boundary_of_a_form_is_made_of_the_moment() {
        let form = MultipartForm::new();
        assert!(form.boundary().starts_with("*****"));
        assert!(form.boundary().ends_with("*****"));
        assert_eq!(form.boundary().len(), 5 + 13 + 5);
    }

    #[test]
    fn a_form_without_a_part_is_only_its_last_boundary() {
        let body = MultipartForm::with_boundary("b").finish();
        assert_eq!(String::from_utf8_lossy(&body), "--b--\r\n");
    }

    #[test]
    fn a_file_that_holds_the_bytes_of_a_line_end_is_written_as_it_is() {
        // The body of a part is written without reading it as a text, so a
        // file that holds the bytes of an end of a line keeps them.
        let body = MultipartForm::with_boundary("b")
            .file("file", "f", "image/png", b"a\r\nb")
            .finish();
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("\r\n\r\na\r\nb\r\n--b--\r\n"), "{text}");
    }
}
