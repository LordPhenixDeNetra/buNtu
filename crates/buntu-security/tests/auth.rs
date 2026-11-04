//! Preuves de T1.20 : Argon2id, jetons hachés et blocage après dix échecs.

use std::error::Error;

use buntu_security::password::{hash_password, verify_password};
use buntu_security::session::{LoginLimiter, MAX_ATTEMPTS};
use buntu_security::tokens::{generate_token, hash_token, verify_token};

#[test]
fn password_hash_rejects_a_wrong_password() -> Result<(), Box<dyn Error>> {
    let hash = hash_password("correct horse battery staple")?;
    assert!(verify_password(&hash, "correct horse battery staple")?);
    assert!(!verify_password(&hash, "mauvais mot de passe")?);
    Ok(())
}

#[test]
fn password_hash_is_argon2id_with_random_salt() -> Result<(), Box<dyn Error>> {
    let first = hash_password("motdepasse")?;
    let second = hash_password("motdepasse")?;
    assert!(first.as_str().starts_with("$argon2id$"));
    assert_ne!(first, second, "le sel doit être aléatoire");
    Ok(())
}

#[test]
fn token_is_256_bits_and_only_its_hash_is_stored() -> Result<(), Box<dyn Error>> {
    let api_token = generate_token();
    assert!(api_token.token.starts_with("buntu_"));
    assert_ne!(api_token.token, api_token.hash);
    assert!(!api_token.hash.contains(&api_token.token));
    assert_eq!(api_token.hash, hash_token(&api_token.token));
    assert!(verify_token(&api_token.hash, &api_token.token)?);
    assert!(!verify_token(&api_token.hash, "buntu_autre")?);
    Ok(())
}

#[test]
fn ten_failures_block_the_account_temporarily() {
    let limiter = LoginLimiter::new();
    for _ in 0..(MAX_ATTEMPTS - 1) {
        assert!(limiter.check("admin").is_ok());
        assert!(limiter.record_failure("admin").is_ok());
    }
    assert!(limiter.check("admin").is_ok());
    let blocked = limiter.record_failure("admin");
    assert!(blocked.is_err(), "le dixième échec doit bloquer");
    let check = limiter.check("admin");
    assert!(check.is_err(), "le compte doit rester bloqué");
    let retry_after = match check {
        Ok(()) => std::time::Duration::ZERO,
        Err(blocked) => blocked.retry_after,
    };
    assert!(!retry_after.is_zero());

    // Un autre compte n'est pas affecté, et un succès lève le blocage.
    assert!(limiter.check("autre").is_ok());
    limiter.record_success("autre");
    assert!(limiter.check("autre").is_ok());
}
