# Publishing

This registry publishes generated STAC from source configs rather than storing generated JSON in
git.

## CI flow

- `Validate Registry` runs on pull requests and on pushes to `main`.
- `Publish Catalog` runs on `main` and on manual dispatch.
- The generator is executed from the pinned `tools/cityjson-stac` submodule.

## Output contract

- Source of truth: `catalog/catalog-config.yaml`, `collections/*.yaml`, and `manifests/*`
- Generated output path in CI: `build/site`
- Published content: STAC root catalog plus generated child collections and items

## Local publish smoke test

```bash
git submodule update --init --recursive

cargo run --release --manifest-path tools/cityjson-stac/Cargo.toml -- \
  catalog --config catalog/catalog-config.yaml -o build/site
```
