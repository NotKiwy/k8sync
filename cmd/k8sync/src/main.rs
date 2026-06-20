use clap::{Parser, Subcommand};
use std::process::Command;

mod diff;

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
        contexts: String,

        #[arg(short, long, default_value = "default")]
        namespace: String,
    },
    List,
}

fn main() {
    let _cli = Cli::parse();

    match &_cli.command {
        Commands::Compare {
            contexts,
            namespace,
        } => {
            let parts: Vec<&str> = contexts.splitn(2, ',').map(str::trim).collect();
            if parts.len() != 2 {
                eprintln!("[!] Provide exactly two contexts: --contexts left,right");
                std::process::exit(1);
            }
            let (leftctx, rightctx) = (parts[0], parts[1]);

            println!("[~] Collecting from {}", leftctx);
            let leftjson = collect(leftctx, namespace);

            println!("[~] Collecting from {}", rightctx);
            let rightjson = collect(rightctx, namespace);

            println!("[~] Comparing deployments in namespace {}", namespace);
            let result = diff::compare_deployments(&leftjson, &rightjson);
            result.print(leftctx, rightctx);
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
