//! Outillage du projet : `cargo xtask bench` et `cargo xtask reload-soak`.

mod bench;
mod reload;
mod report;

use std::error::Error;

const USAGE: &str = "\
usage: cargo xtask <commande>

commandes:
  bench [options]        banc de benchmark (protocole : benchmarks/README.md)
  reload-soak [options]  preuve de rechargement à chaud sous charge (T1.2)

options de bench:
  --runs N        exécutions complètes (défaut : 2 ; la reproductibilité est vérifiée à 2)
  --qps A,B,C     paliers de débit en requêtes/s (défaut : 500,1000,2000)
  --seconds N     durée de chaque palier, en secondes (défaut : 5)
  --warmup N      durée de la chauffe, en secondes (défaut : 2)
  --tolerance F   écart relatif maximal des P99 entre exécutions (défaut : 0.10)
  --allow-variance  publie la campagne même si les P99 dépassent la tolérance (dispersion reportée)
  --output DIR    répertoire de sortie (défaut : benchmarks/results/<date>)

options de reload-soak:
  --qps N             débit maintenu pendant l'essai (défaut : 5000)
  --seconds N         durée de l'essai (défaut : 60)
  --reload-every N    intervalle entre deux réécritures de configuration (défaut : 2)
";

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = std::env::args().skip(1);
    match arguments.next().as_deref() {
        Some("bench") => {
            let options = bench::Options::parse(arguments.collect())?;
            bench::run(options)
        }
        Some("reload-soak") => {
            let options = reload::Options::parse(arguments.collect())?;
            reload::run(options)
        }
        Some("help") | Some("--help") | Some("-h") | None => {
            println!("{USAGE}");
            Ok(())
        }
        Some(other) => Err(format!("commande xtask inconnue : {other}\n\n{USAGE}").into()),
    }
}
