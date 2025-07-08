use crate::error::{ChronicleError, Result};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::timeout;

#[derive(Clone)]
pub struct ChronicleClient {
    client: Client,
    base_url: String,
    timeout: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub uptime: u64,
    pub storage_usage: StorageInfo,
    pub memory_usage: MemoryInfo,
    pub active_connections: u32,
    pub last_event_time: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StorageInfo {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub usage_percent: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub usage_percent: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub filters: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub events: Vec<Event>,
    pub total_count: usize,
    pub query_time_ms: u64,
    pub has_more: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_type: String,
    pub data: serde_json::Value,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportRequest {
    pub format: String,
    pub query: SearchQuery,
    pub destination: Option<String>,
    pub compression: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExportResponse {
    pub export_id: String,
    pub status: String,
    pub download_url: Option<String>,
    pub file_size: Option<u64>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupRequest {
    pub destination: String,
    pub include_metadata: bool,
    pub compression: Option<String>,
    pub encryption: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupResponse {
    pub backup_id: String,
    pub status: String,
    pub file_path: Option<String>,
    pub file_size: Option<u64>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WipeRequest {
    pub confirm_passphrase: String,
    pub preserve_config: bool,
    pub secure_delete: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigInfo {
    pub config: HashMap<String, serde_json::Value>,
    pub config_file: String,
    pub last_modified: chrono::DateTime<chrono::Utc>,
}

impl ChronicleClient {
    pub fn new(base_url: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url,
            timeout: Duration::from_secs(30),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    async fn request_with_timeout<T>(&self, future: impl std::future::Future<Output = Result<T>>) -> Result<T> {
        timeout(self.timeout, future)
            .await
            .map_err(|_| ChronicleError::Timeout)?
    }

    pub async fn health(&self) -> Result<HealthStatus> {
        self.request_with_timeout(async {
            let response = self
                .client
                .get(&format!("{}/health", self.base_url))
                .send()
                .await?;

            self.handle_response(response).await
        })
        .await
    }

    pub async fn search(&self, query: &SearchQuery) -> Result<SearchResult> {
        self.request_with_timeout(async {
            let response = self
                .client
                .post(&format!("{}/search", self.base_url))
                .json(query)
                .send()
                .await?;

            self.handle_response(response).await
        })
        .await
    }

    pub async fn export(&self, request: &ExportRequest) -> Result<ExportResponse> {
        self.request_with_timeout(async {
            let response = self
                .client
                .post(&format!("{}/export", self.base_url))
                .json(request)
                .send()
                .await?;

            self.handle_response(response).await
        })
        .await
    }

    pub async fn export_status(&self, export_id: &str) -> Result<ExportResponse> {
        self.request_with_timeout(async {
            let response = self
                .client
                .get(&format!("{}/export/{}", self.base_url, export_id))
                .send()
                .await?;

            self.handle_response(response).await
        })
        .await
    }

    pub async fn download_export(&self, export_id: &str) -> Result<reqwest::Response> {
        self.request_with_timeout(async {
            let response = self
                .client
                .get(&format!("{}/export/{}/download", self.base_url, export_id))
                .send()
                .await?;

            if response.status().is_success() {
                Ok(response)
            } else {
                Err(ChronicleError::Api {
                    message: format!("Failed to download export: {}", response.status()),
                })
            }
        })
        .await
    }

    pub async fn backup(&self, request: &BackupRequest) -> Result<BackupResponse> {
        self.request_with_timeout(async {
            let response = self
                .client
                .post(&format!("{}/backup", self.base_url))
                .json(request)
                .send()
                .await?;

            self.handle_response(response).await
        })
        .await
    }

    pub async fn backup_status(&self, backup_id: &str) -> Result<BackupResponse> {
        self.request_with_timeout(async {
            let response = self
                .client
                .get(&format!("{}/backup/{}", self.base_url, backup_id))
                .send()
                .await?;

            self.handle_response(response).await
        })
        .await
    }

    pub async fn wipe(&self, request: &WipeRequest) -> Result<()> {
        self.request_with_timeout(async {
            let response = self
                .client
                .post(&format!("{}/wipe", self.base_url))
                .json(request)
                .send()
                .await?;

            let _: serde_json::Value = self.handle_response(response).await?;
            Ok(())
        })
        .await
    }

    pub async fn config(&self) -> Result<ConfigInfo> {
        self.request_with_timeout(async {
            let response = self
                .client
                .get(&format!("{}/config", self.base_url))
                .send()
                .await?;

            self.handle_response(response).await
        })
        .await
    }

    pub async fn update_config(&self, config: &HashMap<String, serde_json::Value>) -> Result<ConfigInfo> {
        self.request_with_timeout(async {
            let response = self
                .client
                .put(&format!("{}/config", self.base_url))
                .json(config)
                .send()
                .await?;

            self.handle_response(response).await
        })
        .await
    }

    async fn handle_response<T>(&self, response: Response) -> Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let status = response.status();
        let text = response.text().await?;

        if status.is_success() {
            // Try to parse as API response first
            if let Ok(api_response) = serde_json::from_str::<ApiResponse<T>>(&text) {
                if api_response.success {
                    api_response.data.ok_or_else(|| ChronicleError::Api {
                        message: "API response missing data".to_string(),
                    })
                } else {
                    Err(ChronicleError::Api {
                        message: api_response.error.unwrap_or_else(|| "Unknown API error".to_string()),
                    })
                }
            } else {
                // Try to parse directly as T
                serde_json::from_str::<T>(&text).map_err(|e| ChronicleError::Api {
                    message: format!("Failed to parse response: {}", e),
                })
            }
        } else {
            // Try to parse error response
            if let Ok(api_response) = serde_json::from_str::<ApiResponse<serde_json::Value>>(&text) {
                Err(ChronicleError::Api {
                    message: api_response.error.unwrap_or_else(|| format!("HTTP {}", status)),
                })
            } else {
                Err(ChronicleError::Api {
                    message: format!("HTTP {}: {}", status, text),
                })
            }
        }
    }

    pub async fn ping(&self) -> Result<()> {
        self.request_with_timeout(async {
            let response = self
                .client
                .get(&format!("{}/ping", self.base_url))
                .send()
                .await?;

            if response.status().is_success() {
                Ok(())
            } else {
                Err(ChronicleError::ServiceUnavailable)
            }
        })
        .await
    }
}