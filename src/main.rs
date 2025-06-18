mod collection;
mod commands;
mod db;
mod entry;
mod types;

use clap::{Parser, Subcommand};
use std::path::{PathBuf};

#[derive(Parser)]
#[command(name = "simdex")]
#[command(about = "A tool to manage scientific data", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan & sync simulation data into the cache database
    Sync {
        #[arg(default_value = ".")]
        root: PathBuf,
        #[arg(short, long, default_value = "simdex.db")]
        db: PathBuf,
    },

    LsCollections {
        #[arg(short, long, default_value = "simdex.db")]
        db: PathBuf,
    },

    LsParams {
        #[arg(short, long)]
        db: PathBuf,
        #[arg()]
        collection: String,
    },

    Migrate {
        #[arg(default_value = ".")]
        root: PathBuf,
    },

    Display {
        #[arg(short, long, default_value = "simdex.db")]
        db_path: PathBuf,
        #[arg()]
        collection: String,
    }
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Sync { root, db } => commands::sync(root, db),
        Commands::LsCollections { db } => commands::ls_collections(db),
        Commands::LsParams { db, collection } => commands::ls_params(db, collection),
        Commands::Migrate { root } => commands::migrate(root),
        Commands::Display {
            db_path,
            collection,
        } => commands::display(db_path, collection),
    }
}
