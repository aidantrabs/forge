use clap::{Parser, Subcommand};
use forge::config::SiteConfig;
use std::path::Path;

#[derive(Parser)]
#[command(name = "forge", version, about = "a rust static site generator")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Build,
    New { title: String },
    Clean,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build => {
            let config = SiteConfig::load(Path::new("forge.toml"));
            println!("building {}...", config.title);
        }
        Commands::New { title } => {
            println!("creating post: {}", title);
        }
        Commands::Clean => {
            println!("cleaning output directory...");
        }
    }
}
