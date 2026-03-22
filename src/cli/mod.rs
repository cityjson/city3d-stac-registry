#![allow(clippy::uninlined_format_args)]
//! Command-line interface

pub mod progress;

use crate::config::{CollectionCliArgs, CollectionConfigFile};
use crate::error::{CityJsonStacError, Result};
use crate::metadata::CRS;
use crate::reader::{get_reader_from_source, InputSource};
use crate::stac::{StacCollectionBuilder, StacItemBuilder};
use crate::traversal;
use clap::{Parser, Subcommand};
use progress::{
    create_progress_bar, create_spinner, finish_spinner_err, finish_spinner_ok, print_banner,
    print_error, print_info, print_success, print_warning, Summary,
};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "citystac")]
#[command(author, version, about = "Generate STAC metadata for CityJSON datasets", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Dry run: validate config and inputs without generating output
    #[arg(long, global = true)]
    dry_run: bool,
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

        /// Overwrite existing item files
        #[arg(long)]
        overwrite_items: bool,

        /// Overwrite existing collection file
        #[arg(long)]
        overwrite_collection: bool,

        /// Overwrite all (items and collection)
        #[arg(long)]
        overwrite: bool,

        /// Generate STAC GeoParquet file (items.parquet) alongside JSON output
        #[arg(long)]
        geoparquet: bool,
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

        /// Generate STAC GeoParquet file (items.parquet) alongside JSON output
        #[arg(long)]
        geoparquet: bool,
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

        /// Overwrite existing item files
        #[arg(long)]
        overwrite_items: bool,

        /// Overwrite existing collection files
        #[arg(long)]
        overwrite_collections: bool,

        /// Overwrite all (items, collections, and catalog)
        #[arg(long)]
        overwrite: bool,

        /// Generate STAC GeoParquet file (items.parquet) alongside JSON output
        #[arg(long)]
        geoparquet: bool,
    },
}

/// Helper to create a GeoParquet asset
fn make_geoparquet_asset() -> crate::stac::Asset {
    let mut asset = crate::stac::Asset::new("./items.parquet");
    asset.r#type = Some("application/vnd.apache.parquet".to_string());
    asset.roles = vec!["collection-mirror".to_string()];
    asset
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
            .filter_level(log::LevelFilter::Warn)
            .init();
    }

    print_banner();

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
                cli.dry_run,
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
            overwrite_items,
            overwrite_collection,
            overwrite,
            geoparquet,
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
                dry_run: cli.dry_run,
                overwrite_items: overwrite_items || overwrite,
                overwrite_collection: overwrite_collection || overwrite,
                geoparquet,
                parent_href: None,
                root_href: None,
            })
            .await
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
            geoparquet,
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
            dry_run: cli.dry_run,
            geoparquet,
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
            overwrite_items,
            overwrite_collections,
            overwrite,
            geoparquet,
        } => {
            handle_catalog_command(CatalogConfig {
                inputs,
                output,
                config,
                id,
                title,
                description,
                license,
                base_url,
                pretty,
                dry_run: cli.dry_run,
                overwrite_items: overwrite_items || overwrite,
                overwrite_collections: overwrite_collections || overwrite,
                geoparquet,
            })
            .await
        }
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
    dry_run: bool,
) -> Result<()> {
    // Dry-run mode: validate only
    if dry_run {
        use crate::validation;
        use progress::{print_banner, print_error, print_success};

        print_banner();

        println!("\nRunning in dry-run mode...\n");

        let result = validation::validate_item_input(&input).await?;

        println!();

        if result.is_valid() {
            print_success("Dry run complete: All validations passed");
            std::process::exit(0);
        } else {
            print_error("Dry run failed: Errors found");
            std::process::exit(result.exit_code());
        }
    }

    // Parse input as either local file or remote URL
    let spinner = create_spinner(format!("Reading {input}…"));
    let source = InputSource::from_str_input(&input)?;
    let reader = match get_reader_from_source(&source).await {
        Ok(r) => r,
        Err(e) => {
            finish_spinner_err(spinner, format!("Failed to read input: {e}"));
            return Err(e);
        }
    };
    finish_spinner_ok(
        spinner,
        format!("Loaded {} ({} format)", input, reader.encoding()),
    );

    let spinner = create_spinner("Building STAC Item…");

    // Build STAC Item
    // For remote URLs, use the original URL as the asset href when no base_url is given
    let original_url = match &source {
        InputSource::Remote(url) => Some(url.as_str()),
        InputSource::Local(_) => None,
    };
    let mut builder = StacItemBuilder::from_file(
        reader.file_path(),
        reader.as_ref(),
        base_url.as_deref(),
        original_url,
    )?;

    // Apply custom options
    if let Some(custom_id) = id {
        builder = StacItemBuilder::new(custom_id).cityjson_metadata(reader.as_ref())?;

        if let Ok(bbox) = reader.bbox() {
            let crs = reader.crs().unwrap_or_default();
            let wgs84_bbox = bbox.to_wgs84(&crs)?;
            builder = builder.bbox(wgs84_bbox).geometry_from_bbox();
        }
    }

    if let Some(t) = title {
        builder = builder.title(t);
    }

    if let Some(d) = description {
        builder = builder.description(d);
    }

    // Add collection link and ID if specified
    if let Some(coll_id) = collection {
        builder = builder
            .collection_id(&coll_id)
            .collection_link(format!("./{coll_id}.json"));
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

    finish_spinner_ok(
        spinner,
        format!("Item written to {}", output_path.display()),
    );

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
    dry_run: bool,
    overwrite_items: bool,
    overwrite_collections: bool,
    geoparquet: bool,
}

/// Sanitize a string for use as a folder name by replacing invalid characters with underscores
fn sanitize_folder_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Extract a folder name from a path string (filename stem)
fn fallback_folder_name(path_str: &str) -> String {
    std::path::Path::new(path_str)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("collection")
        .to_string()
}

async fn handle_catalog_command(config: CatalogConfig) -> Result<()> {
    use crate::config::{CatalogCliArgs, CatalogConfigFile};
    use crate::stac::StacCatalogBuilder;

    // Dry-run mode: validate only
    if config.dry_run {
        use progress::{print_banner, print_error, print_success};

        print_banner();

        println!("\nRunning in dry-run mode...\n");

        // Validate config file if provided
        if let Some(config_path) = &config.config {
            println!("  → Checking config file: {}", config_path.display());
            match CatalogConfigFile::from_file(config_path) {
                Ok(catalog_config) => {
                    println!("  ✓ Config file syntax: valid");

                    // Validate semantic content
                    let mut semantic_errors = Vec::new();

                    if catalog_config.id.is_none()
                        || catalog_config
                            .id
                            .as_ref()
                            .map(|s| s.trim())
                            .unwrap_or_default()
                            .is_empty()
                    {
                        semantic_errors.push("Missing required field: 'id'".to_string());
                    }

                    if catalog_config.title.is_none()
                        || catalog_config
                            .title
                            .as_ref()
                            .map(|s| s.trim())
                            .unwrap_or_default()
                            .is_empty()
                    {
                        semantic_errors.push("Missing recommended field: 'title'".to_string());
                    }

                    if catalog_config.description.is_none()
                        || catalog_config
                            .description
                            .as_ref()
                            .map(|s| s.trim())
                            .unwrap_or_default()
                            .is_empty()
                    {
                        semantic_errors
                            .push("Missing recommended field: 'description'".to_string());
                    }

                    if !semantic_errors.is_empty() {
                        for error in &semantic_errors {
                            println!("  ✗ {}", error);
                        }
                        println!();
                        print_error("Dry run failed: Config semantic errors");
                        std::process::exit(1);
                    }

                    println!("  ✓ Config file content: valid");
                }
                Err(e) => {
                    println!("  ✗ Config file syntax: {}", e);
                    println!();
                    print_error("Dry run failed: Config error");
                    std::process::exit(1);
                }
            }
        }

        // Validate input directories/collections
        let mut found = 0;
        let mut missing = Vec::new();

        for input in &config.inputs {
            if input.exists() {
                found += 1;
            } else {
                missing.push(input.clone());
            }
        }

        if missing.is_empty() {
            println!("  ✓ Input paths: {}/{} found", found, config.inputs.len());
        } else {
            println!("  ⚠ Input paths: {}/{} found", found, config.inputs.len());
            for path in &missing {
                println!("    ✗ {}", path.display());
            }
        }

        println!();

        if missing.is_empty() {
            print_success("Dry run complete: All validations passed");
            std::process::exit(0);
        } else {
            print_error("Dry run failed: Missing paths");
            std::process::exit(2);
        }
    }

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
        base_url: config.base_url.clone(),
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

            // Try to read the id from the config file for the folder name
            let id_hint = if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if matches!(ext, "toml" | "yaml" | "yml") {
                        // Try to parse the config file to get its id
                        match CollectionConfigFile::from_file(&path) {
                            Ok(cfg) => {
                                if let Some(id) = cfg.id {
                                    // Sanitize the id for use as a folder name
                                    sanitize_folder_name(&id)
                                } else {
                                    // No id in config, fall back to filename
                                    fallback_folder_name(&coll_path_str)
                                }
                            }
                            Err(_) => {
                                // Failed to parse, fall back to filename
                                fallback_folder_name(&coll_path_str)
                            }
                        }
                    } else {
                        fallback_folder_name(&coll_path_str)
                    }
                } else {
                    fallback_folder_name(&coll_path_str)
                }
            } else {
                // Directory: use directory name
                fallback_folder_name(&coll_path_str)
            };
            collection_targets.push((path, id_hint));
        }
    }

    if collection_targets.is_empty() {
        print_error("No collections provided. Specify input directories via CLI or 'collections' in config file.");
        std::process::exit(1);
    }

    print_info(format!(
        "Processing {} collection(s) for catalog",
        collection_targets.len()
    ));

    let total_collections = collection_targets.len() as u64;
    let catalog_pb = create_progress_bar(total_collections, "Generating collections…");

    let mut generated_collections: Vec<(String, String)> = Vec::new(); // (href, title)
    let mut catalog_errors: u64 = 0;

    for (input_dir, id_hint) in collection_targets {
        if !input_dir.exists() {
            catalog_pb.println(format!(
                "  {} Directory not found, skipping: {}",
                console::style("⚠").yellow(),
                input_dir.display()
            ));
            catalog_pb.inc(1);
            catalog_errors += 1;
            continue;
        }

        let collection_output_dir = config.output.join(&id_hint);

        let mut collection_config = CollectionConfig {
            inputs: Vec::new(),
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
            base_url: None, // Will be set below based on input type
            pretty: config.pretty,
            dry_run: config.dry_run,
            overwrite_items: config.overwrite_items,
            overwrite_collection: config.overwrite_collections,
            geoparquet: config.geoparquet,
            // Collections under a catalog get parent/root links
            parent_href: Some("../catalog.json".to_string()),
            root_href: Some("../catalog.json".to_string()),
        };

        // Check if input is a config file
        if input_dir.is_file() {
            if let Some(ext) = input_dir.extension().and_then(|e| e.to_str()) {
                if matches!(ext, "toml" | "yaml" | "yml") {
                    catalog_pb.println(format!(
                        "  {} Loading config: {}",
                        console::style("›").blue(),
                        input_dir.display()
                    ));
                    collection_config.config = Some(input_dir.clone());
                    // Don't set base_url here - let the config file's base_url take precedence
                    // via merge_with_cli in process_collection_logic
                } else {
                    collection_config.inputs = vec![input_dir.clone()];
                    // For non-config files, use catalog's base_url if available
                    collection_config.base_url =
                        config.base_url.clone().map(|u| format!("{u}{id_hint}/"));
                }
            } else {
                collection_config.inputs = vec![input_dir.clone()];
                collection_config.base_url =
                    config.base_url.clone().map(|u| format!("{u}{id_hint}/"));
            }
        } else {
            // Directory
            collection_config.inputs = vec![input_dir.clone()];
            // For directories, use catalog's base_url if available
            collection_config.base_url = config.base_url.clone().map(|u| format!("{u}{id_hint}/"));
        }

        catalog_pb.set_message(format!("Processing: {id_hint}"));
        match process_collection_logic(collection_config).await {
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

                catalog_pb.println(format!(
                    "  {} Collection ready: {}",
                    console::style("✓").green(),
                    col_title.clone().unwrap_or_else(|| col_id.clone())
                ));
                generated_collections.push((href, col_title.unwrap_or(col_id)));
            }
            Err(e) => {
                catalog_pb.println(format!(
                    "  {} Failed ({}): {}",
                    console::style("✗").red(),
                    input_dir.display(),
                    e
                ));
                catalog_errors += 1;
            }
        }
        catalog_pb.inc(1);
    }
    catalog_pb.finish_and_clear();

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

    let collection_count = generated_collections.len();
    for (href, title) in generated_collections {
        catalog_builder = catalog_builder.child_link(href, Some(title));
    }

    catalog_builder = catalog_builder
        .self_link("./catalog.json")
        .root_link("./catalog.json");

    let catalog = catalog_builder.build();
    let catalog_json = if config.pretty {
        serde_json::to_string_pretty(&catalog)?
    } else {
        serde_json::to_string(&catalog)?
    };

    let catalog_path = config.output.join("catalog.json");
    std::fs::write(&catalog_path, catalog_json)?;

    Summary::new()
        .add("Catalog", catalog_path.display().to_string())
        .add("Collections", format!("{collection_count}"))
        .add("Errors", format!("{catalog_errors}"))
        .print();
    print_success("Catalog generated successfully");

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
    dry_run: bool,
    overwrite_items: bool,
    overwrite_collection: bool,
    geoparquet: bool,
    /// Parent link href (set when collection is part of a catalog)
    parent_href: Option<String>,
    /// Root link href (set when collection is part of a catalog)
    root_href: Option<String>,
}

async fn handle_collection_command(config: CollectionConfig) -> Result<()> {
    // Dry-run mode: validate only
    if config.dry_run {
        use crate::validation;
        use progress::{print_banner, print_error, print_success};

        print_banner();

        println!("\nRunning in dry-run mode...\n");

        // Determine final inputs
        let base_config = if let Some(config_path) = &config.config {
            // Load config to validate it
            let _base_config = CollectionConfigFile::from_file(config_path)?;
            validation::validate_collection_config(
                &Some(config_path.clone()),
                &config.inputs,
                &config.base_url,
            )
            .await?
        } else {
            validation::validate_collection_config(&None, &config.inputs, &config.base_url).await?
        };

        println!();

        // Print final status
        if base_config.is_valid() {
            print_success("Dry run complete: All validations passed");
            std::process::exit(0);
        } else {
            print_error("Dry run failed: Errors found");
            std::process::exit(base_config.exit_code());
        }
    }

    match process_collection_logic(config).await {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

async fn process_collection_logic(
    config: CollectionConfig,
) -> Result<(PathBuf, String, Option<String>)> {
    use crate::stac::{CollectionAccumulator, ItemMetadata};

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
        base_url: config.base_url.clone(),
    });

    // Determine final inputs: CLI inputs take precedence, fall back to config inputs
    let final_inputs = if !config.inputs.is_empty() {
        // CLI inputs provided - use them
        config.inputs.clone()
    } else if let Some(config_inputs) = merged_config.inputs {
        // No CLI inputs, but config file has inputs
        // Resolve the inputs (may need to read from file if using from_file)
        let config_dir = config
            .config
            .as_ref()
            .and_then(|p| p.parent())
            .unwrap_or(Path::new("."));
        let resolved_inputs = config_inputs.resolve(config_dir)?;
        resolved_inputs
            .iter()
            .map(|s| PathBuf::from(s.as_str()))
            .collect()
    } else {
        return Err(CityJsonStacError::StacError(
            "No inputs provided".to_string(),
        ));
    };

    // Extract CRS override from config (used as fallback when files lack CRS metadata)
    let crs_override: Option<CRS> = merged_config
        .extent
        .as_ref()
        .and_then(|e| e.spatial.as_ref())
        .and_then(|s| s.crs.as_ref())
        .and_then(|crs_str| CRS::from_citygml_srs_name(crs_str));

    // Determine collection ID early so items can reference it
    let collection_id = merged_config.id.clone().unwrap_or_else(|| {
        final_inputs
            .first()
            .and_then(|p| p.file_name().and_then(|n| n.to_str()))
            .unwrap_or("collection")
            .to_string()
    });

    // Check for remote URLs vs local files
    let mut sources: Vec<InputSource> = Vec::new();
    let mut local_search_paths: Vec<PathBuf> = Vec::new();

    for input in &final_inputs {
        let input_str = input.to_string_lossy();
        if crate::remote::is_remote_url(&input_str) {
            sources.push(InputSource::Remote(input_str.to_string()));
        } else {
            local_search_paths.push(input.clone());
        }
    }

    log::info!(
        "Scanning {} local path(s) and {} remote URL(s)",
        local_search_paths.len(),
        sources.len()
    );

    // Find all supported files in local search paths
    if !local_search_paths.is_empty() {
        let files = traversal::find_files_with_patterns(
            &local_search_paths,
            &config.include,
            &config.exclude,
            config.recursive,
            config.max_depth,
        )?;

        // Add found local files to sources
        for file in files {
            sources.push(InputSource::Local(file));
        }
    }

    if sources.is_empty() {
        return Err(crate::error::CityJsonStacError::NoFilesFound);
    }

    print_info(format!("Found {} input source(s)", sources.len()));

    // Detect filename collisions (only relevant for local files really, but let's check all virtual names)
    let mut stem_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    // Pre-scan sources to count filenames for collision detection
    for source in &sources {
        let filename = match source {
            InputSource::Local(p) => p
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string(),
            InputSource::Remote(u) => crate::remote::url_filename(u),
        };
        // Get stem (remove extension)
        let path = PathBuf::from(&filename);
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        *stem_counts.entry(stem.to_string()).or_insert(0) += 1;
    }

    // Create output directories early
    std::fs::create_dir_all(&config.output)?;
    let items_dir = config.output.join("items");
    std::fs::create_dir_all(&items_dir)?;

    // Accumulator for streaming processing
    let mut accumulator = CollectionAccumulator::new();

    // Buffer items for GeoParquet output
    let mut geoparquet_items: Vec<crate::stac::StacItem> = Vec::new();

    // Process each file - write items immediately, accumulate metadata
    let pb = create_progress_bar(sources.len() as u64, "Processing files…");
    for source in &sources {
        let source_desc = match source {
            InputSource::Local(p) => p.display().to_string(),
            InputSource::Remote(u) => u.clone(),
        };
        // Truncate long paths for the progress message
        let short_desc = source_desc
            .split(['/', '\\'])
            .next_back()
            .unwrap_or(&source_desc);
        pb.set_message(format!("Processing: {short_desc}"));

        // First, get the reader to determine the item ID and filename
        let reader = match get_reader_from_source(source).await {
            Ok(r) => r,
            Err(e) => {
                if config.skip_errors {
                    accumulator.add_error(source_desc.clone(), e.to_string());
                    pb.println(format!(
                        "  {} Skipping {short_desc}: {e}",
                        console::style("⚠").yellow()
                    ));
                    pb.inc(1);
                    continue;
                } else {
                    pb.finish_and_clear();
                    return Err(e);
                }
            }
        };

        // Determine item ID and filename
        let file_path = reader.file_path();
        let stem = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let has_collision = stem_counts.get(stem).is_some_and(|&count| count > 1);

        // Generate item ID based on collision detection
        let item_id = if has_collision {
            let encoding = reader.encoding();
            let suffix = match encoding {
                "CityJSON" => "_cj",
                "CityJSONSeq" => "_cjseq",
                "FlatCityBuf" => "_fcb",
                _ => "",
            };
            format!("{}{}", stem, suffix)
        } else {
            stem.to_string()
        };

        let item_filename = format!("{item_id}_item.json");
        let item_path = items_dir.join(&item_filename);

        // Check if item already exists and overwrite flag
        if item_path.exists() && !config.overwrite_items {
            // Skip processing, read existing item for metadata
            pb.println(format!(
                "  {} Skipping existing: {}",
                console::style("⚠").yellow(),
                item_filename
            ));

            match ItemMetadata::from_file(&item_path) {
                Ok(metadata) => {
                    // Buffer existing item for GeoParquet if enabled
                    if config.geoparquet {
                        if let Ok(content) = std::fs::read_to_string(&item_path) {
                            if let Ok(existing_item) =
                                serde_json::from_str::<crate::stac::StacItem>(&content)
                            {
                                geoparquet_items.push(existing_item);
                            }
                        }
                    }

                    let item_href = format!("./items/{item_filename}");
                    let title = file_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(String::from);
                    accumulator.add_item(metadata, item_href, title);
                }
                Err(e) => {
                    // Failed to read existing item - this is an error
                    accumulator.add_error(item_filename.clone(), e.clone());
                    if config.skip_errors {
                        pb.println(format!(
                            "  {} Failed to read existing item: {e}",
                            console::style("✗").red()
                        ));
                    } else {
                        pb.finish_and_clear();
                        return Err(CityJsonStacError::StacError(format!(
                            "Failed to read existing item {}: {}",
                            item_path.display(),
                            e
                        )));
                    }
                }
            }
            pb.inc(1);
            continue;
        }

        // For remote sources, preserve the original URL as the asset href fallback
        let original_url = match source {
            InputSource::Remote(url) => Some(url.as_str()),
            InputSource::Local(_) => None,
        };

        // Process and generate item
        let builder_result = if has_collision {
            StacItemBuilder::from_file_with_format_suffix_and_crs(
                file_path,
                reader.as_ref(),
                config.base_url.as_deref(),
                original_url,
                crs_override.as_ref(),
            )
        } else {
            StacItemBuilder::from_file_with_crs_override(
                file_path,
                reader.as_ref(),
                config.base_url.as_deref(),
                original_url,
                crs_override.as_ref(),
            )
        };

        match builder_result {
            Ok(builder) => match builder
                .collection_id(&collection_id)
                .collection_link("../collection.json")
                .build()
            {
                Ok(item) => {
                    // Buffer item for GeoParquet if enabled
                    if config.geoparquet {
                        geoparquet_items.push(item.clone());
                    }

                    // Extract metadata before writing
                    let metadata = ItemMetadata::from_item(&item);
                    let item_id = item.id.clone();

                    // Serialize item
                    let json = if config.pretty {
                        match serde_json::to_string_pretty(&item) {
                            Ok(j) => j,
                            Err(e) => {
                                if config.skip_errors {
                                    accumulator.add_error(source_desc.clone(), e.to_string());
                                    pb.println(format!(
                                        "  {} Skipping {short_desc}: {e}",
                                        console::style("⚠").yellow()
                                    ));
                                    pb.inc(1);
                                    continue;
                                } else {
                                    pb.finish_and_clear();
                                    return Err(CityJsonStacError::JsonError(e));
                                }
                            }
                        }
                    } else {
                        match serde_json::to_string(&item) {
                            Ok(j) => j,
                            Err(e) => {
                                if config.skip_errors {
                                    accumulator.add_error(source_desc.clone(), e.to_string());
                                    pb.println(format!(
                                        "  {} Skipping {short_desc}: {e}",
                                        console::style("⚠").yellow()
                                    ));
                                    pb.inc(1);
                                    continue;
                                } else {
                                    pb.finish_and_clear();
                                    return Err(CityJsonStacError::JsonError(e));
                                }
                            }
                        }
                    };

                    // Write item immediately to disk
                    let item_filename = format!("{item_id}_item.json");
                    let item_path = items_dir.join(&item_filename);
                    if let Err(e) = std::fs::write(&item_path, &json) {
                        if config.skip_errors {
                            accumulator.add_error(source_desc.clone(), e.to_string());
                            pb.println(format!(
                                "  {} Skipping {short_desc}: {e}",
                                console::style("⚠").yellow()
                            ));
                            pb.inc(1);
                            continue;
                        } else {
                            pb.finish_and_clear();
                            return Err(CityJsonStacError::IoError(e));
                        }
                    }

                    // Add to accumulator
                    let item_href = format!("./items/{item_filename}");
                    let title = file_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(String::from);
                    accumulator.add_item(metadata, item_href, title);
                }
                Err(e) => {
                    if config.skip_errors {
                        accumulator.add_error(source_desc.clone(), e.to_string());
                        pb.println(format!(
                            "  {} Skipping {short_desc}: {e}",
                            console::style("⚠").yellow()
                        ));
                    } else {
                        pb.finish_and_clear();
                        return Err(e);
                    }
                }
            },
            Err(e) => {
                if config.skip_errors {
                    accumulator.add_error(source_desc.clone(), e.to_string());
                    pb.println(format!(
                        "  {} Skipping {short_desc}: {e}",
                        console::style("⚠").yellow()
                    ));
                } else {
                    pb.finish_and_clear();
                    return Err(e);
                }
            }
        }
        pb.inc(1);
    }
    pb.finish_and_clear();

    // Check if collection file exists and overwrite flag
    let collection_path = config.output.join("collection.json");
    if collection_path.exists() && !config.overwrite_collection {
        print_warning(
            "Collection file already exists, skipping (use --overwrite-collection to regenerate)",
        );

        // Still generate GeoParquet if requested
        if config.geoparquet {
            let mut items_for_parquet: Vec<crate::stac::StacItem> = Vec::new();
            let spinner = create_spinner("Reading existing items for GeoParquet…");
            for entry in std::fs::read_dir(&items_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(item) = serde_json::from_str::<crate::stac::StacItem>(&content) {
                            items_for_parquet.push(item);
                        }
                    }
                }
            }
            finish_spinner_ok(
                spinner,
                format!("Read {} item(s) from disk", items_for_parquet.len()),
            );

            if !items_for_parquet.is_empty() {
                // Read existing collection, add geoparquet asset, write back
                let collection_content = std::fs::read_to_string(&collection_path)?;
                let mut collection: crate::stac::StacCollection =
                    serde_json::from_str(&collection_content)?;

                // Add items-geoparquet asset if not already present
                collection
                    .assets
                    .entry("items-geoparquet".to_string())
                    .or_insert_with(make_geoparquet_asset);

                // Write updated collection back
                let updated_json = if config.pretty {
                    serde_json::to_string_pretty(&collection)?
                } else {
                    serde_json::to_string(&collection)?
                };
                std::fs::write(&collection_path, &updated_json)?;

                // Write parquet file
                let parquet_path = config.output.join("items.parquet");
                let spinner = create_spinner("Writing GeoParquet…");
                crate::stac::geoparquet::write_geoparquet(
                    &items_for_parquet,
                    &collection,
                    &parquet_path,
                )?;
                finish_spinner_ok(
                    spinner,
                    format!(
                        "GeoParquet written: {} ({} items)",
                        parquet_path.display(),
                        items_for_parquet.len()
                    ),
                );
            }
        }

        // Return info about existing collection
        return Ok((collection_path, collection_id, merged_config.title));
    }

    // Check for errors - only generate collection if no errors
    if accumulator.has_errors() {
        print_error(format!(
            "Collection generation failed: {} item(s) had errors",
            accumulator.error_count()
        ));

        // Print details about errors
        for (source, error) in &accumulator.errors {
            eprintln!("  {} {}: {}", console::style("✗").red(), source, error);
        }

        return Err(CityJsonStacError::StacError(format!(
            "{} item(s) failed to process",
            accumulator.error_count()
        )));
    }

    // Build collection from accumulated metadata
    let license = merged_config
        .license
        .clone()
        .unwrap_or_else(|| config.license.clone());

    let mut collection_builder = StacCollectionBuilder::new(&collection_id)
        .license(license)
        .temporal_extent(Some(chrono::Utc::now()), None)
        .aggregate_from_metadata(&accumulator.items_metadata)?;

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

    // Add item links from accumulator
    for (href, title) in &accumulator.item_links {
        collection_builder = collection_builder.item_link(href.clone(), title.clone());
    }

    // Add self link
    collection_builder = collection_builder.self_link("./collection.json");

    // Add parent and root links (set when collection is part of a catalog)
    if let Some(parent_href) = &config.parent_href {
        collection_builder = collection_builder.parent_link(parent_href);
    }
    if let Some(root_href) = &config.root_href {
        collection_builder = collection_builder.root_link(root_href);
    }

    // GeoParquet: if buffer is empty (e.g. all items skipped), read items from disk
    if config.geoparquet && geoparquet_items.is_empty() {
        let spinner = create_spinner("Reading existing items for GeoParquet…");
        for entry in std::fs::read_dir(&items_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(item) = serde_json::from_str::<crate::stac::StacItem>(&content) {
                        geoparquet_items.push(item);
                    }
                }
            }
        }
        finish_spinner_ok(
            spinner,
            format!("Read {} item(s) from disk", geoparquet_items.len()),
        );
    }

    // Add GeoParquet asset if enabled
    if config.geoparquet && !geoparquet_items.is_empty() {
        collection_builder = collection_builder.asset("items-geoparquet", make_geoparquet_asset());
    }

    // Build and write collection
    let collection = collection_builder.build()?;
    let collection_json = if config.pretty {
        serde_json::to_string_pretty(&collection)?
    } else {
        serde_json::to_string(&collection)?
    };

    std::fs::write(&collection_path, &collection_json)?;

    // Write GeoParquet file if enabled
    if config.geoparquet && !geoparquet_items.is_empty() {
        let parquet_path = config.output.join("items.parquet");
        let spinner = create_spinner("Writing GeoParquet…");
        crate::stac::geoparquet::write_geoparquet(&geoparquet_items, &collection, &parquet_path)?;
        finish_spinner_ok(
            spinner,
            format!(
                "GeoParquet written: {} ({} items)",
                parquet_path.display(),
                geoparquet_items.len()
            ),
        );
    }

    // Print summary
    let mut summary = Summary::new()
        .add("Collection", collection_path.display().to_string())
        .add("Items dir", items_dir.display().to_string())
        .add(
            "Items generated",
            format!("{}", accumulator.successful_count()),
        );
    if config.geoparquet && !geoparquet_items.is_empty() {
        summary = summary.add(
            "GeoParquet",
            config.output.join("items.parquet").display().to_string(),
        );
    }
    summary.print();

    print_success("Collection generated successfully");

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
    dry_run: bool,
    geoparquet: bool,
}

fn handle_update_collection_command(config: UpdateCollectionConfig) -> Result<()> {
    // Dry-run mode: validate only
    if config.dry_run {
        use progress::{print_banner, print_error, print_success};

        print_banner();

        println!("\nRunning in dry-run mode...\n");

        let mut all_valid = true;
        let mut found = 0;

        for item_path in &config.items {
            let fname = item_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            if item_path.exists() {
                // Try to parse as STAC item
                match std::fs::read_to_string(item_path) {
                    Ok(content) => match serde_json::from_str::<crate::stac::StacItem>(&content) {
                        Ok(_) => {
                            println!("  ✓ {}", fname);
                            found += 1;
                        }
                        Err(e) => {
                            println!("  ✗ {}: Invalid STAC item - {}", fname, e);
                            all_valid = false;
                        }
                    },
                    Err(e) => {
                        println!("  ✗ {}: Cannot read - {}", fname, e);
                        all_valid = false;
                    }
                }
            } else {
                println!("  ✗ {}: File not found", fname);
                all_valid = false;
            }
        }

        println!("\n  STAC items: {}/{} valid", found, config.items.len());

        println!();

        if all_valid {
            print_success("Dry run complete: All validations passed");
            std::process::exit(0);
        } else {
            print_error("Dry run failed: Errors found");
            std::process::exit(1);
        }
    }

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
        base_url: None, // update-collection uses items_base_url for item links, not asset hrefs
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

    let pb = create_progress_bar(config.items.len() as u64, "Parsing STAC items…");
    for item_path in &config.items {
        let fname = item_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        pb.set_message(format!("Parsing: {fname}"));
        match std::fs::read_to_string(item_path) {
            Ok(content) => match serde_json::from_str::<crate::stac::StacItem>(&content) {
                Ok(item) => {
                    parsed_items.push(item);
                }
                Err(e) => {
                    if config.skip_errors {
                        errors.push((item_path.clone(), e.to_string()));
                        pb.println(format!(
                            "  {} Skipping {fname}: {e}",
                            console::style("⚠").yellow()
                        ));
                    } else {
                        pb.finish_and_clear();
                        return Err(crate::error::CityJsonStacError::JsonError(e));
                    }
                }
            },
            Err(e) => {
                if config.skip_errors {
                    errors.push((item_path.clone(), e.to_string()));
                    pb.println(format!(
                        "  {} Skipping {fname}: {e}",
                        console::style("⚠").yellow()
                    ));
                } else {
                    pb.finish_and_clear();
                    return Err(crate::error::CityJsonStacError::IoError(e));
                }
            }
        }
        pb.inc(1);
    }
    pb.finish_and_clear();

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

    // Add GeoParquet asset if enabled
    if config.geoparquet && !parsed_items.is_empty() {
        collection_builder = collection_builder.asset("items-geoparquet", make_geoparquet_asset());
    }

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

    std::fs::write(&config.output, &collection_json)?;

    // Write GeoParquet file if enabled
    if config.geoparquet && !parsed_items.is_empty() {
        let parquet_path = config
            .output
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("items.parquet");
        let spinner = create_spinner("Writing GeoParquet…");
        crate::stac::geoparquet::write_geoparquet(&parsed_items, &collection, &parquet_path)?;
        finish_spinner_ok(
            spinner,
            format!(
                "GeoParquet written: {} ({} items)",
                parquet_path.display(),
                parsed_items.len()
            ),
        );
    }

    // Print summary
    let mut summary = Summary::new()
        .add("Collection", config.output.display().to_string())
        .add("Items aggregated", format!("{}", parsed_items.len()));
    if !errors.is_empty() {
        summary = summary.add("Skipped", format!("{} item(s)", errors.len()));
    }
    if config.geoparquet && !parsed_items.is_empty() {
        let parquet_path = config
            .output
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("items.parquet");
        summary = summary.add("GeoParquet", parquet_path.display().to_string());
    }
    summary.print();

    if errors.is_empty() {
        print_success("Collection updated successfully");
    } else {
        print_warning(format!(
            "Collection updated with {} skipped item(s)",
            errors.len()
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_folder_name_basic() {
        // Valid characters should pass through
        assert_eq!(sanitize_folder_name("my-collection"), "my-collection");
        assert_eq!(sanitize_folder_name("my_collection"), "my_collection");
        assert_eq!(sanitize_folder_name("my.collection"), "my.collection");
        assert_eq!(sanitize_folder_name("collection123"), "collection123");
    }

    #[test]
    fn test_sanitize_folder_name_spaces() {
        // Spaces should be replaced with underscores
        assert_eq!(sanitize_folder_name("my collection"), "my_collection");
        assert_eq!(sanitize_folder_name("my  collection"), "my__collection");
    }

    #[test]
    fn test_sanitize_folder_name_special_chars() {
        // Special characters should be replaced with underscores
        assert_eq!(sanitize_folder_name("my@collection"), "my_collection");
        assert_eq!(sanitize_folder_name("my/collection"), "my_collection");
        assert_eq!(sanitize_folder_name("my\\collection"), "my_collection");
        assert_eq!(sanitize_folder_name("my:collection"), "my_collection");
        assert_eq!(sanitize_folder_name("my*collection"), "my_collection");
        assert_eq!(sanitize_folder_name("my?collection"), "my_collection");
        assert_eq!(sanitize_folder_name("my<collection"), "my_collection");
        assert_eq!(sanitize_folder_name("my>collection"), "my_collection");
        assert_eq!(sanitize_folder_name("my|collection"), "my_collection");
    }

    #[test]
    fn test_sanitize_folder_name_unicode() {
        // Unicode letters are alphanumeric and pass through (good for internationalization)
        assert_eq!(sanitize_folder_name("münchen"), "münchen");
        assert_eq!(sanitize_folder_name("東京"), "東京");
        // But special unicode symbols are replaced
        assert_eq!(sanitize_folder_name("hello★world"), "hello_world");
    }

    #[test]
    fn test_sanitize_folder_name_mixed() {
        // Mixed valid and invalid characters
        assert_eq!(
            sanitize_folder_name("my awesome collection!"),
            "my_awesome_collection_"
        );
        assert_eq!(
            sanitize_folder_name("collection (v1.0)"),
            "collection__v1.0_"
        );
    }

    #[test]
    fn test_fallback_folder_name() {
        assert_eq!(fallback_folder_name("path/to/config.yaml"), "config.yaml");
        assert_eq!(
            fallback_folder_name("./opendata/vienna-config.yaml"),
            "vienna-config.yaml"
        );
        assert_eq!(fallback_folder_name("config"), "config");
    }
}
