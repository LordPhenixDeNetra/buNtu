//! Orchestration du banc : construire, démarrer, charger, mesurer, vérifier.

use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::Value;

use crate::report;

const DEFAULT_RUNS: u32 = 2;
const DEFAULT_QPS: &str = "500,1000,2000";
const DEFAULT_SECONDS: u32 = 5;
const DEFAULT_WARMUP: u32 = 2;
const DEFAULT_TOLERANCE: f64 = 0.10;
const READY_TIMEOUT: Duration = Duration::from_secs(15);
const WARMUP_QPS: u32 = 50;

#[derive(Debug, Clone)]
pub struct Options {
    pub runs: u32,
    pub qps: Vec<u32>,
    pub seconds: u32,
    pub warmup: u32,
    pub tolerance: f64,
    pub allow_variance: bool,
    pub output: Option<PathBuf>,
}

impl Options {
    pub fn parse(arguments: Vec<String>) -> Result<Self, Box<dyn Error>> {
        let mut options = Self {
            runs: DEFAULT_RUNS,
            qps: parse_qps(DEFAULT_QPS)?,
            seconds: DEFAULT_SECONDS,
            warmup: DEFAULT_WARMUP,
            tolerance: DEFAULT_TOLERANCE,
            allow_variance: false,
            output: None,
        };

        let mut index = 0;
        while index < arguments.len() {
            match arguments[index].as_str() {
                "--runs" => options.runs = take_value(&arguments, &mut index)?.parse()?,
                "--qps" => options.qps = parse_qps(&take_value(&arguments, &mut index)?)?,
                "--seconds" => options.seconds = take_value(&arguments, &mut index)?.parse()?,
                "--warmup" => options.warmup = take_value(&arguments, &mut index)?.parse()?,
                "--tolerance" => options.tolerance = take_value(&arguments, &mut index)?.parse()?,
                "--allow-variance" => options.allow_variance = true,
                "--output" => {
                    options.output = Some(PathBuf::from(take_value(&arguments, &mut index)?));
                }
                other => return Err(format!("option inconnue : {other}").into()),
            }
            index += 1;
        }

        if options.runs == 0 || options.seconds == 0 || options.qps.is_empty() {
            return Err("--runs, --seconds et --qps doivent être positifs".into());
        }
        Ok(options)
    }
}

fn take_value(arguments: &[String], index: &mut usize) -> Result<String, Box<dyn Error>> {
    *index += 1;
    arguments
        .get(*index)
        .cloned()
        .ok_or_else(|| "valeur manquante pour une option".into())
}

fn parse_qps(value: &str) -> Result<Vec<u32>, Box<dyn Error>> {
    let mut steps = Vec::new();
    for item in value.split(',') {
        steps.push(item.trim().parse()?);
    }
    Ok(steps)
}

#[derive(Debug, Clone, Serialize)]
pub struct Measurement {
    pub target: String,
    pub qps: u32,
    pub run: u32,
    pub requests_per_sec: f64,
    pub success_rate: f64,
    pub p50_ms: f64,
    pub p90_ms: f64,
    pub p99_ms: f64,
    pub p999_ms: f64,
    pub rss_kib: Option<u64>,
    pub cpu_percent: Option<f64>,
}

struct TargetRequest<'a> {
    name: &'a str,
    url: &'a str,
    method: &'a str,
    with_body: bool,
}

pub fn run(options: Options) -> Result<(), Box<dyn Error>> {
    build()?;
    let target_directory = target_directory()?;
    let oha = tool_version("oha")?;
    let rustc = tool_version("rustc")?;

    let mut measurements = Vec::new();
    let mut cold_starts_ms = Vec::new();
    for run_index in 1..=options.runs {
        println!("== exécution {run_index}/{}", options.runs);
        let (batch, cold_start_ms) = measure_run(&target_directory, &options, run_index)?;
        measurements.extend(batch);
        cold_starts_ms.push(cold_start_ms);
    }

    let gateway_binary_bytes = fs::metadata(binary(&target_directory, "buntu"))
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    let statics = report::StaticMetrics::new(gateway_binary_bytes, cold_starts_ms);
    let report = report::Report::build(&options, measurements, statics, oha, rustc);
    let directory = match &options.output {
        Some(path) => path.clone(),
        None => PathBuf::from("benchmarks/results").join(report::stamp()),
    };
    let written = report::write(&report, &directory)?;
    println!("rapport écrit dans {}", written.display());

    if report.reproducibility.passed || options.allow_variance {
        Ok(())
    } else {
        Err("reproductibilité au-delà de la tolérance : voir la section Reproductibilité du rapport (--allow-variance pour publier malgré la dispersion)".into())
    }
}

fn build() -> Result<(), Box<dyn Error>> {
    println!("== compilation (release)");
    let status = Command::new("cargo")
        .args([
            "build",
            "--release",
            "-p",
            "buntu-mock-provider",
            "-p",
            "buntu-cli",
        ])
        .status()?;
    if !status.success() {
        return Err("cargo build --release a échoué".into());
    }
    Ok(())
}

pub(crate) fn target_directory() -> Result<PathBuf, Box<dyn Error>> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()?;
    if !output.status.success() {
        return Err("cargo metadata a échoué".into());
    }
    let metadata: Value = serde_json::from_slice(&output.stdout)?;
    let directory = metadata["target_directory"]
        .as_str()
        .ok_or("target_directory absent de cargo metadata")?;
    Ok(PathBuf::from(directory))
}

fn tool_version(tool: &str) -> Result<String, Box<dyn Error>> {
    let output = Command::new(tool).arg("--version").output()?;
    if !output.status.success() {
        return Err(format!("{tool} n'est pas disponible (--version a échoué)").into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn binary(target_directory: &Path, name: &str) -> PathBuf {
    target_directory
        .join("release")
        .join(format!("{name}{}", std::env::consts::EXE_SUFFIX))
}

pub(crate) fn free_port() -> Result<u16, Box<dyn Error>> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}

struct Server {
    child: Child,
}

impl Server {
    fn spawn(mut command: Command) -> Result<Self, Box<dyn Error>> {
        command.stdout(Stdio::null()).stderr(Stdio::null());
        Ok(Self {
            child: command.spawn()?,
        })
    }

    fn pid(&self) -> u32 {
        self.child.id()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn wait_tcp(port: u16, timeout: Duration) -> Result<(), Box<dyn Error>> {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(format!("aucun service sur le port {port} après {timeout:?}").into())
}

pub(crate) fn wait_http_ready(
    port: u16,
    path: &str,
    timeout: Duration,
) -> Result<(), Box<dyn Error>> {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port)) {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let request =
                format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
            if stream.write_all(request.as_bytes()).is_ok() {
                let mut response = String::new();
                if stream.read_to_string(&mut response).is_ok()
                    && response.starts_with("HTTP/1.1 200")
                {
                    return Ok(());
                }
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(format!("le service sur le port {port} n'est pas prêt après {timeout:?}").into())
}

/// Une exécution complète : simulateur direct, puis passerelle routée vers ce simulateur.
///
/// Rend les mesures et le démarrage à froid (ms) de la passerelle jusqu'à `/ready`.
fn measure_run(
    target_directory: &Path,
    options: &Options,
    run: u32,
) -> Result<(Vec<Measurement>, f64), Box<dyn Error>> {
    println!("-- cible : simulateur (direct)");
    let simulator_port = free_port()?;
    let mut simulator_command = Command::new(binary(target_directory, "mock-provider"));
    simulator_command.env("MOCK_LISTEN", format!("127.0.0.1:{simulator_port}"));
    let simulator = Server::spawn(simulator_command)?;
    wait_tcp(simulator_port, READY_TIMEOUT)?;

    let direct_url = format!("http://127.0.0.1:{simulator_port}/v1/chat/completions");
    warmup(&direct_url, "POST", true, options)?;
    let mut measurements = Vec::new();
    for &qps in &options.qps {
        measurements.push(measure_once(
            &TargetRequest {
                name: "simulator",
                url: &direct_url,
                method: "POST",
                with_body: true,
            },
            qps,
            run,
            options,
            simulator.pid(),
        )?);
    }

    println!("-- cible : passerelle /v1/chat/completions (routée vers le simulateur)");
    let gateway_port = free_port()?;
    let config = write_bench_config(simulator_port, gateway_port)?;
    let database =
        std::env::temp_dir().join(format!("buntu-bench-{}-{run}.sqlite", std::process::id()));
    let mut gateway_command = Command::new(binary(target_directory, "buntu"));
    gateway_command
        .arg("serve")
        .arg("--listen")
        .arg(format!("127.0.0.1:{gateway_port}"))
        .arg("--config")
        .arg(&config)
        .env("BUNTU_DB_PATH", &database);
    let started = Instant::now();
    let gateway = Server::spawn(gateway_command)?;
    wait_http_ready(gateway_port, "/ready", READY_TIMEOUT)?;
    let cold_start_ms = started.elapsed().as_secs_f64() * 1_000.0;

    let gateway_url = format!("http://127.0.0.1:{gateway_port}/v1/chat/completions");
    warmup(&gateway_url, "POST", true, options)?;
    for &qps in &options.qps {
        measurements.push(measure_once(
            &TargetRequest {
                name: "gateway",
                url: &gateway_url,
                method: "POST",
                with_body: true,
            },
            qps,
            run,
            options,
            gateway.pid(),
        )?);
    }

    drop(gateway);
    drop(simulator);
    let _ = fs::remove_file(&config);
    for suffix in ["", "-wal", "-shm"] {
        let _ = fs::remove_file(format!("{}{suffix}", database.display()));
    }
    Ok((measurements, cold_start_ms))
}

fn write_bench_config(simulator_port: u16, gateway_port: u16) -> Result<PathBuf, Box<dyn Error>> {
    let path = std::env::temp_dir().join(format!(
        "buntu-bench-{}-{}.toml",
        std::process::id(),
        gateway_port
    ));
    let content = format!(
        r#"[server]
listen = "127.0.0.1:{gateway_port}"

[[providers]]
id = "mock"
type = "mock"
base_url = "http://127.0.0.1:{simulator_port}"
keys = [{{ secret = "bench-key", weight = 1 }}]

[[routes]]
alias = "bench"
fallbacks = ["mock/fake-model"]
"#
    );
    fs::write(&path, content)?;
    Ok(path)
}

fn warmup(
    url: &str,
    method: &str,
    with_body: bool,
    options: &Options,
) -> Result<(), Box<dyn Error>> {
    if options.warmup == 0 {
        return Ok(());
    }
    run_oha(url, method, with_body, WARMUP_QPS, options.warmup)?;
    Ok(())
}

fn measure_once(
    target: &TargetRequest<'_>,
    qps: u32,
    run: u32,
    options: &Options,
    pid: u32,
) -> Result<Measurement, Box<dyn Error>> {
    println!("   {} @ {qps} req/s", target.name);
    let cpu_before = cpu_ticks(pid);
    let value = run_oha(
        target.url,
        target.method,
        target.with_body,
        qps,
        options.seconds,
    )?;
    let cpu_after = cpu_ticks(pid);

    let summary = &value["summary"];
    let percentiles = &value["latencyPercentiles"];
    let cpu_percent = match (cpu_before, cpu_after) {
        (Some(before), Some(after)) => Some(
            (after.saturating_sub(before)) as f64 / (100.0 * f64::from(options.seconds)) * 100.0,
        ),
        _ => None,
    };

    Ok(Measurement {
        target: target.name.to_owned(),
        qps,
        run,
        requests_per_sec: summary["requestsPerSec"].as_f64().unwrap_or(0.0),
        success_rate: summary["successRate"].as_f64().unwrap_or(0.0),
        p50_ms: percentiles["p50"].as_f64().unwrap_or(0.0) * 1000.0,
        p90_ms: percentiles["p90"].as_f64().unwrap_or(0.0) * 1000.0,
        p99_ms: percentiles["p99"].as_f64().unwrap_or(0.0) * 1000.0,
        p999_ms: percentiles["p99.9"].as_f64().unwrap_or(0.0) * 1000.0,
        rss_kib: rss_kib(pid),
        cpu_percent,
    })
}

fn run_oha(
    url: &str,
    method: &str,
    with_body: bool,
    qps: u32,
    seconds: u32,
) -> Result<Value, Box<dyn Error>> {
    let mut command = Command::new("oha");
    command.args([
        "-z",
        &format!("{seconds}s"),
        "-q",
        &qps.to_string(),
        "-m",
        method,
        "--no-tui",
        "--output-format",
        "json",
    ]);
    if with_body {
        command.args([
            "-H",
            "content-type: application/json",
            "-d",
            "{\"model\":\"bench\",\"messages\":[{\"role\":\"user\",\"content\":\"bonjour\"}],\"stream\":false}",
        ]);
    }
    command.arg(url);

    let output = command.output()?;
    if !output.status.success() {
        return Err(format!("oha a échoué : {}", String::from_utf8_lossy(&output.stderr)).into());
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

#[cfg(target_os = "linux")]
fn rss_kib(pid: u32) -> Option<u64> {
    let status = fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            return rest.split_whitespace().next()?.parse().ok();
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn rss_kib(_pid: u32) -> Option<u64> {
    None
}

/// Tics CPU consommés par le processus. Linux seulement, USER_HZ supposé à 100.
#[cfg(target_os = "linux")]
fn cpu_ticks(pid: u32) -> Option<u64> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let close = stat.rfind(')')?;
    let fields: Vec<&str> = stat[close + 1..].split_whitespace().collect();
    let utime: u64 = fields.get(11)?.parse().ok()?;
    let stime: u64 = fields.get(12)?.parse().ok()?;
    Some(utime + stime)
}

#[cfg(not(target_os = "linux"))]
fn cpu_ticks(_pid: u32) -> Option<u64> {
    None
}
