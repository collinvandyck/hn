use std::fmt;

/// API errors with user-friendly messages.
#[derive(Debug)]
pub enum ApiError {
    /// Network-level failure (connection, timeout, DNS)
    Network(String),
    /// HTTP error response (4xx, 5xx)
    HttpStatus(u16, String),
    /// Failed to parse response
    Parse(String),
}

impl ApiError {
    /// Returns a user-friendly error message.
    pub fn user_message(&self) -> String {
        match self {
            Self::Network(details) => {
                if details.contains("timed out") {
                    "Request timed out. Please try again.".into()
                } else if details.contains("dns") || details.contains("resolve") {
                    "Network error: Could not reach server.".into()
                } else {
                    format!("Network error: {}", details)
                }
            }
            Self::HttpStatus(429, _) => "Rate limited. Please wait a moment.".into(),
            Self::HttpStatus(404, _) => "Item not found.".into(),
            Self::HttpStatus(500..=599, _) => "Server error. Please try again later.".into(),
            Self::HttpStatus(code, msg) => format!("HTTP error {}: {}", code, msg),
            Self::Parse(details) => format!("Failed to parse response: {}", details),
        }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.user_message())
    }
}

impl std::error::Error for ApiError {}

impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            Self::Network("request timed out".into())
        } else if err.is_connect() {
            Self::Network("connection failed".into())
        } else if err.is_decode() {
            Self::Parse(err.to_string())
        } else if let Some(status) = err.status() {
            Self::HttpStatus(
                status.as_u16(),
                status.canonical_reason().unwrap_or("").into(),
            )
        } else {
            Self::Network(err.to_string())
        }
    }
}
