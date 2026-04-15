#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum UserRole {
    PartialRegistration = 1,
    EmailVerified = 2,
    UsernameChosen = 3,
    Admin = 4,
}

impl UserRole {
    pub const fn basic() -> [UserRole; 3] {
        [
            Self::PartialRegistration,
            Self::EmailVerified,
            Self::UsernameChosen,
        ]
    }

    pub const fn admin() -> [UserRole; 4] {
        [
            Self::PartialRegistration,
            Self::EmailVerified,
            Self::UsernameChosen,
            Self::Admin,
        ]
    }
}

impl From<u8> for UserRole {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::PartialRegistration,
            2 => Self::EmailVerified,
            3 => Self::UsernameChosen,
            4 => Self::Admin,
            _ => panic!("invalid user role"),
        }
    }
}

impl From<UserRole> for u8 {
    fn from(value: UserRole) -> Self {
        value as u8
    }
}
