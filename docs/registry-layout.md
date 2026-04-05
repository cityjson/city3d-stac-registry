# Registry Layout

## Directories

- `catalog/`: root catalog configuration
- `collections/`: collection definitions contributed by users
- `manifests/`: generated URL lists referenced by collection configs
- `city3d-stac-tool/`: pinned generator implementation

## Relative path rules

- Paths in `catalog/catalog-config.yaml` are resolved relative to `catalog/`
- Paths in `collections/*.yaml` are resolved relative to `collections/`
- Manifest-backed collections should use `../manifests/<file>.txt`
