//! Preuves de T1.18 : chemin chaud non bloquant, pertes comptées, écriture par lots.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use buntu_observe::request_log::{LogSink, RequestLog, WriteFuture};
use buntu_store::RequestLogRecord;

fn record(index: usize) -> RequestLogRecord {
    RequestLogRecord {
        id: format!("req-{index}"),
        model: "chat".to_owned(),
        provider: "mock".to_owned(),
        status: 200,
        latency_ms: 1,
        input_tokens: 1,
        output_tokens: 1,
        created_at: 1_700_000_000,
    }
}

/// Base artificiellement lente : 1 s par écriture.
struct SlowSink {
    written: Arc<AtomicUsize>,
}

impl LogSink for SlowSink {
    fn write<'a>(&'a self, records: &'a [RequestLogRecord]) -> WriteFuture<'a> {
        Box::pin(async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            self.written.fetch_add(records.len(), Ordering::Relaxed);
            Ok(())
        })
    }
}

struct RecordingSink {
    batches: Arc<std::sync::Mutex<Vec<usize>>>,
}

impl LogSink for RecordingSink {
    fn write<'a>(&'a self, records: &'a [RequestLogRecord]) -> WriteFuture<'a> {
        Box::pin(async move {
            if let Ok(mut batches) = self.batches.lock() {
                batches.push(records.len());
            }
            Ok(())
        })
    }
}

#[tokio::test]
async fn try_log_never_blocks_even_when_the_database_is_slow() {
    let written = Arc::new(AtomicUsize::new(0));
    let sink = Arc::new(SlowSink {
        written: Arc::clone(&written),
    });
    let (log, task) = RequestLog::start_with_sink(sink);
    let started = std::time::Instant::now();
    let mut accepted = 0;
    for index in 0..20_000 {
        if log.try_log(record(index)) {
            accepted += 1;
        }
    }
    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_millis(100),
        "20 000 try_log ont pris {elapsed:?} (surcoût > 5 µs en moyenne)"
    );
    assert!(accepted > 0);
    assert!(log.dropped() > 0, "les pertes doivent être comptées");
    // La base simulée est lente : on n'attend pas le drainage complet.
    task.abort();
}

#[tokio::test]
async fn entries_are_written_in_bounded_batches() {
    let batches = Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = Arc::new(RecordingSink {
        batches: Arc::clone(&batches),
    });
    let (log, task) = RequestLog::start_with_sink(sink);
    for index in 0..1_000 {
        assert!(log.try_log(record(index)));
    }
    drop(log);
    if let Err(error) = task.await {
        panic!("la tâche d'écriture a échoué : {error}");
    }
    let batches = batches.lock().map(|batches| batches.clone());
    let batches = match batches {
        Ok(batches) => batches,
        Err(_) => panic!("verrou de test empoisonné"),
    };
    let total: usize = batches.iter().sum();
    assert_eq!(total, 1_000);
    assert!(
        batches.iter().all(|size| *size <= 500),
        "lots trop grands : {batches:?}"
    );
}
