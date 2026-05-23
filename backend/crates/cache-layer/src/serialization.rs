//! Multiple serialization formats for cache values.
//!
//! All three serializers implement [`CacheSerializer`] and can be used
//! interchangeably to encode/decode typed values before storing them in a
//! [`CacheBackend`].
//!
//! | Serializer            | Format        | Crate       |
//! |-----------------------|---------------|-------------|
//! | [`JsonSerializer`]    | JSON (UTF-8)  | `serde_json` |
//! | [`BincodeSerializer`] | Bincode v1    | `bincode`   |
//! | [`MessagePackSerializer`] | MessagePack | `rmp-serde` |

use server_core::{Error, Result};

// ── SerializationFormat ───────────────────────────────────────────────────────

/// Discriminant that identifies which wire format is in use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerializationFormat {
    /// JSON text encoding (UTF-8).
    Json,
    /// Bincode binary encoding.
    Bincode,
    /// MessagePack binary encoding.
    MessagePack,
}

// ── CacheSerializer trait ─────────────────────────────────────────────────────

/// Trait for converting typed values to/from raw bytes suitable for storage
/// in a [`CacheBackend`].
pub trait CacheSerializer: Send + Sync {
    /// Encode `value` into bytes.
    fn serialize<T: serde::Serialize>(&self, value: &T) -> Result<Vec<u8>>;

    /// Decode bytes back into a `T`.
    fn deserialize<T: serde::de::DeserializeOwned>(&self, data: &[u8]) -> Result<T>;

    /// The wire format used by this serializer.
    fn format(&self) -> SerializationFormat;
}

// ── JsonSerializer ────────────────────────────────────────────────────────────

/// Serializes values as JSON (UTF-8 text).
///
/// Human-readable and easy to inspect, but larger than binary formats.
pub struct JsonSerializer;

impl CacheSerializer for JsonSerializer {
    fn serialize<T: serde::Serialize>(&self, value: &T) -> Result<Vec<u8>> {
        serde_json::to_vec(value).map_err(|e| Error::Cache(e.to_string()))
    }

    fn deserialize<T: serde::de::DeserializeOwned>(&self, data: &[u8]) -> Result<T> {
        serde_json::from_slice(data).map_err(|e| Error::Cache(e.to_string()))
    }

    fn format(&self) -> SerializationFormat {
        SerializationFormat::Json
    }
}

// ── BincodeSerializer ─────────────────────────────────────────────────────────

/// Serializes values using the Bincode v1 binary format.
///
/// Compact and fast; not human-readable. Requires that the type is
/// `serde::Serialize + serde::de::DeserializeOwned`.
pub struct BincodeSerializer;

impl CacheSerializer for BincodeSerializer {
    fn serialize<T: serde::Serialize>(&self, value: &T) -> Result<Vec<u8>> {
        bincode::serialize(value).map_err(|e| Error::Cache(e.to_string()))
    }

    fn deserialize<T: serde::de::DeserializeOwned>(&self, data: &[u8]) -> Result<T> {
        bincode::deserialize(data).map_err(|e| Error::Cache(e.to_string()))
    }

    fn format(&self) -> SerializationFormat {
        SerializationFormat::Bincode
    }
}

// ── MessagePackSerializer ─────────────────────────────────────────────────────

/// Serializes values using the MessagePack binary format via `rmp-serde`.
///
/// Compact binary format with broad ecosystem support; a good middle ground
/// between JSON readability and Bincode compactness.
pub struct MessagePackSerializer;

impl CacheSerializer for MessagePackSerializer {
    fn serialize<T: serde::Serialize>(&self, value: &T) -> Result<Vec<u8>> {
        rmp_serde::to_vec(value).map_err(|e| Error::Cache(e.to_string()))
    }

    fn deserialize<T: serde::de::DeserializeOwned>(&self, data: &[u8]) -> Result<T> {
        rmp_serde::from_slice(data).map_err(|e| Error::Cache(e.to_string()))
    }

    fn format(&self) -> SerializationFormat {
        SerializationFormat::MessagePack
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct Payload {
        id: u32,
        name: String,
        active: bool,
        scores: Vec<f64>,
    }

    fn sample() -> Payload {
        Payload {
            id: 42,
            name: "draox".to_string(),
            active: true,
            scores: vec![1.5, 2.0, 3.7],
        }
    }

    #[test]
    fn test_json_serializer_roundtrip() {
        let ser = JsonSerializer;
        assert_eq!(ser.format(), SerializationFormat::Json);

        let bytes = ser.serialize(&sample()).unwrap();
        assert!(!bytes.is_empty());

        let decoded: Payload = ser.deserialize(&bytes).unwrap();
        assert_eq!(decoded, sample());
    }

    #[test]
    fn test_bincode_serializer_roundtrip() {
        let ser = BincodeSerializer;
        assert_eq!(ser.format(), SerializationFormat::Bincode);

        let bytes = ser.serialize(&sample()).unwrap();
        assert!(!bytes.is_empty());

        let decoded: Payload = ser.deserialize(&bytes).unwrap();
        assert_eq!(decoded, sample());
    }

    #[test]
    fn test_msgpack_serializer_roundtrip() {
        let ser = MessagePackSerializer;
        assert_eq!(ser.format(), SerializationFormat::MessagePack);

        let bytes = ser.serialize(&sample()).unwrap();
        assert!(!bytes.is_empty());

        let decoded: Payload = ser.deserialize(&bytes).unwrap();
        assert_eq!(decoded, sample());
    }

    #[test]
    fn test_binary_serializers_are_more_compact_than_json_for_string_heavy_data() {
        // JSON repeats field names for every entry, making it more verbose than
        // binary formats for collections of structs with long field names.
        #[derive(Serialize, Deserialize)]
        struct Row {
            long_field_name_alpha: String,
            long_field_name_beta: String,
        }

        let rows: Vec<Row> = (0..50)
            .map(|_| Row {
                long_field_name_alpha: "x".to_string(),
                long_field_name_beta: "y".to_string(),
            })
            .collect();

        let json_bytes = JsonSerializer.serialize(&rows).unwrap();
        let msgpack_bytes = MessagePackSerializer.serialize(&rows).unwrap();

        // MessagePack omits field name strings when using integer keys via
        // named-field encoding; it should be noticeably more compact here.
        assert!(
            msgpack_bytes.len() < json_bytes.len(),
            "msgpack ({} bytes) should be smaller than JSON ({} bytes) for struct arrays",
            msgpack_bytes.len(),
            json_bytes.len()
        );
    }

    #[test]
    fn test_json_deserialize_error_on_bad_input() {
        let ser = JsonSerializer;
        let result: Result<Payload> = ser.deserialize(b"not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_bincode_deserialize_error_on_bad_input() {
        let ser = BincodeSerializer;
        // Try to deserialize bytes that aren't a valid Payload encoding.
        let result: Result<Payload> = ser.deserialize(b"\xFF\xFF\xFF");
        assert!(result.is_err());
    }
}
