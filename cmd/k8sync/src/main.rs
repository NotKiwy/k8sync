use clap::{Parser, Subcommand};

mod diff;

#[derive(Parser)]
#[command(name = "k8sync")]
#[command(version = "0.1.0")]
#[command(about = "Kubernetes multi-cluster drift detector",
          long_about = None)]
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
            println!("[+] Comparing clusters -> {}", contexts);
            println!("[+] Namespace -> {}", namespace);
            // TODO: Call Go collector
        }
        Commands::List => {
            println!("[+] Available contexts -> ");
            // TODO: List kubeconfig contexts
        }
    }
}
