//! Secrets that are handed out once and never stored.
//!
//! Sign-in links and session cookies both work the same way: generate enough
//! randomness that guessing is hopeless, give the raw value to exactly one
//! person, and keep only its SHA-256. Reading the database therefore lets
//! nobody sign in as anybody.
//!
//! SHA-256 rather than a password hash on purpose. Argon2 and friends exist
//! to make *low-entropy* secrets expensive to guess; these are 256 random
//! bits, so there is nothing to guess and a slow hash would only make every
//! request slower.

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::TryRng as _;
use rand::rngs::{SysError, SysRng};
use sha2::{Digest as _, Sha256};

/// How many random bytes each token carries.
const TOKEN_BYTES: usize = 32;

/// A freshly minted secret: the part that is sent, and the part that is kept.
pub struct Token {
    /// Goes in the email link or the cookie. Never stored, never logged.
    pub secret: String,
    /// Goes in the database.
    pub hash: Vec<u8>,
}

impl Token {
    /// Mint one, using the operating system's randomness.
    ///
    /// # Errors
    ///
    /// Returns an error if the system refuses to supply randomness, which
    /// must fail loudly rather than fall back to something weaker.
    pub fn generate() -> Result<Self, SysError> {
        let mut bytes = [0_u8; TOKEN_BYTES];
        SysRng.try_fill_bytes(&mut bytes)?;

        let secret = URL_SAFE_NO_PAD.encode(bytes);
        let hash = hash(&secret);

        Ok(Self { secret, hash })
    }
}

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The whole point is that this value never reaches a log.
        f.debug_struct("Token")
            .field("secret", &"<secret>")
            .finish()
    }
}

/// The stored form of a secret. Looking a token up means hashing what arrived
/// and finding that, so the database never holds anything usable.
#[must_use]
pub fn hash(secret: &str) -> Vec<u8> {
    Sha256::digest(secret.as_bytes()).to_vec()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    #[test]
    fn every_token_is_different() {
        let a = Token::generate().unwrap();
        let b = Token::generate().unwrap();

        assert_ne!(a.secret, b.secret);
        assert_ne!(a.hash, b.hash);
    }

    #[test]
    fn a_token_carries_256_bits_and_is_url_safe() {
        let token = Token::generate().unwrap();

        let decoded = URL_SAFE_NO_PAD.decode(&token.secret).expect("decode");
        assert_eq!(decoded.len(), TOKEN_BYTES);
        assert!(
            token
                .secret
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "a token goes in a URL: {}",
            token.secret
        );
    }

    #[test]
    fn the_stored_hash_matches_what_arrives() {
        let token = Token::generate().unwrap();

        assert_eq!(hash(&token.secret), token.hash);
        assert_ne!(hash("something else"), token.hash);
    }

    #[test]
    fn the_hash_does_not_contain_the_secret() {
        let token = Token::generate().unwrap();

        assert_eq!(token.hash.len(), 32);
        assert_ne!(token.hash, token.secret.as_bytes());
    }

    #[test]
    fn debugging_a_token_never_prints_it() {
        let token = Token::generate().unwrap();

        let printed = format!("{token:?}");
        assert!(!printed.contains(&token.secret), "{printed}");
        assert!(printed.contains("<secret>"), "{printed}");
    }
}
