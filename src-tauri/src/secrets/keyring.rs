use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex, PoisonError};

use keyring::Entry;
use secrecy::{ExposeSecret as _, SecretString};

use crate::error::{AppError, AppResult};

pub const KEYRING_SERVICE: &str = "com.foundergrowthlab.desktop";

pub trait SecretStore: Debug + Send + Sync {
    fn load(&self, account: &str) -> AppResult<Option<SecretString>>;
    fn save(&self, account: &str, value: &SecretString) -> AppResult<()>;
    fn delete(&self, account: &str) -> AppResult<bool>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct OsSecretStore;

impl SecretStore for OsSecretStore {
    fn load(&self, account: &str) -> AppResult<Option<SecretString>> {
        let entry = Entry::new(KEYRING_SERVICE, account)
            .map_err(|error| AppError::Credential(error.to_string()))?;
        match entry.get_password() {
            Ok(value) => Ok(Some(SecretString::from(value))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(AppError::Credential(error.to_string())),
        }
    }

    fn save(&self, account: &str, value: &SecretString) -> AppResult<()> {
        let entry = Entry::new(KEYRING_SERVICE, account)
            .map_err(|error| AppError::Credential(error.to_string()))?;
        entry
            .set_password(value.expose_secret())
            .map_err(|error| AppError::Credential(error.to_string()))
    }

    fn delete(&self, account: &str) -> AppResult<bool> {
        let entry = Entry::new(KEYRING_SERVICE, account)
            .map_err(|error| AppError::Credential(error.to_string()))?;
        match entry.delete_credential() {
            Ok(()) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(error) => Err(AppError::Credential(error.to_string())),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MemorySecretStore {
    values: Arc<Mutex<HashMap<String, String>>>,
}

impl SecretStore for MemorySecretStore {
    fn load(&self, account: &str) -> AppResult<Option<SecretString>> {
        let values = self.values.lock().unwrap_or_else(PoisonError::into_inner);
        Ok(values.get(account).cloned().map(SecretString::from))
    }

    fn save(&self, account: &str, value: &SecretString) -> AppResult<()> {
        let mut values = self.values.lock().unwrap_or_else(PoisonError::into_inner);
        values.insert(account.to_owned(), value.expose_secret().to_owned());
        Ok(())
    }

    fn delete(&self, account: &str) -> AppResult<bool> {
        let mut values = self.values.lock().unwrap_or_else(PoisonError::into_inner);
        Ok(values.remove(account).is_some())
    }
}

#[cfg(test)]
mod tests {
    use secrecy::{ExposeSecret as _, SecretString};

    use super::{MemorySecretStore, SecretStore};

    #[test]
    fn memory_store_round_trips_and_deletes() {
        let store = MemorySecretStore::default();
        store
            .save("x:one", &SecretString::from("sentinel".to_owned()))
            .expect("save");
        assert_eq!(
            store
                .load("x:one")
                .expect("load")
                .expect("value")
                .expose_secret(),
            "sentinel"
        );
        assert!(store.delete("x:one").expect("delete"));
        assert!(store.load("x:one").expect("load after delete").is_none());
    }
}
