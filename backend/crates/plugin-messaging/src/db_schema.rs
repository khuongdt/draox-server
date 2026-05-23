/// Database schemas for all tables owned by the messaging plugin.
///
/// Each entry is `(table_name, CREATE TABLE statement)`.
/// Statements use `IF NOT EXISTS` so they are safe to run on every startup.
/// The `sqlx` layer (or any compatible SQL executor) can apply these in order.
pub const MESSAGING_SCHEMAS: &[(&str, &str)] = &[
    (
        "messages",
        "CREATE TABLE IF NOT EXISTS messages (
            id              TEXT        PRIMARY KEY,
            message_type    TEXT        NOT NULL,
            from_id         TEXT        NOT NULL,
            to_id           TEXT        NOT NULL,
            content         TEXT        NOT NULL,
            content_type    TEXT        NOT NULL DEFAULT 'text',
            status          TEXT        NOT NULL DEFAULT 'sent',
            reply_to        TEXT        REFERENCES messages(id) ON DELETE SET NULL,
            edited          BOOLEAN     NOT NULL DEFAULT FALSE,
            edited_at       TIMESTAMPTZ,
            timestamp       TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
    ),
    (
        "channels",
        "CREATE TABLE IF NOT EXISTS channels (
            id              TEXT        PRIMARY KEY,
            name            TEXT        NOT NULL,
            description     TEXT        NOT NULL DEFAULT '',
            channel_type    TEXT        NOT NULL DEFAULT 'public',
            created_by      TEXT        NOT NULL,
            topic           TEXT        NOT NULL DEFAULT '',
            created_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
    ),
    (
        "channel_members",
        "CREATE TABLE IF NOT EXISTS channel_members (
            channel_id      TEXT        NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
            user_id         TEXT        NOT NULL,
            joined_at       TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (channel_id, user_id)
        )",
    ),
    (
        "read_receipts",
        "CREATE TABLE IF NOT EXISTS read_receipts (
            message_id      TEXT        NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
            reader_id       TEXT        NOT NULL,
            read_at         TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (message_id, reader_id)
        )",
    ),
    (
        "file_references",
        "CREATE TABLE IF NOT EXISTS file_references (
            id              TEXT        PRIMARY KEY,
            filename        TEXT        NOT NULL,
            mime_type       TEXT        NOT NULL,
            size_bytes      BIGINT      NOT NULL,
            url             TEXT,
            checksum        TEXT,
            uploaded_by     TEXT        NOT NULL,
            uploaded_at     TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
    ),
    (
        "message_files",
        "CREATE TABLE IF NOT EXISTS message_files (
            message_id      TEXT        NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
            file_id         TEXT        NOT NULL REFERENCES file_references(id) ON DELETE CASCADE,
            PRIMARY KEY (message_id, file_id)
        )",
    ),
    (
        "user_presence",
        "CREATE TABLE IF NOT EXISTS user_presence (
            user_id         TEXT        PRIMARY KEY,
            status          TEXT        NOT NULL DEFAULT 'offline',
            status_message  TEXT        NOT NULL DEFAULT '',
            last_seen       TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
    ),
    (
        "message_reactions",
        "CREATE TABLE IF NOT EXISTS message_reactions (
            message_id      TEXT        NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
            emoji           TEXT        NOT NULL,
            user_id         TEXT        NOT NULL,
            reacted_at      TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (message_id, emoji, user_id)
        )",
    ),
];

/// Returns the ordered list of table names that the messaging plugin manages.
pub fn table_names() -> Vec<&'static str> {
    MESSAGING_SCHEMAS.iter().map(|(name, _)| *name).collect()
}

/// Returns the `CREATE TABLE` SQL statement for a given table name, if it exists.
pub fn schema_for(table_name: &str) -> Option<&'static str> {
    MESSAGING_SCHEMAS
        .iter()
        .find(|(name, _)| *name == table_name)
        .map(|(_, sql)| *sql)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_schemas_non_empty_and_valid_sql_prefix() {
        assert!(
            !MESSAGING_SCHEMAS.is_empty(),
            "must define at least one schema"
        );

        for (table_name, sql) in MESSAGING_SCHEMAS {
            assert!(
                !table_name.is_empty(),
                "table name must not be empty"
            );
            let upper = sql.to_uppercase();
            assert!(
                upper.starts_with("CREATE TABLE IF NOT EXISTS"),
                "schema for '{}' must start with CREATE TABLE IF NOT EXISTS",
                table_name
            );
            assert!(
                upper.contains(&table_name.to_uppercase()),
                "schema for '{}' must reference the table name",
                table_name
            );
        }
    }

    #[test]
    fn test_required_tables_present() {
        let names = table_names();
        for required in &["messages", "channels", "channel_members", "read_receipts", "file_references", "user_presence"] {
            assert!(
                names.contains(required),
                "required table '{}' not found in MESSAGING_SCHEMAS",
                required
            );
        }
    }

    #[test]
    fn test_schema_for_lookup() {
        let sql = schema_for("messages").expect("messages schema must exist");
        assert!(sql.to_uppercase().contains("CREATE TABLE IF NOT EXISTS MESSAGES"));

        assert!(
            schema_for("nonexistent_table").is_none(),
            "unknown table should return None"
        );
    }
}
