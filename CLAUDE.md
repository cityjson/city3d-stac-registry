# City3D STAC Registry Guidelines

This repository is the public registry of 3D city model dataset definitions.

Canonical repository URL:
`https://github.com/cityjson/city3d-stac-registry`

## Repo purpose

- Store dataset collection configs in `collections/`
- Store the root catalog definition in `catalog/`
- Store generated input URL manifests in `manifests/` when needed
- Publish generated STAC by invoking the CLI from the `tools/cityjson-stac` submodule

## What does not belong here

- Rust CLI implementation
- Reader, parser, or STAC builder source code
- Tool-specific test fixtures and release automation
- Generated STAC output committed to git

Those belong in the separate tool repository vendored at `tools/cityjson-stac`.

## Editing guidance

- Treat this repo as metadata-first, not code-first.
- Prefer small, reviewable edits to collection YAML and catalog membership.
- Keep dataset IDs and filenames stable once published.
- If a dataset needs a very large list of URLs, commit a manifest under `manifests/` and reference it from the collection config with a relative path.
- When changing publication behavior, update the registry docs and GitHub workflows in this repo; update generator behavior in the tool repo instead.

## Validation workflow

- Validate a single collection with:
  `cargo run --manifest-path tools/cityjson-stac/Cargo.toml -- collection --config collections/<dataset>.yaml --dry-run`
- Validate the catalog with:
  `cargo run --manifest-path tools/cityjson-stac/Cargo.toml -- catalog --config catalog/catalog-config.yaml --dry-run`
- Native binary install is also supported:
  `cargo install --git ssh://git@github.com/HideBa/city3d-stac-tool.git --bin city3dstac`
