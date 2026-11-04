//! The one error type the port raises, carrying the message the
//! Python DeadgateError carries.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadgateError {
    pub message: String,
}

impl DeadgateError {
    pub fn new(message: impl Into<String>) -> Self {
        DeadgateError {
            message: message.into(),
        }
    }
}

impl fmt::Display for DeadgateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for DeadgateError {}
