# Contributing

This repository accepts contributions for public 3D city model registry entries.

Canonical repository URL:
`https://github.com/cityjson/city3d-stac-registry`

## What belongs here

- Collection configs in `collections/*.yaml`
- Catalog membership changes in `catalog/catalog-config.yaml`
- Supporting URL manifests in `manifests/` when a dataset is too large or too fragmented to
  maintain inline
- Contributor-facing documentation

The generator implementation does not belong here. Use the pinned submodule in
`tools/cityjson-stac/`.

## Contribution checklist

1. Initialize the submodule:

```bash
git submodule update --init --recursive
```

Optional: install the CLI natively instead of running it through the submodule checkout:

```bash
cargo install --git ssh://git@github.com/HideBa/city3d-stac-tool.git --bin city3dstac
```

2. Validate the changed collection config:

```bash
cargo run --manifest-path tools/cityjson-stac/Cargo.toml -- \
  collection --config collections/<dataset>.yaml --dry-run
```

3. If you changed the catalog membership, validate the root catalog too:

```bash
cargo run --manifest-path tools/cityjson-stac/Cargo.toml -- \
  catalog --config catalog/catalog-config.yaml --dry-run
```

## File conventions

- Use lowercase, hyphen-separated filenames.
- Keep `id` stable once published.
- Prefer direct input URLs in YAML when the list is manageable.
- Use `manifests/*` only for large generated URL lists.
- Do not commit generated STAC JSON from local runs.
