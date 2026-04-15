use base64::Engine;

mod constants;
mod error;
mod id;
pub mod prelude;
mod route;

pub trait GroupshopCodec: Sized + serde::Serialize + serde::de::DeserializeOwned {
    fn encode(&self) -> Result<Vec<u8>, error::ApiError> {
        serde_json::to_vec(self).map_err(|err| error::ApiError::ParseBody(err.to_string()))
    }

    fn encode_str(&self) -> Result<String, error::ApiError> {
        Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(self.encode()?))
    }

    fn decode(bytes: &[u8]) -> Result<Self, error::ApiError> {
        serde_json::from_slice(bytes).map_err(|err| error::ApiError::ParseBody(err.to_string()))
    }

    fn decode_str(value: &str) -> Result<Self, error::ApiError> {
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(value)
            .map_err(|err| error::ApiError::Base64Decode(err.to_string()))?;

        Self::decode(&bytes)
    }
}

impl<T> GroupshopCodec for T where T: Sized + serde::Serialize + serde::de::DeserializeOwned {}
