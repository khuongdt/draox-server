use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::debug;

/// A reference to a file that has been uploaded and associated with the messaging system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReference {
    /// Unique file identifier.
    pub id: String,
    /// Original filename.
    pub filename: String,
    /// MIME type (e.g. "image/png", "application/pdf").
    pub mime_type: String,
    /// File size in bytes.
    pub size_bytes: u64,
    /// Download URL (set after upload is complete).
    pub url: Option<String>,
    /// SHA-256 or similar checksum for integrity verification.
    pub checksum: Option<String>,
    /// ID of the user who uploaded the file.
    pub uploaded_by: String,
    /// Timestamp of upload.
    pub uploaded_at: DateTime<Utc>,
}

/// Registry that stores file metadata and tracks file-to-message associations.
pub struct FileRegistry {
    /// id -> FileReference
    files: DashMap<String, FileReference>,
    /// message_id -> list of file_ids attached to that message
    message_files: DashMap<String, Vec<String>>,
}

impl FileRegistry {
    pub fn new() -> Self {
        Self {
            files: DashMap::new(),
            message_files: DashMap::new(),
        }
    }

    /// Register a new file upload and return the created `FileReference`.
    pub fn register(
        &self,
        filename: &str,
        mime_type: &str,
        size: u64,
        uploader: &str,
    ) -> FileReference {
        let id = format!("file_{}", uuid::Uuid::new_v4().as_simple());
        let file_ref = FileReference {
            id: id.clone(),
            filename: filename.to_string(),
            mime_type: mime_type.to_string(),
            size_bytes: size,
            url: None,
            checksum: None,
            uploaded_by: uploader.to_string(),
            uploaded_at: Utc::now(),
        };
        self.files.insert(id.clone(), file_ref.clone());
        debug!(file_id = %id, filename = %filename, uploader = %uploader, "file registered");
        file_ref
    }

    /// Retrieve a file reference by its ID.
    pub fn get(&self, id: &str) -> Option<FileReference> {
        self.files.get(id).map(|r| r.value().clone())
    }

    /// Attach an existing file to a message.
    /// Returns `true` if the file exists and was attached, `false` if the file is unknown.
    pub fn attach_to_message(&self, file_id: &str, message_id: &str) -> bool {
        if !self.files.contains_key(file_id) {
            return false;
        }
        let mut entry = self
            .message_files
            .entry(message_id.to_string())
            .or_default();
        if !entry.contains(&file_id.to_string()) {
            entry.push(file_id.to_string());
        }
        debug!(file_id = %file_id, message_id = %message_id, "file attached to message");
        true
    }

    /// Get all `FileReference`s attached to a message.
    pub fn get_message_files(&self, message_id: &str) -> Vec<FileReference> {
        self.message_files
            .get(message_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.files.get(id).map(|r| r.value().clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Delete a file from the registry (and any message associations).
    /// Returns `true` if the file existed and was removed.
    pub fn delete(&self, id: &str) -> bool {
        let removed = self.files.remove(id).is_some();
        if removed {
            // Remove this file_id from all message attachment lists.
            let file_id_str = id.to_string();
            for mut entry in self.message_files.iter_mut() {
                entry.retain(|fid| fid != &file_id_str);
            }
            debug!(file_id = %id, "file deleted");
        }
        removed
    }

    /// Calculate the total bytes uploaded by a specific user.
    pub fn total_size_by_user(&self, user_id: &str) -> u64 {
        self.files
            .iter()
            .filter(|r| r.value().uploaded_by == user_id)
            .map(|r| r.value().size_bytes)
            .sum()
    }

    /// Total number of registered files.
    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}

impl Default for FileRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_get() {
        let registry = FileRegistry::new();

        let file = registry.register("report.pdf", "application/pdf", 2048, "cli_alice");
        assert!(file.id.starts_with("file_"));
        assert_eq!(file.filename, "report.pdf");
        assert_eq!(file.mime_type, "application/pdf");
        assert_eq!(file.size_bytes, 2048);
        assert_eq!(file.uploaded_by, "cli_alice");
        assert!(file.url.is_none());
        assert!(file.checksum.is_none());

        let fetched = registry.get(&file.id).unwrap();
        assert_eq!(fetched.id, file.id);
        assert!(registry.get("file_nonexistent").is_none());
    }

    #[test]
    fn test_attach_to_message_and_get_files() {
        let registry = FileRegistry::new();

        let f1 = registry.register("photo.png", "image/png", 512, "cli_alice");
        let f2 = registry.register("doc.txt", "text/plain", 128, "cli_alice");

        assert!(registry.attach_to_message(&f1.id, "msg_001"));
        assert!(registry.attach_to_message(&f2.id, "msg_001"));

        // Unknown file_id returns false
        assert!(!registry.attach_to_message("file_unknown", "msg_001"));

        let files = registry.get_message_files("msg_001");
        assert_eq!(files.len(), 2);

        // No attachments for a different message
        assert!(registry.get_message_files("msg_999").is_empty());
    }

    #[test]
    fn test_delete_file_removes_from_messages() {
        let registry = FileRegistry::new();

        let f = registry.register("image.jpg", "image/jpeg", 1024, "cli_bob");
        registry.attach_to_message(&f.id, "msg_010");

        assert_eq!(registry.get_message_files("msg_010").len(), 1);

        assert!(registry.delete(&f.id));
        assert!(registry.get(&f.id).is_none());

        // Attachment list should now be empty for that message
        assert!(registry.get_message_files("msg_010").is_empty());

        // Deleting a non-existent file returns false
        assert!(!registry.delete("file_nonexistent"));
    }

    #[test]
    fn test_total_size_by_user() {
        let registry = FileRegistry::new();

        registry.register("a.txt", "text/plain", 100, "cli_alice");
        registry.register("b.txt", "text/plain", 200, "cli_alice");
        registry.register("c.txt", "text/plain", 50, "cli_bob");

        assert_eq!(registry.total_size_by_user("cli_alice"), 300);
        assert_eq!(registry.total_size_by_user("cli_bob"), 50);
        assert_eq!(registry.total_size_by_user("cli_charlie"), 0);
    }

    #[test]
    fn test_no_duplicate_attachment() {
        let registry = FileRegistry::new();

        let f = registry.register("file.bin", "application/octet-stream", 8, "cli_alice");
        registry.attach_to_message(&f.id, "msg_002");
        registry.attach_to_message(&f.id, "msg_002"); // duplicate

        let files = registry.get_message_files("msg_002");
        assert_eq!(files.len(), 1, "file should only appear once");
    }
}
