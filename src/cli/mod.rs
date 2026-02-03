//! Command-line interface

use crate::error::Result;
use crate::reader::get_reader;
use crate::stac::{StacCollectionBuilder, StacItemBuilder};
use crate::traversal;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "cityjson-stac")]
#[command(author, version, about = "Generate STAC metadata for CityJSON datasets", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate STAC Item from a single file
    Item {
        /// Input file path
        file: PathBuf,

        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// STAC Item ID
        #[arg(long)]
        id: Option<String>,

        /// Item title
        #[arg(long)]
        title: Option<String>,

        /// Item description
        #[arg(short, long)]
        description: Option<String>,

        /// Parent collection ID
        #[arg(short, long)]
        collection: Option<String>,

        /// Pretty-print JSON
        #[arg(long, default_value_t = true)]
        pretty: bool,
    },

    /// Generate STAC Collection from directory
    Collection {
        /// Directory to scan
        directory: PathBuf,

        /// Output directory
        #[arg(short, long, default_value = "./stac_output")]
        output: PathBuf,

        /// Collection ID
        #[arg(long)]
        id: Option<String>,

        /// Collection title
        #[arg(long)]
        title: Option<String>,

        /// Collection description
        #[arg(short, long)]
        description: Option<String>,

        /// Data license
        #[arg(short, long, default_value = "proprietary")]
        license: String,

        /// Scan subdirectories recursively
        #[arg(short, long, default_value_t = true)]
        recursive: bool,

        /// Maximum directory depth
        #[arg(long)]
        max_depth: Option<usize>,

        /// Skip files with errors
        #[arg(long)]
        skip_errors: bool,

        /// Pretty-print JSON
        #[arg(long, default_value_t = true)]
        pretty: bool,
    },
}

/// Run the CLI application
pub fn run() -> Result<()> {
    let cli = Cli::parse();

    // Set up logging based on verbosity
    if cli.verbose {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();
    } else {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .init();
    }

    match cli.command {
        Commands::Item {
            file,
            output,
            id,
            title,
            description,
            collection,
            pretty,
        } => handle_item_command(file, output, id, title, description, collection, pretty),

        Commands::Collection {
            directory,
            output,
            id,
            title,
            description,
            license,
            recursive,
            max_depth,
            skip_errors,
            pretty,
        } => handle_collection_command(CollectionConfig {
            directory,
            output,
            id,
            title,
            description,
            license,
            recursive,
            max_depth,
            skip_errors,
            pretty,
        }),
    }
}

fn handle_item_command(
    file: PathBuf,
    output: Option<PathBuf>,
    id: Option<String>,
    title: Option<String>,
    description: Option<String>,
    collection: Option<String>,
    pretty: bool,
) -> Result<()> {
    log::info!("Processing file: {}", file.display());

    // Validate file exists
    if !file.exists() {
        return Err(crate::error::CityJsonStacError::IoError(
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", file.display()),
            ),
        ));
    }

    // Get reader for the file
    let reader = get_reader(&file)?;
    log::debug!("Using {} reader", reader.encoding());

    // Build STAC Item
    let mut builder = StacItemBuilder::from_file(&file, reader.as_ref())?;

    // Apply custom options
    if let Some(custom_id) = id {
        builder = StacItemBuilder::new(custom_id).cityjson_metadata(reader.as_ref())?;

        if let Ok(bbox) = reader.bbox() {
            builder = builder.bbox(bbox).geometry_from_bbox();
        }
    }

    if let Some(t) = title {
        builder = builder.title(t);
    }

    if let Some(d) = description {
        builder = builder.description(d);
    }

    // Add collection link if specified
    if let Some(coll_id) = collection {
        builder = builder.collection_link(format!("./{}.json", coll_id));
    }

    // Generate output path
    let output_path = output.unwrap_or_else(|| {
        let mut path = file.clone();
        path.set_extension("item.json");
        path
    });

    // Build and serialize
    let item = builder.build()?;
    let json = if pretty {
        serde_json::to_string_pretty(&item)?
    } else {
        serde_json::to_string(&item)?
    };

    // Write output
    std::fs::write(&output_path, json)?;

    println!("✓ Generated STAC Item: {}", output_path.display());

    Ok(())
}

/// Configuration for collection generation
struct CollectionConfig {
    directory: PathBuf,
    output: PathBuf,
    id: Option<String>,
    title: Option<String>,
    description: Option<String>,
    license: String,
    recursive: bool,
    max_depth: Option<usize>,
    skip_errors: bool,
    pretty: bool,
}

fn handle_collection_command(config: CollectionConfig) -> Result<()> {
    log::info!("Scanning directory: {}", config.directory.display());

    // Find all supported files
    let files = traversal::find_files(&config.directory, config.recursive, config.max_depth)?;

    if files.is_empty() {
        return Err(crate::error::CityJsonStacError::NoFilesFound);
    }

    println!("Found {} files", files.len());

    // Process each file and collect readers
    let mut readers: Vec<Box<dyn crate::reader::CityModelMetadataReader>> = Vec::new();
    let mut items_data: Vec<(PathBuf, String)> = Vec::new(); // (file_path, item_json)
    let mut errors: Vec<(PathBuf, String)> = Vec::new();

    for file in &files {
        match get_reader(file) {
            Ok(reader) => {
                log::debug!("Processing: {}", file.display());

                // Generate STAC Item for this file
                match StacItemBuilder::from_file(file, reader.as_ref()) {
                    Ok(builder) => match builder.build() {
                        Ok(item) => {
                            let json = if config.pretty {
                                serde_json::to_string_pretty(&item)?
                            } else {
                                serde_json::to_string(&item)?
                            };
                            items_data.push((file.clone(), json));
                            readers.push(reader);
                            print!(".");
                        }
                        Err(e) => {
                            if config.skip_errors {
                                errors.push((file.clone(), e.to_string()));
                                log::warn!("Skipping {}: {}", file.display(), e);
                                print!("!");
                            } else {
                                return Err(e);
                            }
                        }
                    },
                    Err(e) => {
                        if config.skip_errors {
                            errors.push((file.clone(), e.to_string()));
                            log::warn!("Skipping {}: {}", file.display(), e);
                            print!("!");
                        } else {
                            return Err(e);
                        }
                    }
                }
            }
            Err(e) => {
                if config.skip_errors {
                    errors.push((file.clone(), e.to_string()));
                    log::warn!("Skipping {}: {}", file.display(), e);
                    print!("!");
                } else {
                    return Err(e);
                }
            }
        }
    }
    println!(); // New line after progress dots

    // Build collection
    let collection_id = config.id.unwrap_or_else(|| {
        config
            .directory
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("collection")
            .to_string()
    });

    let mut collection_builder = StacCollectionBuilder::new(&collection_id)
        .license(config.license)
        .aggregate_cityjson_metadata(&readers)?;

    if let Some(t) = config.title {
        collection_builder = collection_builder.title(t);
    }

    if let Some(d) = config.description {
        collection_builder = collection_builder.description(d);
    }

    // Create output directory
    std::fs::create_dir_all(&config.output)?;
    let items_dir = config.output.join("items");
    std::fs::create_dir_all(&items_dir)?;

    // Write items and add links to collection
    for (file_path, item_json) in &items_data {
        let item_filename = format!(
            "{}_item.json",
            file_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("item")
        );

        let item_path = items_dir.join(&item_filename);
        std::fs::write(&item_path, item_json)?;

        // Add item link to collection
        collection_builder = collection_builder.item_link(
            format!("./items/{}", item_filename),
            file_path
                .file_name()
                .and_then(|n| n.to_str())
                .map(String::from),
        );
    }

    // Add self link
    collection_builder = collection_builder.self_link("./collection.json");

    // Build and write collection
    let collection = collection_builder.build()?;
    let collection_json = if config.pretty {
        serde_json::to_string_pretty(&collection)?
    } else {
        serde_json::to_string(&collection)?
    };

    let collection_path = config.output.join("collection.json");
    std::fs::write(&collection_path, collection_json)?;

    // Print summary
    println!("\n✓ Generated {} items", items_data.len());
    if !errors.is_empty() {
        println!("⚠ {} files skipped due to errors", errors.len());
    }
    println!("Collection: {}", collection_path.display());
    println!("Items: {}", items_dir.display());

    Ok(())
}
