//! Rapport JSON + Markdown du banc de benchmark.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::bench::{Measurement, Options};

#[derive(Debug, Serialize)]
pub struct Report {
    pub generated_at: String,
    pub generated_at_unix: u64,
    pub host: Host,
    pub toolchain: Toolchain,
    pub protocol: Protocol,
    pub measurements: Vec<Measurement>,
    pub reproducibility: Reproducibility,
}

#[derive(Debug, Serialize)]
pub struct Host {
    pub os: String,
    pub cores: usize,
    pub oha: String,
}

#[derive(Debug, Serialize)]
pub struct Toolchain {
    pub rustc: String,
}

#[derive(Debug, Serialize)]
pub struct Protocol {
    pub warmup_seconds: u32,
    pub duration_seconds: u32,
    pub runs: u32,
    pub qps_steps: Vec<u32>,
    pub tolerance: f64,
}

#[derive(Debug, Serialize)]
pub struct Reproducibility {
    pub tolerance: f64,
    pub passed: bool,
    pub checks: Vec<Check>,
}

#[derive(Debug, Serialize)]
pub struct Check {
    pub target: String,
    pub qps: u32,
    pub p99_ms: Vec<f64>,
    pub deviation: f64,
    pub passed: bool,
}

impl Report {
    pub fn build(
        options: &Options,
        measurements: Vec<Measurement>,
        oha: String,
        rustc: String,
    ) -> Self {
        let (generated_at, generated_at_unix) = timestamp();
        let reproducibility = Reproducibility::check(options, &measurements);
        Self {
            generated_at,
            generated_at_unix,
            host: Host {
                os: format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
                cores: std::thread::available_parallelism()
                    .map(|count| count.get())
                    .unwrap_or(1),
                oha,
            },
            toolchain: Toolchain { rustc },
            protocol: Protocol {
                warmup_seconds: options.warmup,
                duration_seconds: options.seconds,
                runs: options.runs,
                qps_steps: options.qps.clone(),
                tolerance: options.tolerance,
            },
            measurements,
            reproducibility,
        }
    }
}

impl Reproducibility {
    fn check(options: &Options, measurements: &[Measurement]) -> Self {
        let mut checks = Vec::new();
        for target in ["simulator", "gateway"] {
            for &qps in &options.qps {
                let values: Vec<f64> = measurements
                    .iter()
                    .filter(|measure| measure.target == target && measure.qps == qps)
                    .map(|measure| measure.p99_ms)
                    .collect();
                if values.len() < 2 {
                    continue;
                }
                let mut min = f64::INFINITY;
                let mut max = f64::NEG_INFINITY;
                for value in &values {
                    min = min.min(*value);
                    max = max.max(*value);
                }
                let deviation = if min > 0.0 { (max - min) / min } else { 0.0 };
                checks.push(Check {
                    target: target.to_owned(),
                    qps,
                    p99_ms: values,
                    deviation,
                    passed: deviation <= options.tolerance,
                });
            }
        }
        let passed = checks.iter().all(|check| check.passed);
        Self {
            tolerance: options.tolerance,
            passed,
            checks,
        }
    }
}

pub fn write(report: &Report, directory: &Path) -> Result<PathBuf, Box<dyn Error>> {
    fs::create_dir_all(directory)?;
    fs::write(
        directory.join("results.json"),
        serde_json::to_string_pretty(report)?,
    )?;
    let path = directory.join("report.md");
    fs::write(&path, markdown(report))?;
    Ok(path)
}

fn markdown(report: &Report) -> String {
    let mut text = String::new();
    text.push_str("# Rapport de benchmark\n\n");
    text.push_str(&format!(
        "- Généré : `{}` (epoch {})\n",
        report.generated_at, report.generated_at_unix
    ));
    text.push_str(&format!(
        "- Machine : {} — {} cœurs logiques\n",
        report.host.os, report.host.cores
    ));
    text.push_str(&format!(
        "- Outils : {} ; {}\n",
        report.host.oha, report.toolchain.rustc
    ));
    text.push_str(&format!(
        "- Protocole : boucle ouverte (`oha -q`), chauffe {} s, paliers {} s, {} exécution(s), QPS {:?}\n",
        report.protocol.warmup_seconds,
        report.protocol.duration_seconds,
        report.protocol.runs,
        report.protocol.qps_steps
    ));
    text.push_str(
        "\n> Mesure locale : générateur et serveur sur la même machine. Ces chiffres sont un\n> plancher, pas la mesure publiée du cahier §11.2 (machines séparées).\n\n",
    );

    text.push_str("## Mesures\n\n");
    text.push_str(
        "| Cible | QPS | Exécution | req/s | Succès | P50 | P90 | P99 | P99,9 | RSS | CPU |\n",
    );
    text.push_str("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |\n");
    for measure in &report.measurements {
        let rss = measure
            .rss_kib
            .map_or_else(|| "n/a".to_owned(), |value| format!("{value} Kio"));
        let cpu = measure
            .cpu_percent
            .map_or_else(|| "n/a".to_owned(), |value| format!("{value:.1} %"));
        text.push_str(&format!(
            "| {} | {} | {} | {:.0} | {:.2} % | {:.2} ms | {:.2} ms | {:.2} ms | {:.2} ms | {} | {} |\n",
            measure.target,
            measure.qps,
            measure.run,
            measure.requests_per_sec,
            measure.success_rate * 100.0,
            measure.p50_ms,
            measure.p90_ms,
            measure.p99_ms,
            measure.p999_ms,
            rss,
            cpu,
        ));
    }

    text.push_str("\n## Reproductibilité des P99\n\n");
    text.push_str(&format!(
        "Tolérance : {:.0} % d'écart relatif entre la plus petite et la plus grande valeur.\n\n",
        report.reproducibility.tolerance * 100.0
    ));
    text.push_str("| Cible | QPS | P99 par exécution (ms) | Écart | Verdict |\n");
    text.push_str("| --- | --- | --- | --- | --- |\n");
    for check in &report.reproducibility.checks {
        let values: Vec<String> = check
            .p99_ms
            .iter()
            .map(|value| format!("{value:.2}"))
            .collect();
        let verdict = if check.passed { "ok" } else { "échec" };
        text.push_str(&format!(
            "| {} | {} | {} | {:.1} % | {} |\n",
            check.target,
            check.qps,
            values.join(" / "),
            check.deviation * 100.0,
            verdict
        ));
    }
    if report.reproducibility.checks.is_empty() {
        text.push_str("\n**Vérification non applicable : moins de deux exécutions.**\n");
    } else if report.reproducibility.passed {
        text.push_str("\n**Verdict : reproductible.**\n");
    } else {
        text.push_str("\n**Verdict : non reproductible — voir les lignes en échec.**\n");
    }

    text.push_str(
        "\n## Limites\n\n- RSS et CPU : collectés via `/proc` sous Linux ; `n/a` sur les autres systèmes.\n- Cible `gateway` : `/ready` pour l'instant ; le coût du routage `/v1/chat/completions` sera mesuré après T1.13.\n",
    );
    text
}

fn timestamp() -> (String, u64) {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let days = (seconds / 86_400) as i64;
    let remainder = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hours = remainder / 3_600;
    let minutes = (remainder % 3_600) / 60;
    (
        format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}Z"),
        seconds,
    )
}

/// Horodatage compact utilisé comme nom de répertoire : `AAAA-MM-JJ-HHMM`.
pub fn stamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let days = (seconds / 86_400) as i64;
    let remainder = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hours = remainder / 3_600;
    let minutes = (remainder % 3_600) / 60;
    format!("{year:04}-{month:02}-{day:02}-{hours:02}{minutes:02}")
}

/// Conversion jours depuis l'époque vers date civile (algorithme de H. Hinnant).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };
    (year, month as u32, day as u32)
}
