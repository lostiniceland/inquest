use secrecy::{ExposeSecret, SecretString};

use crate::crypto::aes::{decrypt, encrypt};
use crate::Result;

mod aes;

const VAULT_PREFIX: &str = "!vault |";

/// TODO
pub enum VaultTypes {
    Aes256,
}

pub fn encrypt_secret(text: SecretString, key: Option<SecretString>) -> Result<String> {
    Ok(format!("{}{}", VAULT_PREFIX, encrypt(text, key)?))
}

pub fn decrypt_secret(secret: SecretString, key: Option<SecretString>) -> Result<SecretString> {
    if !secret.expose_secret().starts_with(VAULT_PREFIX) {
        Ok(secret)
    } else {
        let secret_without_prefix = &secret.expose_secret()[VAULT_PREFIX.len()..];
        Ok(SecretString::new(decrypt(
            String::from(secret_without_prefix),
            key,
        )?))
    }
}

#[cfg(test)]
mod tests {
    use secrecy::{ExposeSecret, SecretString};

    use crate::crypto::{decrypt_secret, encrypt_secret};

    #[test]
    fn aes_encryption_and_decryption() {
        let text = "hello world".to_string();
        let encrypted = encrypt_secret(SecretString::new(text.clone()), None).unwrap();
        let decrypted = decrypt_secret(SecretString::new(encrypted), None).unwrap();
        assert_eq!(decrypted.expose_secret().to_string(), text)
    }

    #[test]
    fn aes_encryption_and_decryption_with_key() {
        let text = "hello world".to_string();
        let key = Some(SecretString::new(
            "RvzQW3MwrcDpPZl8rP3,=HsD1,wdgdew".to_string(),
        ));
        let encrypted = encrypt_secret(SecretString::new(text.clone()), key.clone()).unwrap();
        let decrypted = decrypt_secret(SecretString::new(encrypted), key.clone()).unwrap();
        assert_eq!(decrypted.expose_secret().to_string(), text)
    }

    /// In this test, the string is not prefixed, and thus interpreted as unencrypted
    #[test]
    fn unencrypted_value_returned() {
        let text = "unencrypted text".to_string();
        let decrypted = decrypt_secret(SecretString::new(text.clone()), None).unwrap();
        assert_eq!(decrypted.expose_secret().to_string(), text)
    }
}
