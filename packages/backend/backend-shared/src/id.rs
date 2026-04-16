use base64::Engine;
use serde::{Deserialize, Deserializer, Serialize};
use std::str::FromStr;

// This macro generates a newtype struct that wraps a 32-byte array, along with common trait implementations for convenience.
// String representation uses URL-safe base64 (no padding).
macro_rules! new_hash_id_type {
    ($type_name:ident) => {
        #[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
        pub struct $type_name([u8; 32]);

        impl $type_name {
            pub fn inner(&self) -> [u8; 32] {
                self.0
            }

            pub fn encode_str(&self) -> String {
                base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(self.0)
            }

            pub fn from_encoded_str(s: &str) -> Result<Self, String> {
                let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .decode(s)
                    .map_err(|e| format!("base64 decode error: {e}"))?;
                let arr: [u8; 32] = bytes
                    .try_into()
                    .map_err(|v: Vec<u8>| format!("expected 32 bytes, got {}", v.len()))?;
                Ok($type_name(arr))
            }
        }

        impl From<[u8; 32]> for $type_name {
            fn from(value: [u8; 32]) -> Self {
                Self(value)
            }
        }

        impl AsRef<[u8]> for $type_name {
            fn as_ref(&self) -> &[u8] {
                &self.0
            }
        }

        impl std::fmt::Display for $type_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.encode_str())
            }
        }

        impl std::fmt::Debug for $type_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self)
            }
        }

        impl FromStr for $type_name {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::from_encoded_str(s)
            }
        }

        impl Serialize for $type_name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(&self.encode_str())
            }
        }

        impl<'de> Deserialize<'de> for $type_name {
            fn deserialize<D>(deserializer: D) -> Result<$type_name, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct StrVisitor;

                impl<'de> serde::de::Visitor<'de> for StrVisitor {
                    type Value = $type_name;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        formatter.write_str("expected base64url-encoded string")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
                    where
                        E: serde::de::Error,
                    {
                        $type_name::from_encoded_str(value).map_err(serde::de::Error::custom)
                    }
                }

                deserializer.deserialize_str(StrVisitor)
            }
        }
    };
}

macro_rules! new_string_id_type {
    ($type_name:ident) => {
        #[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $type_name(String);

        impl $type_name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $type_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.as_str())
            }
        }

        impl std::fmt::Debug for $type_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.as_str())
            }
        }
    };
}

macro_rules! new_slug_id_type {
    ($type_name:ident) => {
        #[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $type_name(String);

        impl $type_name {
            pub fn new(value: impl Into<String>) -> Result<Self, String> {
                let slug = value.into().trim().to_ascii_lowercase();

                if slug.is_empty() {
                    return Err(format!("{} cannot be empty", stringify!($type_name)));
                }
                if slug.starts_with('-') || slug.ends_with('-') {
                    return Err(format!(
                        "{} must not start or end with a hyphen",
                        stringify!($type_name)
                    ));
                }
                if slug.contains("--") {
                    return Err(format!(
                        "{} must not contain consecutive hyphens",
                        stringify!($type_name)
                    ));
                }
                if !slug
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
                {
                    return Err(format!(
                        "{} may only contain lowercase letters, digits, and hyphens",
                        stringify!($type_name)
                    ));
                }

                Ok(Self(slug))
            }

            pub fn unchecked_new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $type_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl std::fmt::Debug for $type_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}(\"{}\")", stringify!($type_name), &self.0)
            }
        }

        impl FromStr for $type_name {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::new(s)
            }
        }
    };
}

new_hash_id_type!(UserId);
new_string_id_type!(AuthTokenId);
new_hash_id_type!(AuthTokenValue);
new_hash_id_type!(AuthTokenValueHash);
new_hash_id_type!(AuthTokenSignature);
new_slug_id_type!(AccountUsername);
new_slug_id_type!(ProductId);
new_slug_id_type!(ProductCategoryId);
new_slug_id_type!(ProductBrandId);
