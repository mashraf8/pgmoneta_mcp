# Migration

## From 0.1.x to 0.2.0

### Vault Encryption

The key derivation for vault file encryption has been upgraded to
`PKCS5_PBKDF2_HMAC` (SHA-256, random 16-byte salt, 600,000 iterations).

This is a **breaking change**. Existing vault files encrypted with the
old method cannot be decrypted by version 0.2.0.

**Action required:**

1. Stop pgmoneta-mcp
2. Delete the existing admin configuration file:
   - `pgmoneta_admins.conf` (or the file specified with `-f`)
3. Delete the existing master key:
   - On Linux/Unix: `rm ~/.pgmoneta/master.key`
4. Regenerate the master key:
   ```
   pgmoneta-mcp-admin master-key
   ```
5. Re-add all users/admins:
   ```
   pgmoneta-mcp-admin user add -U <username> -P <password> -f <admins_file>
   ```
6. Restart pgmoneta-mcp

### AES-GCM Upgrade

The encryption system has been upgraded to exclusively use **AES-GCM** (Galois/Counter Mode). Support for legacy CBC and CTR modes has been removed.

**Changes:**
1.  **Strict Enforcement**: Legacy identifiers (`aes_256_cbc`, etc.) are no longer supported.
2.  **Unified Protocol**: All encrypted communication now strictly follows the AES-GCM bundle format.
3.  **Expanded Bit-Length**: Native support for 128, 192, and 256-bit GCM.

**Action Required:**
- Update `pgmoneta-mcp.conf` and set the `encryption` field to one of:
  - `aes_256_gcm` (Recommended)
  - `aes_192_gcm`
  - `aes_128_gcm`
  - `none`

> [!WARNING]
> This is a breaking change. If your configuration continues to use legacy identifiers (`aes_256_cbc`, etc.), the MCP server will now return an explicit error and fail to connect. You MUST update your configuration to a supported GCM mode.

