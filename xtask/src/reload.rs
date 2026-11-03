//! Preuve de rechargement à chaud : charge soutenue + réécritures de configuration.

use std::error::Error;
use std::fs;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use serde_json::Value;

use crate::bench;

const DEFAULT_QPS: u32 = 5000;
const DEFAULT_SECONDS: u32 = 60;
const DEFAULT_RELOAD_EVERY: u32 = 2;
const READY_TIMEOUT: Duration = Duration::from_secs(15);

pub struct Options {
    pub qps: u32,
    pub seconds: u32,
    pub reload_every: u32,
}

impl Options {
    pub fn parse(arguments: Vec<String>) -> Result<Self, Box<dyn Error>> {
        let mut options = Self {
            qps: DEFAULT_QPS,
            seconds: DEFAULT_SECONDS,
            reload_every: DEFAULT_RELOAD_EVERY,
        };
        let mut index = 0;
        while index < arguments.len() {
            match arguments[index].as_str() {
                "--qps" => {
                    index += 1;
                    options.qps = arguments
                        .get(index)
                        .ok_or("--qps requiert une valeur")?
                        .parse()?;
                }
                "--seconds" => {
                    index += 1;
                    options.seconds = arguments
                        .get(index)
                        .ok_or("--seconds requiert une valeur")?
                        .parse()?;
                }
                "--reload-every" => {
                    index += 1;
                    options.reload_every = arguments
                        .get(index)
                        .ok_or("--reload-every requiert une valeur")?
                        .parse()?;
                }
                other => return Err(format!("option inconnue : {other}").into()),
            }
            index += 1;
        }
        if options.qps == 0 || options.seconds == 0 || options.reload_every == 0 {
            return Err("--qps, --seconds et --reload-every doivent être positifs".into());
        }
        Ok(options)
    }
}

struct Server {
    child: Child,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn run(options: Options) -> Result<(), Box<dyn Error>> {
    let target_directory = bench::target_directory()?;
    build()?;

    let path = std::env::temp_dir().join(format!("buntu-reload-soak-{}.toml", std::process::id()));
    write_config(&path, 1)?;

    let port = bench::free_port()?;
    let binary = target_directory
        .join("release")
        .join(format!("buntu{}", std::env::consts::EXE_SUFFIX));
    let child = Command::new(binary)
        .arg("serve")
        .arg("--config")
        .arg(&path)
        .arg("--listen")
        .arg(format!("127.0.0.1:{port}"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let server = Server { child };
    bench::wait_http_ready(port, "/ready", READY_TIMEOUT)?;

    println!(
        "== charge {} req/s pendant {} s, rechargement toutes les {} s",
        options.qps, options.seconds, options.reload_every
    );

    let writer = {
        let path = path.clone();
        let reload_every = Duration::from_secs(u64::from(options.reload_every));
        let seconds = options.seconds;
        std::thread::spawn(move || -> Result<u32, std::io::Error> {
            let mut rewrites = 0;
            let mut elapsed = 0;
            while elapsed < seconds {
                std::thread::sleep(reload_every);
                elapsed += reload_every.as_secs() as u32;
                rewrites += 1;
                write_config(&path, 1 + rewrites % 2)?;
            }
            Ok(rewrites)
        })
    };

    let url = format!("http://127.0.0.1:{port}/health");
    let value = run_oha(&url, options.qps, options.seconds)?;

    let rewrites = writer
        .join()
        .map_err(|_| "le fil de rechargement a paniqué")??;
    drop(server);
    let _ = fs::remove_file(&path);

    let summary = &value["summary"];
    let success_rate = summary["successRate"].as_f64().unwrap_or(0.0);
    let requests_per_sec = summary["requestsPerSec"].as_f64().unwrap_or(0.0);
    let elapsed = summary["total"].as_f64().unwrap_or(0.0);
    let requests = requests_per_sec * elapsed;

    println!(
        "== résultat : {:.0} req/s, succès {:.2} %, ~{:.0} requêtes, {} rechargements",
        requests_per_sec,
        success_rate * 100.0,
        requests,
        rewrites
    );

    if (success_rate - 1.0).abs() > f64::EPSILON {
        return Err(format!(
            "des erreurs ont été observées pendant les rechargements (succès {:.4})",
            success_rate
        )
        .into());
    }
    Ok(())
}

fn build() -> Result<(), Box<dyn Error>> {
    let status = Command::new("cargo")
        .args(["build", "--release", "-p", "buntu-cli"])
        .status()?;
    if !status.success() {
        return Err("cargo build --release a échoué".into());
    }
    Ok(())
}

fn write_config(path: &std::path::Path, weight: u32) -> Result<(), std::io::Error> {
    let content = format!(
        r#"[server]
listen = "127.0.0.1:8080"

[[providers]]
id = "mock"
type = "mock"
keys = [{{ secret = "sk-test", weight = {weight} }}]

[[routes]]
alias = "chat"
fallbacks = ["mock/modele"]
"#
    );
    fs::write(path, content)
}

fn run_oha(url: &str, qps: u32, seconds: u32) -> Result<Value, Box<dyn Error>> {
    let output = Command::new("oha")
        .args([
            "-z",
            &format!("{seconds}s"),
            "-q",
            &qps.to_string(),
            "--no-tui",
            "--output-format",
            "json",
            url,
        ])
        .output()?;
    if !output.status.success() {
        return Err(format!("oha a échoué : {}", String::from_utf8_lossy(&output.stderr)).into());
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}
