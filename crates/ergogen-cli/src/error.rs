use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// CLI usage error (missing args, invalid flags).
    Usage = 1,
    /// Input error (missing file, invalid YAML/JSON, etc.).
    Input = 2,
    /// Processing error (internal failure while generating outputs).
    Processing = 3,
}

#[derive(Debug)]
pub struct CliError {
    pub code: ErrorCode,
    pub message: String,
}

impl CliError {
    pub fn usage(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::Usage,
            message: message.into(),
        }
    }

    pub fn input(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::Input,
            message: message.into(),
        }
    }

    pub fn processing(message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::Processing,
            message: message.into(),
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
