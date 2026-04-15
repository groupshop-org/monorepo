use crate::id::{AuthTokenId, AuthTokenSignature, AuthTokenValue, UserId};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthToken {
    pub claims: AuthTokenClaims,
    pub signature: AuthTokenSignature,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AuthTokenClaims {
    Session {
        uid: UserId,
        token_value: AuthTokenValue,
        expires_at: u64,
    },
    Refresh {
        token_id: AuthTokenId,
        token_value: AuthTokenValue,
    },
    Once {
        token_id: AuthTokenId,
        token_value: AuthTokenValue,
    },
}
