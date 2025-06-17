mod collection;
mod entry;
mod types;

use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    let root = if args.len() > 1 { &args[1] } else { "." };
    let collections = collection::find_collections(Path::new(root));

    println!("Found {} collections:", collections.len());
    for c in &collections {
        println!("Collection: {:?}", c);
        let entries = collection::find_entries(c);
        for entry in entries {
            println!("   - Entry: {:?}", entry);
            match entry::load_entry_meta(&entry) {
                Some((meta, params)) => {
                    println!("      Meta: {:?}", meta);
                    println!("      Params: {:?}", params);
                }
                None => {
                    println!("      Failed to load entry meta for {:?}", entry);
                }
            }
        }
    }
}
