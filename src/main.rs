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
    Scan {
        #[arg(default_value = ".")]
        root: PathBuf,
        #[arg(short, long, default_value = "simdex.db")]
        db: PathBuf,
    },

    Ls {
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
    // Ds {
    //     #[arg()]
    //     uid: String,
    // },
    Create {
        #[arg()]
        path: PathBuf,
        #[arg()]
        uid: String,
    },
}
fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Scan { root, db } => simdex::api::scan(root, db),
        Commands::Ls { db } => simdex::api::ls_collections(db),
        Commands::LsParams { db, collection } => simdex::api::ls_params(db, collection),
        Commands::Migrate { root } => simdex::api::migrate(root),
        Commands::Display {
            db_path,
            collection,
        } => simdex::api::display(db_path, collection),
        // Commands::Ds { uid } => simdex::api::display_polars(uid),

        // create returns a Result, so we handle the error
        Commands::Create { path, uid } => {
            if let Err(e) = simdex::core::discovery::new_collection(path, uid) {
                eprintln!("Error: {}", e);
            }
        }
    }
}
