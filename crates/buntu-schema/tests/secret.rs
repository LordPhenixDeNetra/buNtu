//! Preuves du type `Secret<T>` (T1.21).

use buntu_schema::Secret;

#[test]
fn debug_never_reveals_the_value() {
    let secret = Secret::new("sk-tres-secret".to_owned());
    let rendered = format!("{secret:?}");
    assert_eq!(rendered, "Secret([masqué])");
    assert!(!rendered.contains("sk-tres-secret"));
}

#[test]
fn expose_returns_the_value() {
    let secret = Secret::new("valeur".to_owned());
    assert_eq!(secret.expose(), "valeur");
    let copy = secret.clone();
    assert_eq!(copy.expose(), "valeur");
}
