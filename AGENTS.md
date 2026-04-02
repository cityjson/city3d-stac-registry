# City3D STAC Registry Agent Guidelines

This repository is the public registry of dataset definitions for 3D city model STAC publishing.

Canonical repository URL:
`https://github.com/cityjson/city3d-stac-registry`

## Repository overview

- `catalog/` contains the root catalog configuration
- `collections/` contains one collection config per dataset
- `manifests/` contains generated URL lists for large datasets
- `tools/cityjson-stac/` is the vendored CLI tool repository

## Working rules

- Treat this repo as metadata and publication configuration, not generator source code
- Prefer editing YAML configs, catalog membership, docs, and CI workflows
- Do not add Rust source code, parser logic, or release tooling here
- Do not commit generated STAC output JSON into this repo

## Validation

- Validate a collection:
  `cargo run --manifest-path tools/cityjson-stac/Cargo.toml -- collection --config collections/<dataset>.yaml --dry-run`
- Validate the catalog:
  `cargo run --manifest-path tools/cityjson-stac/Cargo.toml -- catalog --config catalog/catalog-config.yaml --dry-run`
- Optional native install:
  `cargo install --git ssh://git@github.com/HideBa/city3d-stac-tool.git --bin city3dstac`

## Scope boundary

- Registry policy and public dataset entries belong here
- CLI implementation, tests, and release engineering belong in `tools/cityjson-stac`
