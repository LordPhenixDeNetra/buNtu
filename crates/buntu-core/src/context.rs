//! Contexte d'une requête : identifiant, instant de début, clé virtuelle, extensions typées.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;
use std::time::Instant;

/// Identifiant de requête, fourni par la passerelle.
pub type RequestId = String;

/// Contexte transmis à la chaîne de hooks pendant toute la durée d'une requête.
#[derive(Debug)]
pub struct RequestContext {
    pub id: RequestId,
    pub started_at: Instant,
    /// Identifiant de la clé virtuelle, une fois la gouvernance branchée (T2.12).
    pub virtual_key: Option<String>,
    pub metadata: HashMap<String, String>,
    pub extensions: Extensions,
}

impl RequestContext {
    pub fn new(id: impl Into<RequestId>) -> Self {
        Self {
            id: id.into(),
            started_at: Instant::now(),
            virtual_key: None,
            metadata: HashMap::new(),
            extensions: Extensions::default(),
        }
    }

    pub fn with_virtual_key(mut self, key: impl Into<String>) -> Self {
        self.virtual_key = Some(key.into());
        self
    }

    pub fn elapsed(&self) -> std::time::Duration {
        self.started_at.elapsed()
    }
}

/// Sac d'extensions typées, sans dépendance supplémentaire.
#[derive(Default)]
pub struct Extensions {
    values: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Extensions {
    pub fn insert<T: Any + Send + Sync>(&mut self, value: T) {
        self.values.insert(TypeId::of::<T>(), Box::new(value));
    }

    pub fn get<T: Any + Send + Sync>(&self) -> Option<&T> {
        self.values.get(&TypeId::of::<T>())?.downcast_ref::<T>()
    }

    pub fn get_mut<T: Any + Send + Sync>(&mut self) -> Option<&mut T> {
        self.values.get_mut(&TypeId::of::<T>())?.downcast_mut::<T>()
    }

    pub fn remove<T: Any + Send + Sync>(&mut self) -> Option<T> {
        let boxed = self.values.remove(&TypeId::of::<T>())?;
        boxed.downcast::<T>().ok().map(|value| *value)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

impl fmt::Debug for Extensions {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Extensions")
            .field("len", &self.values.len())
            .finish()
    }
}
