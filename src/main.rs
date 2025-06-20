mod cli;
use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Sync { root, db } => simdex::api::sync(root, db),
        Commands::LsCollections { db } => simdex::api::ls_collections(db),
        Commands::LsParams { db, collection } => simdex::api::ls_params(db, collection),
        Commands::Migrate { root } => simdex::api::migrate(root),
        Commands::Display {
            db_path,
            collection,
        } => simdex::api::display(db_path, collection),
        // create returns a Result, so we handle the error
        Commands::Create { path, uid } => {
            if let Err(e) = simdex::core::collection::create_collection(path, uid) {
                eprintln!("Error: {}", e);
            }
        }
    }
}
