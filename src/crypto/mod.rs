use crate::Result;
use crate::crypto::aes::{encrypt, decrypt};
use secrecy::{SecretString, ExposeSecret};

mod aes;

const VAULT_PREFIX: &'static str = "!vault |";

/// TODO
pub enum VaultTypes {
    Aes256,
}


pub fn encrypt_secret(text: SecretString, key: Option<&str>) -> Result<String> {
    Ok(format!("{}{}", VAULT_PREFIX, encrypt(text, key)?))
}

pub fn decrypt_secret(
    secret: SecretString,
    key: Option<&str>,
) -> Result<SecretString> {
    if !secret.expose_secret().starts_with(VAULT_PREFIX) {
        Ok(secret)
    }else {
        let secret_without_prefix = &secret.expose_secret()[VAULT_PREFIX.len()..];
        Ok(SecretString::new(decrypt(
            String::from(secret_without_prefix),
            key,
        )?))
    }
}

#[cfg(test)]
mod tests {
    use secrecy::{SecretString, ExposeSecret};
    use crate::crypto::{encrypt_secret, decrypt_secret};

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
        let key = Some("RvzQW3MwrcDpPZl8rP3,=HsD1,wdgdew");
        let encrypted = encrypt_secret(SecretString::new(text.clone()), key).unwrap();
        let decrypted = decrypt_secret(SecretString::new(encrypted), key).unwrap();
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