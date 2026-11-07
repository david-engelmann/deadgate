//! A text in the form the Python runtime holds it, a run of
//! code points where a byte that did not decode as UTF-8
//! stands as the code point the runtime's escape decoding
//! assigns it, U+DC00 plus the byte. Argument words and file
//! names reach the tool as operating system bytes, and the
//! runtime decodes them with that escape standing, so a word
//! the bytes spell is carried here in points and spelled back
//! on each surface the way the runtime spells it, as a Unicode
//! escape in the JSON rendering, as the raw byte on standard
//! output, and as the escape text on standard error, where the
//! runtime's error handler writes the escape for a code point
//! it cannot encode.

use std::ffi::OsStr;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PyText {
    points: Vec<u32>,
}

const ESCAPE_BASE: u32 = 0xDC00;

fn is_escape_point(point: u32) -> bool {
    (0xD800..0xE000).contains(&point)
}

/// Decode one UTF-8 sequence standing at the head of the
/// bytes, under the rules the runtime's decoder applies, and
/// return the code point and the width it stands in, or None
/// when the head byte opens no well formed sequence here.
fn decode_sequence(bytes: &[u8]) -> Option<(u32, usize)> {
    let first = *bytes.first()?;
    if first < 0x80 {
        return Some((u32::from(first), 1));
    }
    let (width, low, high) = match first {
        0xC2..=0xDF => (2, 0x80, 0xBF),
        0xE0 => (3, 0xA0, 0xBF),
        0xE1..=0xEC | 0xEE | 0xEF => (3, 0x80, 0xBF),
        0xED => (3, 0x80, 0x9F),
        0xF0 => (4, 0x90, 0xBF),
        0xF1..=0xF3 => (4, 0x80, 0xBF),
        0xF4 => (4, 0x80, 0x8F),
        _ => return None,
    };
    if bytes.len() < width {
        return None;
    }
    let second = bytes[1];
    if !(low..=high).contains(&second) {
        return None;
    }
    if bytes[2..width]
        .iter()
        .any(|byte| !(0x80..=0xBF).contains(byte))
    {
        return None;
    }
    let mut point = u32::from(first) & (0x7F >> width);
    for byte in &bytes[1..width] {
        point = (point << 6) | u32::from(byte & 0x3F);
    }
    Some((point, width))
}

impl PyText {
    pub fn new() -> Self {
        PyText { points: Vec::new() }
    }

    /// The text of a plain string, point for point.
    pub fn from_plain(text: &str) -> PyText {
        PyText {
            points: text.chars().map(u32::from).collect(),
        }
    }

    /// The text the runtime reads in a run of bytes, well
    /// formed UTF-8 standing as its characters and every byte
    /// that opens no well formed sequence standing as its
    /// escape point, one byte at a time.
    pub fn from_bytes(bytes: &[u8]) -> PyText {
        let mut points = Vec::new();
        let mut index = 0;
        while index < bytes.len() {
            match decode_sequence(&bytes[index..]) {
                Some((point, width)) => {
                    points.push(point);
                    index += width;
                }
                None => {
                    points.push(ESCAPE_BASE + u32::from(bytes[index]));
                    index += 1;
                }
            }
        }
        PyText { points }
    }

    /// The text of an operating system word, read in as bytes.
    pub fn from_os_str(value: &OsStr) -> PyText {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            PyText::from_bytes(value.as_bytes())
        }
        #[cfg(not(unix))]
        {
            PyText::from_plain(&value.to_string_lossy())
        }
    }

    pub fn points(&self) -> &[u32] {
        &self.points
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Whether no escape point stands in the text, so the text
    /// is a plain string.
    pub fn is_plain(&self) -> bool {
        !self.points.iter().any(|point| is_escape_point(*point))
    }

    /// The plain string the text stands as, when it is one.
    pub fn plain(&self) -> Option<String> {
        if !self.is_plain() {
            return None;
        }
        let mut out = String::new();
        for point in &self.points {
            out.push(char::from_u32(*point)?);
        }
        Some(out)
    }

    /// Whether the text reads as the given plain string.
    pub fn equals_str(&self, text: &str) -> bool {
        let mut chars = text.chars();
        for point in &self.points {
            match chars.next() {
                Some(ch) if u32::from(ch) == *point => {}
                _ => return false,
            }
        }
        chars.next().is_none()
    }

    pub fn push_points(&mut self, points: &[u32]) {
        self.points.extend_from_slice(points);
    }

    pub fn push_str(&mut self, text: &str) {
        self.points.extend(text.chars().map(u32::from));
    }

    pub fn push_text(&mut self, other: &PyText) {
        self.points.extend_from_slice(&other.points);
    }

    /// The spelling standard error carries, every escape point
    /// written as its Unicode escape text and every other
    /// point written as the character it is, as the runtime's
    /// standard error handler writes a text it cannot encode.
    pub fn stderr_spelling(&self) -> String {
        let mut out = String::new();
        for point in &self.points {
            if is_escape_point(*point) {
                out.push_str(&format!("\\u{point:04x}"));
            } else if let Some(ch) = char::from_u32(*point) {
                out.push(ch);
            }
        }
        out
    }

    /// The bytes standard output carries, every escape point
    /// written as the byte it stands for and every other point
    /// written as its UTF-8, as the runtime's escape decoding
    /// round trips through its standard output.
    pub fn stdout_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for point in &self.points {
            if (ESCAPE_BASE + 0x80..ESCAPE_BASE + 0x100).contains(point) {
                out.push((*point - ESCAPE_BASE) as u8);
            } else if let Some(ch) = char::from_u32(*point) {
                let mut buffer = [0u8; 4];
                out.extend_from_slice(ch.encode_utf8(&mut buffer).as_bytes());
            }
        }
        out
    }
}

impl Default for PyText {
    fn default() -> Self {
        PyText::new()
    }
}

/// The spelling a path carries in a refusal message, the
/// runtime's own reading of the path word under its standard
/// error handler.
pub fn path_text(path: &Path) -> String {
    PyText::from_os_str(path.as_os_str()).stderr_spelling()
}
