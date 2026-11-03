//! Sélection pondérée des clés : table d'alias de Walker et pauses 429.
//!
//! La table est construite une fois à la compilation de la configuration. Le tirage est à
//! point fixe (`u32`) et la lecture des pauses est un simple compteur atomique : le chemin
//! sans pause ne lit jamais l'horloge.

use std::fmt;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Erreur de construction ou de sélection d'une clé.
#[derive(Debug, thiserror::Error)]
pub enum KeySelectionError {
    #[error("aucune clé à sélectionner")]
    Empty,
    #[error("le poids d'une clé doit être supérieur à zéro")]
    ZeroWeight,
    #[error("index de clé hors limites : {index} (taille {len})")]
    IndexOutOfRange { index: usize, len: usize },
    #[error("toutes les clés sont en pause")]
    AllPaused { retry_after: Duration },
}

struct AliasEntry {
    /// Probabilité d'être tirée directement, en fraction de `u32::MAX`.
    probability: u32,
    alias: usize,
    weight: u32,
    /// Instant de fin de pause, en millisecondes depuis `base` ; 0 = pas de pause.
    pause_until_ms: AtomicU64,
}

/// Table de sélection O(1), avec une pause indépendante par clé.
pub struct KeySelector {
    base: Instant,
    entries: Vec<AliasEntry>,
    /// Nombre de clés en pause (approximatif après expiration : il sert d'aiguillage rapide).
    paused_count: AtomicUsize,
}

impl Clone for KeySelector {
    fn clone(&self) -> Self {
        let entries = self
            .entries
            .iter()
            .map(|entry| AliasEntry {
                probability: entry.probability,
                alias: entry.alias,
                weight: entry.weight,
                pause_until_ms: AtomicU64::new(0),
            })
            .collect();
        Self {
            base: self.base,
            entries,
            paused_count: AtomicUsize::new(0),
        }
    }
}

impl fmt::Debug for KeySelector {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("KeySelector")
            .field("len", &self.entries.len())
            .finish()
    }
}

impl KeySelector {
    /// Construit la table d'alias à partir des poids, tous strictement positifs.
    pub fn new(weights: &[u32]) -> Result<Self, KeySelectionError> {
        if weights.is_empty() {
            return Err(KeySelectionError::Empty);
        }
        if weights.contains(&0) {
            return Err(KeySelectionError::ZeroWeight);
        }

        let count = weights.len();
        let total: u64 = weights.iter().map(|weight| u64::from(*weight)).sum();
        let mut probabilities: Vec<f64> = weights
            .iter()
            .map(|weight| f64::from(*weight) * count as f64 / total as f64)
            .collect();
        let mut aliases: Vec<usize> = (0..count).collect();
        let mut small: Vec<usize> = Vec::new();
        let mut large: Vec<usize> = Vec::new();

        for (index, probability) in probabilities.iter().enumerate() {
            if *probability < 1.0 {
                small.push(index);
            } else {
                large.push(index);
            }
        }

        while !small.is_empty() {
            let Some(small_index) = small.pop() else {
                break;
            };
            let Some(large_index) = large.pop() else {
                small.push(small_index);
                break;
            };
            aliases[small_index] = large_index;
            let surplus = probabilities[large_index] + probabilities[small_index] - 1.0;
            probabilities[large_index] = surplus;
            if surplus < 1.0 {
                small.push(large_index);
            } else {
                large.push(large_index);
            }
        }
        while let Some(index) = large.pop() {
            probabilities[index] = 1.0;
        }
        while let Some(index) = small.pop() {
            probabilities[index] = 1.0;
        }

        let entries = weights
            .iter()
            .enumerate()
            .map(|(index, weight)| AliasEntry {
                probability: fixed_point(probabilities[index]),
                alias: aliases[index],
                weight: *weight,
                pause_until_ms: AtomicU64::new(0),
            })
            .collect();

        Ok(Self {
            base: Instant::now(),
            entries,
            paused_count: AtomicUsize::new(0),
        })
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn weight(&self, index: usize) -> Option<u32> {
        self.entries.get(index).map(|entry| entry.weight)
    }

    /// Sélectionne une clé non en pause.
    ///
    /// Chemin rapide : si aucune clé n'est en pause (compteur à zéro), un seul tirage suffit et
    /// aucune horloge n'est lue. Sinon, huit tirages filtrés puis un repli déterministe, et une
    /// erreur explicite si toutes les clés sont en pause.
    pub fn select(&self) -> Result<usize, KeySelectionError> {
        if self.paused_count.load(Ordering::Relaxed) == 0 {
            return Ok(self.draw());
        }

        const ATTEMPTS: usize = 8;
        for _ in 0..ATTEMPTS {
            let index = self.draw();
            if !self.is_paused(index) {
                return Ok(index);
            }
        }

        let count = self.entries.len();
        if let Some(index) = (0..count).find(|index| !self.is_paused(*index)) {
            // Le compteur peut avoir dérivé après des expirations : on le recalcule ici, sur le
            // chemin rare où huit tirages ont tous touché des clés en pause.
            let paused = (0..count).filter(|index| self.is_paused(*index)).count();
            self.paused_count.store(paused, Ordering::Relaxed);
            return Ok(index);
        }

        Err(KeySelectionError::AllPaused {
            retry_after: self.minimum_pause().unwrap_or_default(),
        })
    }

    /// Met une clé en pause pour la durée donnée (typiquement `retry-after` d'un 429).
    pub fn pause(&self, index: usize, duration: Duration) -> Result<(), KeySelectionError> {
        let entry = self
            .entries
            .get(index)
            .ok_or(KeySelectionError::IndexOutOfRange {
                index,
                len: self.entries.len(),
            })?;
        let until = self.now_ms() + duration.as_millis() as u64;
        let previous = entry.pause_until_ms.swap(until, Ordering::AcqRel);
        if previous == 0 {
            self.paused_count.fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }

    /// Lève la pause d'une clé.
    pub fn unpause(&self, index: usize) -> Result<(), KeySelectionError> {
        let entry = self
            .entries
            .get(index)
            .ok_or(KeySelectionError::IndexOutOfRange {
                index,
                len: self.entries.len(),
            })?;
        let previous = entry.pause_until_ms.swap(0, Ordering::AcqRel);
        if previous != 0 {
            // Décrément saturant sans `fetch_update`/`try_update` (déprécié selon les versions) :
            // boucle de comparaison-échange, sans risque de passage sous zéro.
            let mut current = self.paused_count.load(Ordering::Relaxed);
            loop {
                let next = current.saturating_sub(1);
                if next == current {
                    break;
                }
                match self.paused_count.compare_exchange_weak(
                    current,
                    next,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(observed) => current = observed,
                }
            }
        }
        Ok(())
    }

    /// Pause restante d'une clé, `None` si elle est disponible.
    pub fn paused_for(&self, index: usize) -> Option<Duration> {
        let entry = self.entries.get(index)?;
        self.remaining(entry)
    }

    fn draw(&self) -> usize {
        let random = fastrand::u64(..);
        let scaled = u128::from((random >> 32) as u32) * self.entries.len() as u128;
        let index = (scaled >> 32) as usize;
        let fraction = (scaled & 0xFFFF_FFFF) as u32;
        let Some(entry) = self.entries.get(index) else {
            return 0;
        };
        if fraction < entry.probability {
            index
        } else {
            entry.alias
        }
    }

    fn is_paused(&self, index: usize) -> bool {
        self.entries
            .get(index)
            .is_some_and(|entry| self.remaining(entry).is_some())
    }

    fn remaining(&self, entry: &AliasEntry) -> Option<Duration> {
        let until = entry.pause_until_ms.load(Ordering::Acquire);
        let now = self.now_ms();
        if until > now {
            Some(Duration::from_millis(until - now))
        } else {
            None
        }
    }

    fn minimum_pause(&self) -> Option<Duration> {
        self.entries
            .iter()
            .filter_map(|entry| self.remaining(entry))
            .min()
    }

    fn now_ms(&self) -> u64 {
        self.base.elapsed().as_millis() as u64
    }
}

fn fixed_point(probability: f64) -> u32 {
    (probability.clamp(0.0, 1.0) * f64::from(u32::MAX)).round() as u32
}
