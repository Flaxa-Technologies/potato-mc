use serde::{Deserialize, Serialize};

/// Configuration for the dedicated server management service (JSON-RPC over WebSocket) introduced in Minecraft 26.3.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct ManagementConfig {
    /// Whether the dedicated server management service is enabled.
    pub enabled: bool,
    /// Host interface address to bind the management service to.
    pub host: String,
    /// Port to listen on for management connections (0 chooses an ephemeral port).
    pub port: u16,
    /// Shared secret or authentication token for JSON-RPC management clients.
    pub secret: String,
    /// Whether TLS is enabled on the management WebSocket endpoint.
    pub tls_enabled: bool,
    /// Path to the TLS keystore/certificate for secure management communication.
    pub tls_keystore: String,
    /// Password for the TLS keystore.
    pub tls_keystore_password: String,
    /// Allowed origins for WebSocket CORS checks.
    pub allowed_origins: String,
}

impl Default for ManagementConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: "localhost".to_string(),
            port: 0,
            secret: String::new(),
            tls_enabled: true,
            tls_keystore: String::new(),
            tls_keystore_password: String::new(),
            allowed_origins: String::new(),
        }
    }
}
