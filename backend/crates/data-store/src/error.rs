use server_core::Error;

/// Helper function to convert a `sqlx::Error` into `server_core::Error::Storage`.
///
/// We cannot use `#[from]` in server_core because that crate should not depend on sqlx.
/// Instead, callers in this crate use `.map_err(into_storage_error)` on sqlx results.
pub(crate) fn into_storage_error(e: sqlx::Error) -> Error {
    Error::Storage(e.to_string())
}

/// Helper function to convert a `mongodb::error::Error` into `server_core::Error::Storage`.
pub(crate) fn into_mongo_error(e: mongodb::error::Error) -> Error {
    Error::Storage(e.to_string())
}
