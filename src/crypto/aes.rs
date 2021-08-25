use std::str::FromStr;

use aes::Aes256;
use block_modes::block_padding::Iso7816;
use block_modes::{BlockMode, Cbc};
use secrecy::{ExposeSecret, SecretString};

use crate::error::InquestError;
use crate::Result;

// taken from https://markv.nl/blog/symmetric-encryption-in-rust

/// Set up the cipher.
/// Cbc means each block affects the next, which is more secure than Ecb.
/// Aes256 is the actual encryption/decryption algorithm.
/// Iso7816 is the padding using if the data doesn't fit in the block size.
type Aes256Cbc = Cbc<Aes256, Iso7816>;

struct AesCrypto {
    cipher: Aes256Cbc,
}

impl AesCrypto {
    /// Initializes the AesCrypto struct with a ready-to-use cipher.
    /// The encryption key is either passed in or left empty in which case a default key is used.
    fn new(key: Option<SecretString>) -> Result<AesCrypto> {
        // Key must be 32 bytes for Aes256. It should probably be the hashed
        // version of the input key, so is not limited to printable ascii.
        let key = match key {
            None => SecretString::from_str("RvzQW3Mwrc!_y5-DpPZl8rP3,=HsD1,!").unwrap(),
            Some(key) => {
                if key.expose_secret().len() < 10 || key.expose_secret().len() > 32 {
                    return Err(InquestError::BadCryptoKeyError {
                        length: key.expose_secret().len(),
                    });
                }
                // lef-pad the key so it is 32 characters long
                SecretString::new(format!("{:0>32}", key.expose_secret()))
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

        Ok(AesCrypto { cipher })
    }

    /// Encrypt the given String to a byte-array. The result will not be well-formed UTF-8,
    /// so it cannot be converted to String or &str.
    fn encrypt(self, text: SecretString) -> Vec<u8> {
        self.cipher.encrypt_vec(&text.expose_secret().as_bytes())
    }

    /// Decrypts the given byte-array back to another byte-array. The result, if used with the proper
    /// key, is well-formed UTF-8 and can be converted to String or &str.
    fn decrypt(self, encrypted: Vec<u8>) -> Result<Vec<u8>> {
        Ok(self.cipher.decrypt_vec(&encrypted)?)
    }
}

/// Encrypts the given String with an AES Block-Cypher and Base64 encodes the resulting bytes
/// in order to have a well-formed UTF-8 String to return
pub fn encrypt(text: SecretString, key: Option<SecretString>) -> Result<String> {
    let crypto = AesCrypto::new(key);
    Ok(base64::encode(crypto?.encrypt(text)))
}

/// Decrypts the given String by first reverting the Base64 encoding to the former bytes which
/// are then decrypted with the former AES Block-Cypher
pub fn decrypt(encrypted: String, key: Option<SecretString>) -> Result<String> {
    let crypto = AesCrypto::new(key);
    let text = crypto?.decrypt(base64::decode(encrypted)?);
    Ok(String::from_utf8(text?)?)
}

#[cfg(test)]
mod tests {
    use secrecy::SecretString;

    use crate::crypto::aes::encrypt;
    use crate::error::InquestError;

    #[test]
    // #[should_panic(expected="Key must consist of 32 characters!")]
    fn encrypt_fails_on_key_to_short() {
        let key = "to short"; // less than 10
        let result = encrypt(
            SecretString::new("hello world".to_string()),
            Some(SecretString::new(key.to_string())),
        );
        assert_matches!(result, Err(InquestError::BadCryptoKeyError {length}) if length == key.len());
    }

    #[test]
    // #[should_panic(expected="Key must consist of 32 characters!")]
    fn encrypt_fails_on_key_to_long() {
        let key = format!("{:0>33}", "key"); // key will be to long by left-padding it to 33 characters
        let result = encrypt(
            SecretString::new("hello world".to_string()),
            Some(SecretString::new(key.clone())),
        );
        assert_matches!(result, Err(InquestError::BadCryptoKeyError {length}) if length == key.len());
    }
}
