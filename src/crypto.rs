use std::str::FromStr;
use aes::Aes256;
use block_modes::{BlockMode, Cbc};
use block_modes::block_padding::Iso7816;
use secrecy::{ExposeSecret, SecretString};

use crate::core::Result;
use crate::core::error::InquestError;
use std::fmt::format;

// taken from https://markv.nl/blog/symmetric-encryption-in-rust

/// Set up the cipher.
/// Cbc means each block affects the next, which is more secure than Ecb.
/// Aes256 is the actual encryption/decryption algorithm.
/// Iso7816 is the padding using if the data doesn't fit in the block size.
type Aes256Cbc = Cbc<Aes256, Iso7816>;

struct Crypto {
    cipher: Aes256Cbc,
}

impl Crypto {
    /// Initializes the Crypto struct with a ready-to-use cipher.
    /// The encryption key is either passed in or left empty in which case a default key is used.
    fn new(key: Option<&str>) -> Result<Crypto> {
        // Key must be 32 bytes for Aes256. It should probably be the hashed
        // version of the input key, so is not limited to printable ascii.
        let key = match key {
            None => SecretString::from_str("RvzQW3Mwrc!_y5-DpPZl8rP3,=HsD1,!").unwrap(),
            Some(key) => {
                println!("{}", key.len());
                if key.len() < 10 || key.len() > 32 {
                    return Err(InquestError::BadCryptoKeyError { length: key.len() });
                }
                SecretString::from_str(format!("{:0>32}", key).as_str()).unwrap()
            }
        };

        // The initialization vector (like salt or nonce) must be 16 bytes for
        // this block size. It could be generated using a secure random generator,
        // and should be different each time. It is not a secret.
        let initialization_vector: Vec<u8> = vec![
            89, 63, 254, 34, 209, 155, 236, 158, 195, 104, 11, 16, 240, 4, 26, 76,
        ];

        // Fails if the key or initialization_vector are the wrong length, so it is safe to unwrap
        // as we have the correct lengths. Key length depends on algorithm, iv length
        // depends on the block size. If it's not documented, experiment with 16 or 32.
        let cipher =
            Aes256Cbc::new_var(key.expose_secret().as_bytes(), &initialization_vector).unwrap();

        Ok(Crypto { cipher })
    }

    /// Encrypt the given String to a byte-array. The result will not be well-formed UTF-8,
    /// so it cannot be converted to String or &str.
    fn encrypt(self, text: String) -> Vec<u8> {
        self.cipher.encrypt_vec(&text.as_bytes())
    }

    /// Decrypts the given byte-array back to another byte-array. The result, if used with the proper
    /// key, is well-formed UTF-8 and can be converted to String or &str.
    fn decrypt(self, encrypted: Vec<u8>) -> Result<Vec<u8>> {
        Ok(self.cipher.decrypt_vec(&encrypted)?)
    }
}

/// Encrypts the given String with an AES Block-Cypher and Base64 encodes the resulting bytes
/// in order to have a well-formed UTF-8 String to return
pub fn encrypt(text: String, key: Option<&str>) -> Result<String> {
    let crypto = Crypto::new(key);
    Ok(base64::encode(crypto?.encrypt(text)))
}

/// Decrypts the given String by first reverting the Base64 encoding to the former bytes which
/// are then decrypted with the former AES Block-Cypher
pub fn decrypt(encrypted: String, key: Option<&str>) -> Result<String> {
    let crypto = Crypto::new(key);
    let text = crypto?.decrypt(base64::decode(encrypted)?);
    Ok(String::from_utf8(text?)?)
}

pub fn decrypt_secret(
    secret: SecretString,
    key: Option<&str>,
) -> Result<SecretString> {
    Ok(SecretString::new(decrypt(
        String::from(secret.expose_secret()),
        key,
    )?))
}

#[cfg(test)]
mod tests {
    use crate::crypto::{encrypt, decrypt};
    use crate::core::error::InquestError;

    #[test]
    fn encryption() {
        let text = "hello world".to_string();
        let encrypted = encrypt(text.clone(), None).unwrap();
        let decrypted = decrypt(encrypted, None).unwrap();
        assert_eq!(decrypted, text)
    }

    #[test]
    fn encryption_with_key() {
        let text = "hello world".to_string();
        let key = Some("RvzQW3MwrcDpPZl8rP3,=HsD1,wdgdew");
        let encrypted = encrypt(text.clone(), key).unwrap();
        let decrypted = decrypt(encrypted, key).unwrap();
        assert_eq!(decrypted, text)
    }

    #[test]
    // #[should_panic(expected="Key must consist of 32 characters!")]
    fn encryption_with_key_short() {
        let key = Some("to short"); // less than 10
        let mut result = false;
        if let Err(InquestError::BadCryptoKeyError { .. }) = encrypt("hello world".to_string(), key)
        {
            result = true;
        };
        assert_eq!(true, result, "Expected an InquestError::BadCryptoKeyError")
        // encrypt("hello world".to_string(), key).unwrap();
    }
}