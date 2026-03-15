// Copyright (C) 2026 The pgmoneta community
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use super::PgmonetaClient;
use crate::constant::Command;
use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
struct ShutdownRequest {
    #[serde(rename = "Username")]
    username: String,
}

impl PgmonetaClient {
    /// Sends a shutdown command to the pgmoneta server.
    ///
    /// # Arguments
    /// * `username` - The admin username making the request. Must be one of the pgmoneta admins configured in the system.
    ///
    /// # Returns
    /// The raw string response from the pgmoneta server.
    pub async fn request_shutdown(username: &str) -> anyhow::Result<String> {
        let shutdown_request = ShutdownRequest {
            username: username.to_string(),
        };
        Self::forward_request(username, Command::SHUTDOWN, shutdown_request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shutdown_request_serialization() {
        let request = ShutdownRequest {
            username: "admin".to_string(),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"Username\""));
        assert!(json.contains("\"admin\""));
    }

    #[test]
    fn test_shutdown_command_constant() {
        assert_eq!(Command::SHUTDOWN, 6);
    }

    #[test]
    fn test_shutdown_request_structure() {
        // Verify the request structure matches pgmoneta's expected format
        let request = ShutdownRequest {
            username: "backup_user".to_string(),
        };

        // Serialize and verify field naming
        let json_str = serde_json::to_string(&request).unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert!(json.get("Username").is_some());
        assert_eq!(
            json.get("Username").unwrap().as_str().unwrap(),
            "backup_user"
        );
    }
}
