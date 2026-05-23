//! Cache error helpers.
//!
//! The in-memory moka backend is essentially infallible, but this module
//! provides conversion utilities for future backends (e.g. Redis) that can
//! produce I/O or protocol errors.

use server_core::Error;

/// Convert an arbitrary error message into a [`server_core::Error::Cache`].
pub fn cache_error(msg: impl Into<String>) -> Error {
    Error::Cache(msg.into())
}
