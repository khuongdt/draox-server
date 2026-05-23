//! SQL schema definitions for all core Draox server tables.
//!
//! Each [`SchemaDefinition`] bundles:
//! - `table_name` — short identifier used for look-ups.
//! - `create_sql` — the `CREATE TABLE IF NOT EXISTS …` statement.
//! - `indexes` — zero or more `CREATE INDEX IF NOT EXISTS …` statements.
//!
//! All SQL is SQLite-compatible; it also runs on PostgreSQL / MySQL with minor
//! dialect differences (not enforced here — callers adapt as needed).

// ── SchemaDefinition ──────────────────────────────────────────────────────────

/// A table schema together with its associated indexes.
pub struct SchemaDefinition {
    /// Short, stable table name (matches the SQL table name).
    pub table_name: &'static str,
    /// `CREATE TABLE IF NOT EXISTS …` DDL for the table.
    pub create_sql: &'static str,
    /// Additional `CREATE INDEX IF NOT EXISTS …` statements (may be empty).
    pub indexes: &'static [&'static str],
}

// ── SCHEMAS ───────────────────────────────────────────────────────────────────

/// All core table schemas in the Draox server database.
pub const SCHEMAS: &[SchemaDefinition] = &[
    // ── sessions ─────────────────────────────────────────────────────────────
    SchemaDefinition {
        table_name: "sessions",
        create_sql: "CREATE TABLE IF NOT EXISTS sessions (
            id            TEXT PRIMARY KEY,
            client_id     TEXT NOT NULL,
            state         TEXT NOT NULL DEFAULT 'active',
            metadata      TEXT,
            created_at    TEXT NOT NULL,
            last_activity TEXT NOT NULL
        )",
        indexes: &[
            "CREATE INDEX IF NOT EXISTS idx_sessions_client ON sessions(client_id)",
            "CREATE INDEX IF NOT EXISTS idx_sessions_state  ON sessions(state)",
        ],
    },
    // ── audit_logs ────────────────────────────────────────────────────────────
    SchemaDefinition {
        table_name: "audit_logs",
        create_sql: "CREATE TABLE IF NOT EXISTS audit_logs (
            id          TEXT PRIMARY KEY,
            actor_id    TEXT NOT NULL,
            action      TEXT NOT NULL,
            resource    TEXT NOT NULL,
            details     TEXT,
            ip_address  TEXT,
            created_at  TEXT NOT NULL
        )",
        indexes: &[
            "CREATE INDEX IF NOT EXISTS idx_audit_actor      ON audit_logs(actor_id)",
            "CREATE INDEX IF NOT EXISTS idx_audit_created_at ON audit_logs(created_at)",
        ],
    },
    // ── messages ──────────────────────────────────────────────────────────────
    SchemaDefinition {
        table_name: "messages",
        create_sql: "CREATE TABLE IF NOT EXISTS messages (
            id          TEXT PRIMARY KEY,
            channel_id  TEXT NOT NULL,
            sender_id   TEXT NOT NULL,
            content     TEXT NOT NULL,
            content_type TEXT NOT NULL DEFAULT 'text',
            metadata    TEXT,
            created_at  TEXT NOT NULL,
            edited_at   TEXT
        )",
        indexes: &[
            "CREATE INDEX IF NOT EXISTS idx_messages_channel    ON messages(channel_id)",
            "CREATE INDEX IF NOT EXISTS idx_messages_sender     ON messages(sender_id)",
            "CREATE INDEX IF NOT EXISTS idx_messages_created_at ON messages(created_at)",
        ],
    },
    // ── channels ──────────────────────────────────────────────────────────────
    SchemaDefinition {
        table_name: "channels",
        create_sql: "CREATE TABLE IF NOT EXISTS channels (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            kind        TEXT NOT NULL DEFAULT 'public',
            owner_id    TEXT NOT NULL,
            metadata    TEXT,
            created_at  TEXT NOT NULL
        )",
        indexes: &[
            "CREATE INDEX IF NOT EXISTS idx_channels_owner ON channels(owner_id)",
            "CREATE INDEX IF NOT EXISTS idx_channels_kind  ON channels(kind)",
        ],
    },
    // ── clans ─────────────────────────────────────────────────────────────────
    SchemaDefinition {
        table_name: "clans",
        create_sql: "CREATE TABLE IF NOT EXISTS clans (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL UNIQUE,
            description TEXT,
            owner_id    TEXT NOT NULL,
            max_members INTEGER NOT NULL DEFAULT 100,
            metadata    TEXT,
            created_at  TEXT NOT NULL
        )",
        indexes: &[
            "CREATE INDEX IF NOT EXISTS idx_clans_owner ON clans(owner_id)",
        ],
    },
    // ── clan_members ──────────────────────────────────────────────────────────
    SchemaDefinition {
        table_name: "clan_members",
        create_sql: "CREATE TABLE IF NOT EXISTS clan_members (
            clan_id   TEXT NOT NULL,
            client_id TEXT NOT NULL,
            role      TEXT NOT NULL DEFAULT 'member',
            joined_at TEXT NOT NULL,
            PRIMARY KEY (clan_id, client_id)
        )",
        indexes: &[
            "CREATE INDEX IF NOT EXISTS idx_clan_members_client ON clan_members(client_id)",
        ],
    },
    // ── connection_history ────────────────────────────────────────────────────
    SchemaDefinition {
        table_name: "connection_history",
        create_sql: "CREATE TABLE IF NOT EXISTS connection_history (
            id            TEXT PRIMARY KEY,
            client_id     TEXT NOT NULL,
            session_id    TEXT,
            protocol      TEXT NOT NULL,
            remote_addr   TEXT NOT NULL,
            connected_at  TEXT NOT NULL,
            disconnected_at TEXT,
            bytes_sent    INTEGER NOT NULL DEFAULT 0,
            bytes_received INTEGER NOT NULL DEFAULT 0
        )",
        indexes: &[
            "CREATE INDEX IF NOT EXISTS idx_conn_history_client     ON connection_history(client_id)",
            "CREATE INDEX IF NOT EXISTS idx_conn_history_connected   ON connection_history(connected_at)",
        ],
    },
    // ── api_keys ──────────────────────────────────────────────────────────────
    SchemaDefinition {
        table_name: "api_keys",
        create_sql: "CREATE TABLE IF NOT EXISTS api_keys (
            id          TEXT PRIMARY KEY,
            key_hash    TEXT NOT NULL UNIQUE,
            client_id   TEXT NOT NULL,
            description TEXT,
            scopes      TEXT NOT NULL DEFAULT '[]',
            created_at  TEXT NOT NULL,
            expires_at  TEXT,
            last_used_at TEXT
        )",
        indexes: &[
            "CREATE INDEX IF NOT EXISTS idx_api_keys_client   ON api_keys(client_id)",
            "CREATE INDEX IF NOT EXISTS idx_api_keys_key_hash ON api_keys(key_hash)",
        ],
    },
    // ── config_snapshots ──────────────────────────────────────────────────────
    SchemaDefinition {
        table_name: "config_snapshots",
        create_sql: "CREATE TABLE IF NOT EXISTS config_snapshots (
            id          TEXT PRIMARY KEY,
            version     INTEGER NOT NULL,
            config_json TEXT NOT NULL,
            author      TEXT NOT NULL,
            created_at  TEXT NOT NULL
        )",
        indexes: &[
            "CREATE INDEX IF NOT EXISTS idx_config_snapshots_version ON config_snapshots(version)",
        ],
    },
    // ── plugin_state ──────────────────────────────────────────────────────────
    SchemaDefinition {
        table_name: "plugin_state",
        create_sql: "CREATE TABLE IF NOT EXISTS plugin_state (
            plugin_id   TEXT NOT NULL,
            key         TEXT NOT NULL,
            value       TEXT NOT NULL,
            updated_at  TEXT NOT NULL,
            PRIMARY KEY (plugin_id, key)
        )",
        indexes: &[
            "CREATE INDEX IF NOT EXISTS idx_plugin_state_plugin ON plugin_state(plugin_id)",
        ],
    },
];

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Look up a [`SchemaDefinition`] by `table_name`.
///
/// Returns `None` if no schema is registered for the given name.
pub fn find_schema(table_name: &str) -> Option<&'static SchemaDefinition> {
    SCHEMAS.iter().find(|s| s.table_name == table_name)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn memory_pool() -> sqlx::SqlitePool {
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("open in-memory SQLite")
    }

    #[test]
    fn test_schema_count() {
        // Ensure we have at least the 10 required tables.
        assert!(SCHEMAS.len() >= 10, "expected at least 10 table schemas, got {}", SCHEMAS.len());
    }

    #[test]
    fn test_table_names_are_unique() {
        let mut names: Vec<&str> = SCHEMAS.iter().map(|s| s.table_name).collect();
        names.dedup();
        assert_eq!(names.len(), SCHEMAS.len(), "duplicate table names found");
    }

    #[tokio::test]
    async fn test_all_schemas_execute_on_sqlite() {
        let pool = memory_pool().await;

        for schema in SCHEMAS {
            sqlx::query(schema.create_sql)
                .execute(&pool)
                .await
                .unwrap_or_else(|e| {
                    panic!("failed to create table '{}': {e}", schema.table_name)
                });

            for idx_sql in schema.indexes {
                sqlx::query(idx_sql)
                    .execute(&pool)
                    .await
                    .unwrap_or_else(|e| {
                        panic!(
                            "failed to create index for '{}': {e}\nSQL: {idx_sql}",
                            schema.table_name
                        )
                    });
            }
        }
    }

    #[test]
    fn test_find_schema_known() {
        let s = find_schema("sessions").expect("sessions schema should exist");
        assert_eq!(s.table_name, "sessions");
        assert!(s.create_sql.contains("PRIMARY KEY"), "sessions should have a PRIMARY KEY");
    }

    #[test]
    fn test_find_schema_unknown() {
        assert!(find_schema("nonexistent_table").is_none());
    }
}
