use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChronicleError {
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Date/time error: {0}")]
    DateTime(#[from] chrono::ParseError),

    #[error("Arrow error: {0}")]
    Arrow(#[from] arrow::error::ArrowError),

    #[error("Parquet error: {0}")]
    Parquet(#[from] parquet::errors::ParquetError),

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("API error: {message}")]
    Api { message: String },

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Permission denied: {0}")]
    Permission(String),

    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    #[error("Invalid time range: {0}")]
    InvalidTimeRange(String),

    #[error("Export error: {0}")]
    Export(String),

    #[error("Backup error: {0}")]
    Backup(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Operation cancelled by user")]
    Cancelled,

    #[error("Operation timed out")]
    Timeout,

    #[error("Insufficient storage space")]
    InsufficientStorage,

    #[error("Service unavailable")]
    ServiceUnavailable,

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl ChronicleError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            ChronicleError::Network(_)
                | ChronicleError::ServiceUnavailable
                | ChronicleError::Timeout
        )
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            ChronicleError::Config(_) => 1,
            ChronicleError::Io(_) => 2,
            ChronicleError::Auth(_) => 3,
            ChronicleError::Permission(_) => 4,
            ChronicleError::FileNotFound { .. } => 5,
            ChronicleError::InvalidQuery(_) => 6,
            ChronicleError::InvalidTimeRange(_) => 7,
            ChronicleError::Api { .. } => 8,
            ChronicleError::Network(_) => 9,
            ChronicleError::ServiceUnavailable => 10,
            ChronicleError::Cancelled => 130, // Standard Unix signal for SIGINT
            ChronicleError::Timeout => 124,   // Standard timeout exit code
            _ => 1,                           // Generic error
        }
    }
}

pub type Result<T> = std::result::Result<T, ChronicleError>;

/// Format error for user-friendly display
pub fn format_error(error: &ChronicleError) -> String {
    match error {
        ChronicleError::Config(e) => {
            format!("Configuration Error: {}\n\nTry running 'chronictl config show' to check your configuration.", e)
        }
        ChronicleError::Auth(msg) => {
            format!("Authentication Error: {}\n\nPlease check your credentials and try again.", msg)
        }
        ChronicleError::Permission(msg) => {
            format!("Permission Denied: {}\n\nPlease check file permissions or run with appropriate privileges.", msg)
        }
        ChronicleError::FileNotFound { path } => {
            format!("File Not Found: {}\n\nPlease check that the file exists and is accessible.", path)
        }
        ChronicleError::InvalidQuery(msg) => {
            format!("Invalid Query: {}\n\nPlease check your query syntax. Use 'chronictl search --help' for examples.", msg)
        }
        ChronicleError::InvalidTimeRange(msg) => {
            format!("Invalid Time Range: {}\n\nSupported formats: '2024-01-01', '2024-01-01T10:00:00', 'last-week', 'today', etc.", msg)
        }
        ChronicleError::ServiceUnavailable => {
            "Service Unavailable: Chronicle service is not running or not accessible.\n\nTry running 'chronictl status' to check service health.".to_string()
        }
        ChronicleError::Network(msg) => {
            format!("Network Error: {}\n\nPlease check your network connection and try again.", msg)
        }
        ChronicleError::Cancelled => {
            "Operation cancelled by user.".to_string()
        }
        _ => error.to_string(),
    }
}