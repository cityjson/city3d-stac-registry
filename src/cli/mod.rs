#![allow(clippy::uninlined_format_args)]
//! Command-line interface

use crate::config::{CollectionCliArgs, CollectionConfigFile};
use crate::error::{CityJsonStacError, Result};
use crate::reader::{get_reader, get_reader_from_source, InputSource};
use crate::stac::{StacCollectionBuilder, StacItemBuilder};
use crate::traversal;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "citystac")]
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
    ///
    /// The input can be a local file path or a remote URL (http://, https://)
    Item {
        /// Input file path or URL
        input: String,

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

        /// Base URL for asset href (e.g., "https://example.com/data/")
        /// If provided, asset hrefs will be absolute URLs
        #[arg(long)]
        base_url: Option<String>,

        /// Pretty-print JSON
        #[arg(long, default_value_t = true)]
        pretty: bool,
    },

    /// Generate STAC Collection from directory
    Collection {
        /// Input paths (directories, files, or glob patterns like "data/*.json")
        #[arg(num_args = 0..)]
        inputs: Vec<PathBuf>,

        /// Output directory
        #[arg(short, long, default_value = "./stac_output")]
        output: PathBuf,

        /// YAML configuration file for collection metadata
        #[arg(short = 'C', long)]
        config: Option<PathBuf>,

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

        /// Glob patterns to include (e.g., "*.json", "*.jsonl")
        #[arg(long)]
        include: Vec<String>,

        /// Glob patterns to exclude (e.g., "*test*", "*.bak")
        #[arg(long)]
        exclude: Vec<String>,

        /// Scan subdirectories recursively
        #[arg(short, long, default_value_t = true)]
        recursive: bool,

        /// Maximum directory depth
        #[arg(long)]
        max_depth: Option<usize>,

        /// Skip files with errors
        #[arg(long, default_value_t = true)]
        skip_errors: bool,

        /// Base URL for asset href (e.g., "https://example.com/data/")
        /// If provided, asset hrefs will be absolute URLs
        #[arg(long)]
        base_url: Option<String>,

        /// Pretty-print JSON
        #[arg(long, default_value_t = true)]
        pretty: bool,
    },

    /// Generate STAC Collection from a list of existing STAC item files
    ///
    /// This command is useful when STAC items are generated individually (e.g., for
    /// assets stored in Object Storage) and then need to be aggregated into a collection.
    /// It reads the CityJSON extension properties from each item and merges them.
    #[command(visible_alias = "aggregate")]
    UpdateCollection {
        /// STAC item JSON files to aggregate
        #[arg(required = true)]
        items: Vec<PathBuf>,

        /// Output file path for the collection (collection.json)
        #[arg(short, long, default_value = "./collection.json")]
        output: PathBuf,

        /// YAML configuration file for collection metadata
        #[arg(short = 'C', long)]
        config: Option<PathBuf>,

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

        /// Base URL for item links (e.g., "https://example.com/stac/items/")
        /// If provided, item links will be absolute URLs
        #[arg(long)]
        items_base_url: Option<String>,

        /// Skip items with parsing errors
        #[arg(long, default_value_t = true)]
        skip_errors: bool,

        /// Pretty-print JSON
        #[arg(long, default_value_t = true)]
        pretty: bool,
    },

    /// Generate STAC Catalog from multiple directories/collections
    Catalog {
        /// Input directories (each directory will be a collection)
        #[arg(num_args = 0..)]
        inputs: Vec<PathBuf>,

        /// Output directory for the catalog
        #[arg(short, long, default_value = "./catalog")]
        output: PathBuf,

        /// YAML/TOML configuration file for catalog metadata
        #[arg(short = 'C', long)]
        config: Option<PathBuf>,

        /// Catalog ID (defaults to output directory name)
        #[arg(long)]
        id: Option<String>,

        /// Catalog title
        #[arg(long)]
        title: Option<String>,

        /// Catalog description
        #[arg(short, long)]
        description: Option<String>,

        /// Configuration for collections (license, etc.)
        /// This will be applied to all generated sub-collections
        #[arg(short, long, default_value = "proprietary")]
        license: String,

        /// Base URL for catalog child links
        #[arg(long)]
        base_url: Option<String>,

        /// Pretty-print JSON
        #[arg(long, default_value_t = true)]
        pretty: bool,
    },
}

/// Run the CLI application
pub async fn run() -> Result<()> {
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
            input,
            output,
            id,
            title,
            description,
            collection,
            base_url,
            pretty,
        } => {
            handle_item_command(
                input,
                output,
                id,
                title,
                description,
                collection,
                base_url,
                pretty,
            )
            .await
        }

        Commands::Collection {
            inputs,
            output,
            config,
            id,
            title,
            description,
            license,
            include,
            exclude,
            recursive,
            max_depth,
            skip_errors,
            base_url,
            pretty,
        } => {
            // Check if no inputs provided via CLI and no config file
            if inputs.is_empty() && config.is_none() {
                // No inputs in CLI and no config file - show error
                eprintln!("Error: No inputs provided. Specify inputs via CLI arguments or in a config file.");
                eprintln!("Usage: citystac collection [OPTIONS] <INPUTS>...");
                eprintln!("       citystac collection --config <CONFIG_FILE>");
                std::process::exit(1);
            }

            handle_collection_command(CollectionConfig {
                inputs,
                output,
                config,
                id,
                title,
                description,
                license,
                include,
                exclude,
                recursive,
                max_depth,
                skip_errors,
                base_url,
                pretty,
            })
        }

        Commands::UpdateCollection {
            items,
            output,
            config,
            id,
            title,
            description,
            license,
            items_base_url,
            skip_errors,
            pretty,
        } => handle_update_collection_command(UpdateCollectionConfig {
            items,
            output,
            config,
            id,
            title,
            description,
            license,
            items_base_url,
            skip_errors,
            pretty,
        }),

        Commands::Catalog {
            inputs,
            output,
            config,
            id,
            title,
            description,
            license,
            base_url,
            pretty,
        } => handle_catalog_command(CatalogConfig {
            inputs,
            output,
            config,
            id,
            title,
            description,
            license,
            base_url,
            pretty,
        }),
    }
}

#[allow(clippy::too_many_arguments)]
async fn handle_item_command(
    input: String,
    output: Option<PathBuf>,
    id: Option<String>,
    title: Option<String>,
    description: Option<String>,
    collection: Option<String>,
    base_url: Option<String>,
    pretty: bool,
) -> Result<()> {
    log::info!("Processing input: {}", input);

    // Parse input as either local file or remote URL
    let source = InputSource::from_str_input(&input)?;
    let reader = get_reader_from_source(&source).await?;
    log::debug!("Using {} reader", reader.encoding());

    // Build STAC Item
    // For remote URLs, use the virtual path from the reader
    let mut builder =
        StacItemBuilder::from_file(reader.file_path(), reader.as_ref(), base_url.as_deref())?;

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
        builder = builder.collection_link(format!("./{coll_id}.json"));
    }

    // Generate output path
    let output_path = output.unwrap_or_else(|| {
        // For URLs, use a filename derived from the URL
        // For local files, use the file path with .item.json extension
        match source {
            InputSource::Local(path) => {
                let mut p = path.clone();
                p.set_extension("item.json");
                p
            }
            InputSource::Remote(url) => {
                let filename = url
                    .split('/')
                    .next_back()
                    .and_then(|s| s.split('?').next())
                    .unwrap_or("remote.item.json");
                PathBuf::from(format!("{}.json", filename.trim_end_matches(".json")))
            }
        }
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

/// Configuration for catalog generation
struct CatalogConfig {
    inputs: Vec<PathBuf>,
    output: PathBuf,
    config: Option<PathBuf>,
    id: Option<String>,
    title: Option<String>,
    description: Option<String>,
    license: String,
    base_url: Option<String>,
    pretty: bool,
}

fn handle_catalog_command(config: CatalogConfig) -> Result<()> {
    use crate::config::{CatalogCliArgs, CatalogConfigFile};
    use crate::stac::StacCatalogBuilder;

    // Load config file if provided
    let base_config = if let Some(config_path) = &config.config {
        CatalogConfigFile::from_file(config_path)?
    } else {
        CatalogConfigFile::default()
    };

    // Merge with CLI args
    let merged_config = base_config.merge_with_cli(&CatalogCliArgs {
        id: config.id.clone(),
        title: config.title.clone(),
        description: config.description.clone(),
    });

    // Create output directory
    std::fs::create_dir_all(&config.output)?;

    // Determine collections to process
    let mut collection_targets: Vec<(PathBuf, String)> = Vec::new(); // (path, id_hint)

    // Process CLI inputs (directories)
    for input in &config.inputs {
        let id_hint = input
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("collection")
            .to_string();
        collection_targets.push((input.clone(), id_hint));
    }

    // Process config collections
    if let Some(config_collections) = merged_config.collections {
        // Resolve paths relative to config file if provided, otherwise CWD
        let base_dir = config
            .config
            .as_ref()
            .and_then(|p| p.parent())
            .unwrap_or_else(|| std::path::Path::new("."));

        for coll_path_str in config_collections {
            let path = base_dir.join(&coll_path_str);
            let id_hint = std::path::Path::new(&coll_path_str)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("collection")
                .to_string();
            collection_targets.push((path, id_hint));
        }
    }

    if collection_targets.is_empty() {
        eprintln!("Error: No collections provided. Specify input directories via CLI or 'collections' in config file.");
        std::process::exit(1);
    }

    log::info!(
        "Processing {} collections for catalog",
        collection_targets.len()
    );

    let mut generated_collections: Vec<(String, String)> = Vec::new(); // (href, title)

    for (input_dir, id_hint) in collection_targets {
        if !input_dir.exists() {
            log::warn!("Input directory does not exist: {}", input_dir.display());
            continue;
        }

        let collection_output_dir = config.output.join(&id_hint);

        let sub_collection_config = CollectionConfig {
            inputs: vec![input_dir.clone()],
            output: collection_output_dir.clone(),
            config: None,
            id: Some(id_hint.clone()),
            title: Some(format!("Collection from {}", id_hint)),
            description: None,
            license: config.license.clone(),
            include: vec![],
            exclude: vec![],
            recursive: true,
            max_depth: None,
            skip_errors: true,
            base_url: config.base_url.clone().map(|u| format!("{u}{id_hint}/")),
            pretty: config.pretty,
        };

        println!("Processing collection: {}", input_dir.display());
        match process_collection_logic(sub_collection_config) {
            Ok((_col_path, col_id, col_title)) => {
                let relative_href = format!("./{}/collection.json", id_hint);

                let href = if let Some(base) = &config.base_url {
                    let normalized_base = if base.ends_with('/') {
                        base.to_string()
                    } else {
                        format!("{base}/")
                    };
                    format!("{normalized_base}{id_hint}/collection.json")
                } else {
                    relative_href
                };

                generated_collections.push((href, col_title.unwrap_or(col_id)));
            }
            Err(e) => {
                log::error!(
                    "Failed to generate collection for {}: {}",
                    input_dir.display(),
                    e
                );
            }
        }
    }

    // Generate Catalog
    let catalog_id = merged_config.id.unwrap_or_else(|| {
        config
            .output
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("catalog")
            .to_string()
    });

    let description = merged_config
        .description
        .unwrap_or_else(|| "Root catalog".to_string());

    let mut catalog_builder = StacCatalogBuilder::new(catalog_id, description);

    if let Some(t) = merged_config.title {
        catalog_builder = catalog_builder.title(t);
    }

    for (href, title) in generated_collections {
        catalog_builder = catalog_builder.child_link(href, Some(title));
    }

    catalog_builder = catalog_builder.self_link("./catalog.json");

    let catalog = catalog_builder.build();
    let catalog_json = if config.pretty {
        serde_json::to_string_pretty(&catalog)?
    } else {
        serde_json::to_string(&catalog)?
    };

    let catalog_path = config.output.join("catalog.json");
    std::fs::write(&catalog_path, catalog_json)?;

    println!("\n✓ Generated Catalog: {}", catalog_path.display());

    Ok(())
}

/// Configuration for collection generation
struct CollectionConfig {
    inputs: Vec<PathBuf>,
    output: PathBuf,
    config: Option<PathBuf>,
    id: Option<String>,
    title: Option<String>,
    description: Option<String>,
    license: String,
    include: Vec<String>,
    exclude: Vec<String>,
    recursive: bool,
    max_depth: Option<usize>,
    skip_errors: bool,
    base_url: Option<String>,
    pretty: bool,
}

fn handle_collection_command(config: CollectionConfig) -> Result<()> {
    match process_collection_logic(config) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

fn process_collection_logic(config: CollectionConfig) -> Result<(PathBuf, String, Option<String>)> {
    // Load config file if provided
    let base_config = if let Some(config_path) = &config.config {
        CollectionConfigFile::from_file(config_path)?
    } else {
        CollectionConfigFile::default()
    };

    // Merge with CLI args
    let merged_config = base_config.merge_with_cli(&CollectionCliArgs {
        id: config.id.clone(),
        title: config.title.clone(),
        description: config.description.clone(),
        license: if config.license != "proprietary" {
            Some(config.license.clone())
        } else {
            None
        },
    });

    // Determine final inputs: CLI inputs take precedence, fall back to config inputs
    let final_inputs = if !config.inputs.is_empty() {
        // CLI inputs provided - use them
        config.inputs.clone()
    } else if let Some(config_inputs) = merged_config.inputs {
        // No CLI inputs, but config file has inputs
        config_inputs
            .iter()
            .map(|s| PathBuf::from(s.as_str()))
            .collect()
    } else {
        // No inputs anywhere - this should have been caught earlier
        return Err(CityJsonStacError::StacError(
            "No inputs provided".to_string(),
        ));
    };

    log::info!("Scanning {} input(s)", final_inputs.len());

    // Find all supported files with pattern matching
    let files = traversal::find_files_with_patterns(
        &final_inputs,
        &config.include,
        &config.exclude,
        config.recursive,
        config.max_depth,
    )?;

    if files.is_empty() {
        return Err(crate::error::CityJsonStacError::NoFilesFound);
    }

    println!("Found {} files", files.len());

    // Detect filename collisions: count how many times each file stem appears
    let mut stem_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for file in &files {
        if let Some(stem) = file.file_stem().and_then(|s| s.to_str()) {
            *stem_counts.entry(stem.to_string()).or_insert(0) += 1;
        }
    }

    // Process each file and collect readers
    let mut readers: Vec<Box<dyn crate::reader::CityModelMetadataReader>> = Vec::new();
    // (file_path, item_json, item_id) - we now track item_id separately
    let mut items_data: Vec<(PathBuf, String, String)> = Vec::new();
    let mut errors: Vec<(PathBuf, String)> = Vec::new();

    for file in &files {
        match get_reader(file) {
            Ok(reader) => {
                log::debug!("Processing: {}", file.display());

                // Check if this file stem has collisions
                let stem = file
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");
                let has_collision = stem_counts.get(stem).is_some_and(|&count| count > 1);

                // Generate STAC Item - use format suffix if there are collisions
                let builder_result = if has_collision {
                    StacItemBuilder::from_file_with_format_suffix(
                        file,
                        reader.as_ref(),
                        config.base_url.as_deref(),
                    )
                } else {
                    StacItemBuilder::from_file(file, reader.as_ref(), config.base_url.as_deref())
                };

                match builder_result {
                    Ok(builder) => match builder.build() {
                        Ok(item) => {
                            let item_id = item.id.clone();
                            let json = if config.pretty {
                                serde_json::to_string_pretty(&item)?
                            } else {
                                serde_json::to_string(&item)?
                            };
                            items_data.push((file.to_path_buf(), json, item_id));
                            readers.push(reader);
                            print!(".");
                        }
                        Err(e) => {
                            if config.skip_errors {
                                errors.push((file.to_path_buf(), e.to_string()));
                                log::warn!("Skipping {}: {}", file.display(), e);
                                print!("!");
                            } else {
                                return Err(e);
                            }
                        }
                    },
                    Err(e) => {
                        if config.skip_errors {
                            errors.push((file.to_path_buf(), e.to_string()));
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
                    errors.push((file.to_path_buf(), e.to_string()));
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
    let collection_id = merged_config.id.unwrap_or_else(|| {
        // For multiple inputs, try to use the first input's name
        // or fall back to "collection"
        final_inputs
            .first()
            .and_then(|p| p.file_name().and_then(|n| n.to_str()))
            .unwrap_or("collection")
            .to_string()
    });

    let license = merged_config
        .license
        .clone()
        .unwrap_or_else(|| config.license.clone());

    let mut collection_builder = StacCollectionBuilder::new(&collection_id)
        .license(license)
        .temporal_extent(Some(chrono::Utc::now()), None)
        .aggregate_cityjson_metadata(&readers)?;

    // Apply config-based metadata
    if let Some(t) = &merged_config.title {
        collection_builder = collection_builder.title(t.clone());
    }

    if let Some(d) = &merged_config.description {
        collection_builder = collection_builder.description(d.clone());
    }

    if let Some(keywords) = &merged_config.keywords {
        collection_builder = collection_builder.keywords(keywords.clone());
    }

    if let Some(providers) = &merged_config.providers {
        for provider in providers {
            collection_builder = collection_builder.provider(provider.clone().into());
        }
    }

    // Create output directory
    std::fs::create_dir_all(&config.output)?;
    let items_dir = config.output.join("items");
    std::fs::create_dir_all(&items_dir)?;

    // Write items and add links to collection
    for (file_path, item_json, item_id) in &items_data {
        let item_filename = format!("{item_id}_item.json");

        let item_path = items_dir.join(&item_filename);
        std::fs::write(&item_path, item_json)?;

        // Add item link to collection
        collection_builder = collection_builder.item_link(
            format!("./items/{item_filename}"),
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

    Ok((collection_path, collection_id, merged_config.title))
}

/// Configuration for update-collection/aggregate command
struct UpdateCollectionConfig {
    items: Vec<PathBuf>,
    output: PathBuf,
    config: Option<PathBuf>,
    id: Option<String>,
    title: Option<String>,
    description: Option<String>,
    license: String,
    items_base_url: Option<String>,
    skip_errors: bool,
    pretty: bool,
}

fn handle_update_collection_command(config: UpdateCollectionConfig) -> Result<()> {
    // Load config file if provided
    let base_config = if let Some(config_path) = &config.config {
        CollectionConfigFile::from_file(config_path)?
    } else {
        CollectionConfigFile::default()
    };

    // Merge with CLI args
    let merged_config = base_config.merge_with_cli(&CollectionCliArgs {
        id: config.id.clone(),
        title: config.title.clone(),
        description: config.description.clone(),
        license: if config.license != "proprietary" {
            Some(config.license.clone())
        } else {
            None
        },
    });

    log::info!(
        "Aggregating {} STAC items into collection",
        config.items.len()
    );

    if config.items.is_empty() {
        return Err(crate::error::CityJsonStacError::StacError(
            "No STAC item files provided".to_string(),
        ));
    }

    // Parse all STAC items
    let mut parsed_items: Vec<crate::stac::StacItem> = Vec::new();
    let mut errors: Vec<(PathBuf, String)> = Vec::new();

    for item_path in &config.items {
        match std::fs::read_to_string(item_path) {
            Ok(content) => match serde_json::from_str::<crate::stac::StacItem>(&content) {
                Ok(item) => {
                    log::debug!("Parsed STAC item: {} from {}", item.id, item_path.display());
                    parsed_items.push(item);
                    print!(".");
                }
                Err(e) => {
                    if config.skip_errors {
                        errors.push((item_path.clone(), e.to_string()));
                        log::warn!("Skipping {}: {}", item_path.display(), e);
                        print!("!");
                    } else {
                        return Err(crate::error::CityJsonStacError::JsonError(e));
                    }
                }
            },
            Err(e) => {
                if config.skip_errors {
                    errors.push((item_path.clone(), e.to_string()));
                    log::warn!("Skipping {}: {}", item_path.display(), e);
                    print!("!");
                } else {
                    return Err(crate::error::CityJsonStacError::IoError(e));
                }
            }
        }
    }
    println!(); // New line after progress dots

    if parsed_items.is_empty() {
        return Err(crate::error::CityJsonStacError::StacError(
            "No valid STAC items could be parsed".to_string(),
        ));
    }

    // Generate collection ID from first item or output filename
    let collection_id = merged_config.id.unwrap_or_else(|| {
        config
            .output
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("collection")
            .to_string()
    });

    let license = merged_config
        .license
        .unwrap_or_else(|| config.license.clone());

    // Build collection by aggregating item metadata
    let mut collection_builder = StacCollectionBuilder::new(&collection_id)
        .license(license)
        .temporal_extent(Some(chrono::Utc::now()), None)
        .aggregate_from_items(&parsed_items)?;

    // Apply config-based metadata
    if let Some(t) = merged_config.title {
        collection_builder = collection_builder.title(t);
    }

    if let Some(d) = merged_config.description {
        collection_builder = collection_builder.description(d);
    }

    if let Some(keywords) = merged_config.keywords {
        collection_builder = collection_builder.keywords(keywords);
    }

    if let Some(providers) = merged_config.providers {
        for provider in providers {
            collection_builder = collection_builder.provider(provider.into());
        }
    }

    // Add item links
    for (item_path, item) in config.items.iter().zip(parsed_items.iter()) {
        let fallback_filename = format!("{}.json", item.id);
        let item_filename = item_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&fallback_filename);

        let href = match &config.items_base_url {
            Some(base) => {
                // Ensure base URL ends with a slash
                let normalized_base = if base.ends_with('/') {
                    base.to_string()
                } else {
                    format!("{base}/")
                };
                format!("{normalized_base}{item_filename}")
            }
            None => {
                // Use relative path from collection to item
                format!("./{item_filename}")
            }
        };

        collection_builder = collection_builder.item_link(href, Some(item.id.clone()));
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

    // Create parent directory if needed
    if let Some(parent) = config.output.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    std::fs::write(&config.output, collection_json)?;

    // Print summary
    println!(
        "\n✓ Aggregated {} items into collection",
        parsed_items.len()
    );
    if !errors.is_empty() {
        println!("⚠ {} items skipped due to errors", errors.len());
    }
    println!("Collection: {}", config.output.display());

    Ok(())
}
