use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "simdex")]
#[command(about = "A tool to manage scientific data", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
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
    },
    Create {
        #[arg()]
        path: PathBuf,
        #[arg()]
        uid: String,
    },
}
