use pgmoneta_mcp::configuration::{
    CONFIG, Configuration, PgmonetaConfiguration, PgmonetaMcpConfiguration,
};
use pgmoneta_mcp::security::SecurityUtil;
use std::collections::HashMap;
use std::sync::Once;

static INIT_CONFIG: Once = Once::new();

pub fn init_config() {
    INIT_CONFIG.call_once(|| {
        let force_plain = std::env::var("PGMONETA_MCP_FORCE_PLAIN")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let requested_compression =
            std::env::var("PGMONETA_MCP_COMPRESSION").unwrap_or_else(|_| "zstd".to_string());
        let requested_encryption =
            std::env::var("PGMONETA_MCP_ENCRYPTION").unwrap_or_else(|_| "aes_256_gcm".to_string());

        let compression = if force_plain {
            "none".to_string()
        } else {
            requested_compression
        };

        let encryption = if force_plain {
            "none".to_string()
        } else {
            requested_encryption
        };

        let security: SecurityUtil = SecurityUtil::new();
        let (master_password, master_salt) =
            security.load_master_key().expect("master key must exist");
        let encrypted = security
            .encrypt_to_base64_string(b"backup_pass", &master_password, &master_salt)
            .expect("password encryption should succeed");

        let mut admins: HashMap<String, String> = HashMap::new();
        admins.insert("backup_user".to_string(), encrypted);

        let config = Configuration {
            pgmoneta_mcp: PgmonetaMcpConfiguration {
                port: 8000,
                log_path: "pgmoneta_mcp.log".to_string(),
                log_level: "info".to_string(),
                log_type: "console".to_string(),
                log_line_prefix: "%Y-%m-%d %H:%M:%S".to_string(),
                log_mode: "append".to_string(),
                log_rotation_age: "0".to_string(),
            },
            pgmoneta: PgmonetaConfiguration {
                host: "127.0.0.1".to_string(),
                port: 5002,
                compression,
                encryption,
            },
            admins,
            llm: None,
        };

        CONFIG
            .set(config)
            .expect("CONFIG should be initialized once");
    });
}
