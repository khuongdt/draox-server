/// Presigned URL request for client-direct uploads.
#[derive(Debug, Clone)]
pub struct PresignedUploadRequest {
    pub key: String,
    pub bucket: String,
    pub content_type: String,
    pub max_size_bytes: Option<u64>,
    pub expires_secs: u64,
    pub owner: Option<String>,
}

/// Presigned URL response to return to clients.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PresignedUploadResponse {
    pub upload_url: String,
    pub key: String,
    pub bucket: String,
    pub expires_in_secs: u64,
    pub content_type: String,
}

/// Validate that a content-type is allowed for upload.
pub fn validate_content_type(content_type: &str, allowed: &[&str]) -> bool {
    if allowed.is_empty() {
        return true;
    }
    allowed.iter().any(|&a| {
        if a.ends_with("/*") {
            let prefix = &a[..a.len() - 1];
            content_type.starts_with(prefix)
        } else {
            a == content_type
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wildcard_content_type() {
        assert!(validate_content_type("image/png", &["image/*"]));
        assert!(validate_content_type("image/jpeg", &["image/*"]));
        assert!(!validate_content_type("application/pdf", &["image/*"]));
    }

    #[test]
    fn test_exact_content_type() {
        assert!(validate_content_type("video/mp4", &["video/mp4", "video/webm"]));
        assert!(!validate_content_type("video/avi", &["video/mp4", "video/webm"]));
    }

    #[test]
    fn test_empty_allowed_list() {
        assert!(validate_content_type("anything/goes", &[]));
    }
}
