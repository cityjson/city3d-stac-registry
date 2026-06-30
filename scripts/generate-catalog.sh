#!/usr/bin/env bash
# Generate the full TU Delft 3D Open Cities STAC catalog.
#
# Per-collection parallelism (e.g. PLATEAU's concurrency:4 throttle for the
# rate-sensitive CMS) lives in each collections/*.yaml as `concurrency:`.
# The tool reads that field; this script only iterates — do NOT add a
# script-level --concurrency override, it would mask each collection's tuning.
#
# Usage:
#   scripts/generate-catalog.sh                 # all 54 collections + catalog
#   scripts/generate-catalog.sh japan-plateau   # subset by config stem
#   scripts/generate-catalog.sh --catalog-only  # skip the slow item step,
#                                               # just rebuild catalog.json

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TOOL="$ROOT/city3d-stac-tool/target/release/city3dstac"
CATALOG_CONFIG="$ROOT/catalog/catalog-config.yaml"
STAC_OUT="${STAC_OUT:-/data2/hideba/stac}"

# Large CityGML zips (PLATEAU goes up to ~17 GB each) will fill / if TMPDIR
# falls back there. Force temp onto the big /data2 volume.
export TMPDIR="${TMPDIR:-/data2/hideba/tmp}"
export RUST_LOG="${RUST_LOG:-info}"
mkdir -p "$TMPDIR" "$STAC_OUT"

[ -x "$TOOL" ] || cargo build --release \
  --manifest-path "$ROOT/city3d-stac-tool/Cargo.toml"

catalog_only=0
declare -a targets=()
for arg in "$@"; do
  case "$arg" in
    --catalog-only) catalog_only=1 ;;
    -h|--help)
      sed -n '2,12p' "$0" | sed 's|^# \{0,1\}||'; exit 0 ;;
    -*)
      echo "unknown flag: $arg" >&2; exit 2 ;;
    *)
      targets+=("$arg") ;;
  esac
done

if (( catalog_only == 0 )); then
  # Default: every collection referenced in catalog-config.yaml
  if (( ${#targets[@]} == 0 )); then
    mapfile -t targets < <(python3 -c "
import yaml, os, sys
for p in (yaml.safe_load(open('$CATALOG_CONFIG')).get('collections') or []):
    stem = os.path.basename(p)
    for suffix in ('-config.yaml', '-config.yml', '.yaml', '.yml'):
        if stem.endswith(suffix):
            stem = stem[:-len(suffix)]
            break
    print(stem)
")
  fi

  for name in "${targets[@]}"; do
    cfg="$ROOT/collections/${name}-config.yaml"
    [ -f "$cfg" ] || cfg="$ROOT/collections/${name}.yaml"
    if [ ! -f "$cfg" ]; then
      echo "skip: no config found for '$name'" >&2
      continue
    fi
    id=$(python3 -c "import yaml; print(yaml.safe_load(open('$cfg'))['id'])")
    echo "═══ $name → $STAC_OUT/$id ═══"
    "$TOOL" collection \
      --config "$cfg" \
      --output "$STAC_OUT/$id" \
      --skip-errors --overwrite --pretty --geoparquet
  done
fi

# Rebuild the root catalog from whatever currently lives under $STAC_OUT.
# Relies on the c77f828 update-catalog --config fix that resolves each
# referenced collection-config YAML to <output>/<id>/collection.json.
echo "═══ catalog → $STAC_OUT/catalog.json ═══"
"$TOOL" update-catalog \
  --config "$CATALOG_CONFIG" \
  --output "$STAC_OUT" \
  --pretty
