-- Couche « base » de la configuration (cahier §7.1, revue 2026-11-05 section 2).
-- Une clé est soit chiffrée (`secret_ciphertext`), soit une référence `env:VAR` (`reference`,
-- chiffré vide) : une référence n'est pas un secret.
ALTER TABLE provider_keys ADD COLUMN reference TEXT;
-- Surcharges de timeouts d'une route, en JSON (`RawTimeouts`).
ALTER TABLE routes ADD COLUMN timeouts TEXT NOT NULL DEFAULT '{}';
