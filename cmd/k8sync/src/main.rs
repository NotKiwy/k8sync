use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::process::Command;

use k8sync::diff;

#[derive(Parser)]
#[command(name = "k8sync")]
#[command(version = "0.1.0")]
#[command(about = "Kubernetes multi-cluster drift detector", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Compare {
        #[arg(short, long)]
        contexts: Option<String>,

        #[arg(short, long)]
        namespace: Option<String>,

        #[arg(short, long, default_value = "text", value_parser = ["text", "json"])]
        output: String,
    },
    List,
    Version,
}

#[derive(Deserialize, Default)]
struct Config {
    contexts: Option<Vec<String>>,
    namespace: Option<String>,
}

fn load_config() -> Config {
    let path = std::path::Path::new(".k8sync.yaml");
    if !path.exists() {
        return Config::default();
    }
    let content = std::fs::read_to_string(path).unwrap_or_default();
    serde_yaml::from_str(&content).unwrap_or_default()
}

fn main() {
    let _cli = Cli::parse();
    let cfg = load_config();

    match &_cli.command {
        Commands::Compare {
            contexts,
            namespace,
            output,
        } => {
            let ctxstr = contexts
                .clone()
                .or_else(|| cfg.contexts.map(|v| v.join(",")))
                .unwrap_or_else(|| {
                    eprintln!("[!] No contexts specified. Use --contexts or set in .k8sync.yaml");
                    std::process::exit(1);
                });

            let ns = namespace
                .clone()
                .or(cfg.namespace)
                .unwrap_or_else(|| "default".to_string());

            let parts: Vec<&str> = ctxstr.splitn(2, ',').map(str::trim).collect();
            if parts.len() != 2 {
                eprintln!("[!] Provide exactly two contexts: --contexts left,right");
                std::process::exit(1);
            }
            let (leftctx, rightctx) = (parts[0], parts[1]);

            eprintln!("[~] Collecting from {}", leftctx);
            let leftjson = collect(leftctx, &ns);

            eprintln!("[~] Collecting from {}", rightctx);
            let rightjson = collect(rightctx, &ns);

            eprintln!("[~] Comparing deployments in namespace {}", ns);
            let result = diff::compare_resources(&leftjson, &rightjson);
            if output == "json" {
                result.print_json(leftctx, rightctx);
            } else {
                result.print(leftctx, rightctx);
            }
        }
        Commands::Version => {
            println!("k8sync {}", env!("CARGO_PKG_VERSION"));
        }
        Commands::List => {
            let output = Command::new("kubectl")
                .args(["config", "get-contexts", "-o", "name"])
                .output();

            match output {
                Ok(out) if out.status.success() => {
                    println!("[+] Available contexts:");
                    let text = String::from_utf8_lossy(&out.stdout);
                    for line in text.lines() {
                        println!("    {}", line);
                    }
                }
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr);
                    eprintln!("[!] kubectl failed: {}", err);
                    std::process::exit(1);
                }
                Err(e) => {
                    eprintln!("[!] Failed to run kubectl: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}

fn collect(ctx: &str, namespace: &str) -> serde_json::Value {
    let collector = find_collector();
    let out = Command::new(&collector)
        .args(["-context", ctx, "-namespace", namespace])
        .output()
        .unwrap_or_else(|e| {
            eprintln!("[!] Failed to run collector ({}): {}", collector, e);
            std::process::exit(1);
        });

    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        eprintln!("[!] Collector failed for {}: {}", ctx, err.trim());
        std::process::exit(1);
    }

    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        eprintln!("[!] Failed to parse collector output: {}", e);
        std::process::exit(1);
    })
}

fn find_collector() -> String {
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let candidate = dir.join("collector");
        if candidate.exists() {
            return candidate.to_string_lossy().into_owned();
        }
    }
    "collector".to_string()
}
