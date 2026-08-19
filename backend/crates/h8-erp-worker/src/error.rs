use std::{error::Error, fmt};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WorkerError {
    code: &'static str,
    message: String,
}

impl WorkerError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn code(&self) -> &'static str {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for WorkerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl Error for WorkerError {}

impl From<deadpool_tiberius_rustls::tiberius_rustls::error::Error> for WorkerError {
    fn from(error: deadpool_tiberius_rustls::tiberius_rustls::error::Error) -> Self {
        Self::new("H8_WORKER_MSSQL_FAILED", error.to_string())
    }
}
