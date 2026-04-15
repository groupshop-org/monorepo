use crate::error::{FrontendError, FrontendResult};

fn local_storage() -> FrontendResult<web_sys::Storage> {
    web_sys::window()
        .ok_or(FrontendError::StorageUnavailable)?
        .local_storage()
        .map_err(|_| FrontendError::StorageUnavailable)?
        .ok_or(FrontendError::StorageUnavailable)
}

pub fn set_local_storage(key: &str, value: &str) -> FrontendResult<()> {
    local_storage()?
        .set_item(key, value)
        .map_err(|_| FrontendError::StorageUnavailable)
}

pub fn get_local_storage(key: &str) -> FrontendResult<Option<String>> {
    local_storage()?
        .get_item(key)
        .map_err(|_| FrontendError::StorageUnavailable)
}

pub fn delete_local_storage(key: &str) -> FrontendResult<()> {
    local_storage()?
        .remove_item(key)
        .map_err(|_| FrontendError::StorageUnavailable)
}

pub fn has_local_storage(key: &str) -> FrontendResult<bool> {
    Ok(get_local_storage(key)?.is_some())
}
