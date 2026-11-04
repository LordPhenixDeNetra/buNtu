//! Type `Secret<T>` : valeur sensible jamais lisible par `Debug`/`Display` (T1.21).
//!
//! S'appuie sur `secrecy` : la valeur est zéroïsée à la destruction et ne peut être lue que par
//! `expose()`.

use std::fmt;

/// Valeur sensible (clé API, jeton, mot de passe) protégée contre l'affichage accidentel.
pub struct Secret<T: zeroize::Zeroize + Clone>(secrecy::SecretBox<T>);

impl<T: zeroize::Zeroize + Clone> Secret<T> {
    pub fn new(value: T) -> Self {
        Self(secrecy::SecretBox::new(Box::new(value)))
    }

    /// Seul accès à la valeur ; à n'utiliser qu'au point d'usage (en-tête, déchiffrement).
    pub fn expose(&self) -> &T {
        secrecy::ExposeSecret::expose_secret(&self.0)
    }
}

impl<T: zeroize::Zeroize + Clone> Clone for Secret<T> {
    fn clone(&self) -> Self {
        Self::new(self.expose().clone())
    }
}

impl<T: zeroize::Zeroize + Clone> fmt::Debug for Secret<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Secret([masqué])")
    }
}

impl<T: zeroize::Zeroize + Clone> From<T> for Secret<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}
