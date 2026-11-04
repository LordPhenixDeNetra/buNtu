//! Journal des requêtes asynchrone (T1.18).
//!
//! Le chemin chaud ne fait qu'un `try_send` : il ne bloque **jamais**, même si la base est
//! lente. La tâche d'écriture regroupe les lignes par lots (500 lignes ou 1 s) et les pertes
//! sont comptées (`buntu_log_queue_dropped_total`).

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use buntu_store::{RequestLogRecord, Store};
use tokio::sync::mpsc;

/// Nombre maximal de lignes en attente dans le canal.
pub const DEFAULT_CAPACITY: usize = 10_000;
/// Taille d'un lot d'écriture.
pub const BATCH_SIZE: usize = 500;
/// Période de vidage d'un lot partiel.
pub const FLUSH_INTERVAL: Duration = Duration::from_secs(1);

/// Futur d'écriture d'un lot, sans dépendance à `async-trait`.
pub type WriteFuture<'a> = Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;

/// Destination des lots du journal.
pub trait LogSink: Send + Sync + 'static {
    fn write<'a>(&'a self, records: &'a [RequestLogRecord]) -> WriteFuture<'a>;
}

/// Écrit dans SQLite, ligne par ligne (pool borné : jamais de connexion par requête HTTP).
pub struct StoreSink {
    store: Store,
}

impl StoreSink {
    pub fn new(store: Store) -> Self {
        Self { store }
    }
}

impl LogSink for StoreSink {
    fn write<'a>(&'a self, records: &'a [RequestLogRecord]) -> WriteFuture<'a> {
        Box::pin(async move {
            for record in records {
                self.store
                    .insert_request_log(record)
                    .await
                    .map_err(|error| error.to_string())?;
            }
            Ok(())
        })
    }
}

/// Journal asynchrone : clone bon marché, `try_log` non bloquant.
#[derive(Clone)]
pub struct RequestLog {
    sender: mpsc::Sender<RequestLogRecord>,
    dropped: Arc<AtomicU64>,
}

impl RequestLog {
    /// Démarre la tâche d'écriture ; le `JoinHandle` est à conserver par l'appelant.
    pub fn start(store: Store) -> (Self, tokio::task::JoinHandle<()>) {
        Self::start_with_sink(Arc::new(StoreSink::new(store)))
    }

    pub fn start_with_sink(sink: Arc<dyn LogSink>) -> (Self, tokio::task::JoinHandle<()>) {
        let (sender, receiver) = mpsc::channel(DEFAULT_CAPACITY);
        let dropped = Arc::new(AtomicU64::new(0));
        let task = tokio::spawn(writer_loop(receiver, sink));
        (Self { sender, dropped }, task)
    }

    /// Enregistre une ligne sans jamais attendre ; rend `false` si la ligne a été perdue.
    pub fn try_log(&self, record: RequestLogRecord) -> bool {
        match self.sender.try_send(record) {
            Ok(()) => true,
            Err(mpsc::error::TrySendError::Full(_)) => {
                self.dropped.fetch_add(1, Ordering::Relaxed);
                metrics::counter!("buntu_log_queue_dropped_total").increment(1);
                false
            }
            Err(mpsc::error::TrySendError::Closed(_)) => false,
        }
    }

    /// Lignes perdues depuis le démarrage.
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// Attend que la tâche d'écriture ait vidé le canal (arrêt propre).
    pub async fn shutdown(self, task: tokio::task::JoinHandle<()>) {
        drop(self.sender);
        let _ = task.await;
    }
}

async fn writer_loop(mut receiver: mpsc::Receiver<RequestLogRecord>, sink: Arc<dyn LogSink>) {
    let mut batch: Vec<RequestLogRecord> = Vec::with_capacity(BATCH_SIZE);
    let mut ticker = tokio::time::interval(FLUSH_INTERVAL);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            maybe = receiver.recv() => {
                match maybe {
                    Some(record) => {
                        batch.push(record);
                        if batch.len() >= BATCH_SIZE {
                            flush(&sink, &mut batch).await;
                        }
                    }
                    None => {
                        flush(&sink, &mut batch).await;
                        return;
                    }
                }
            }
            _ = ticker.tick() => {
                if !batch.is_empty() {
                    flush(&sink, &mut batch).await;
                }
            }
        }
    }
}

async fn flush(sink: &Arc<dyn LogSink>, batch: &mut Vec<RequestLogRecord>) {
    if batch.is_empty() {
        return;
    }
    let records = std::mem::take(batch);
    if let Err(error) = sink.write(&records).await {
        // Une base lente ou en erreur ne doit jamais remonter dans le chemin chaud.
        metrics::counter!("buntu_log_write_errors_total").increment(1);
        let _ = error;
    }
}
