# Plan: STAC Extension Compliance + `stac` Crate Migration

## Progress

| Phase | Status | Commit | Notes |
|-------|--------|--------|-------|
| Phase 0: Add Dependencies | Pending | | |
| **Phase 1: Fix Compliance** | **Done** | `b40f98c` | 8 issues fixed, all 402 tests pass |
| Phase 2: Migrate to `stac` crate | Pending | | |
| Phase 3: Migrate GeoParquet | Pending | | |
| Phase 4: E2E Validation Tests | Pending | | |
| Phase 5: Update Config System | Pending | | |
| Phase 6: Final Cleanup | Pending | | |

### Phase 1 Details (completed)

- [x] Step 1.1: `proj:epsg` -> `proj:code` (string format per Projection Ext v2.0.0)
- [x] Step 1.2: `datetime` defaults to null; extracts `referenceDate` from CityJSON metadata
- [x] Step 1.3: Boolean summaries as arrays (`[true]`, `[true, false]`)
- [x] Step 1.4: `file:size` moved to asset (not item properties)
- [x] Step 1.5: `file:checksum` simplified to plain multihash string
- [x] Step 1.6: `item_assets` auto-generated for collections + Item Assets Extension URL
- [x] Step 1.7: `city3d:encoding` removed from accumulator, geoparquet, tests
- [x] Step 1.8: `city-model` rel type added to items

---

## Context

The cityjson-stac project generates STAC metadata for 3D city model datasets. It currently uses custom STAC structs (`StacItem`, `StacCollection`, etc.) in `src/stac/models.rs`. This plan addresses two goals:

1. **Fix compliance issues** with the city3d STAC extension specification
2. **Migrate** from custom STAC types to the upstream `stac` Rust crate for correctness, validation, and GeoParquet support

### Strategy
**Fix compliance first, then migrate.** Phase 1 fixes all output correctness issues on the existing custom types (lower risk, each fix testable independently). Phase 2+ migrates to the `stac` crate. This allows incremental PRs and reduces risk.

---

## Issues Found (Current vs Spec)

### Critical (Wrong Output)

| # | Issue | Current | Required | Files |
|---|-------|---------|----------|-------|
| 1 | **Projection field** | `proj:epsg` (integer: `7415`) | `proj:code` (string: `"EPSG:7415"`) per Projection Ext v2.0.0 | item.rs:191-198, collection.rs:239-251, ~40 refs in tests |
| 2 | **datetime** | `Utc::now()` (metadata generation time) | Data's temporal extent, or `null` with date range | item.rs:31-34 |
| 3 | **Collection boolean summaries** | Single value: `true` | Array of values: `[true]` or `[true, false]` | collection.rs:218-237 (3 aggregate methods) |
| 4 | **`file:size` location** | Item properties | Asset `additional_fields` | item.rs:205-211 |
| 5 | **`file:checksum` format** | Struct `{value, namespace}` | Simple multihash string | models.rs:169-177 |

### Missing Features

| # | Issue | Description | Spec Reference |
|---|-------|-------------|----------------|
| 6 | **`item_assets`** | Collections should define expected asset templates | Extension collection example |
| 7 | **Item Assets Extension URL** | Missing `https://stac-extensions.github.io/item-assets/v1.1.0/schema.json` | Extension collection example |
| 8 | **`city-model` / `preview` rel types** | Custom relation types not offered | Extension README "Relation types" |
| 9 | **File Extension** in `stac_extensions` | File Extension URL not always included when `file:size` is on assets | STAC extensions spec |
| 10 | **Statistics Extension** | Only `city_objects` stats; should also include `file:size` stats in collection summaries | Extension README |

### Stale References

| # | Issue | Location |
|---|-------|----------|
| 11 | `city3d:encoding` still referenced | accumulator.rs:101, geoparquet.rs:85, ~15 test refs |
| 12 | `proj:epsg` in collection aggregation | collection.rs:593-603, integration_tests.rs |

### Config/CLI Gaps

| # | Field | Can be auto-detected? | Proposed source |
|---|-------|-----------------------|-----------------|
| 13 | `datetime` | **Yes** for CityJSON (`referenceDate` in metadata) | 1st: `reader.metadata()["referenceDate"]`, 2nd: config `extent.temporal`, 3rd: null |
| 14 | `file:checksum` | Yes (compute at read time) | Auto-compute SHA-256 for local files |
| 15 | `city-model` link | Partially (asset href) | Auto-generate from data asset href |
| 16 | `preview` link | No | Optional config field `preview_url` |

---

## Implementation Phases

### Phase 0: Add Dependencies

**File: `Cargo.toml`**

```toml
stac = { version = "0.16", features = ["geoparquet"] }
stac-validate = "0.4"
indexmap = "2"
```

Remove direct `arrow`, `parquet`, `geozero` dependencies once GeoParquet migration is complete.

### Phase 1: Fix Compliance Issues (No Crate Migration Yet)

Fix output correctness first, keeping custom types. This makes each fix testable independently.

**Step 1.1: `proj:epsg` → `proj:code`**
- `src/metadata/crs.rs`: Add `to_stac_proj_code() -> Option<String>` returning `"EPSG:{code}"`
- `src/stac/item.rs`: Replace `"proj:epsg"` with `"proj:code"`, change value from integer to string
- `src/stac/collection.rs`: Fix all 3 aggregation methods to collect `proj:code` strings
- `src/stac/item.rs` `build()`: Check for `"proj:code"` not `"proj:epsg"`
- `src/stac/collection.rs` `build()`: Check for `"proj:code"` not `"proj:epsg"`
- Update all test assertions (~40 refs)

**Step 1.2: Fix `datetime`**

The STAC `datetime` should represent the data's temporal extent, not metadata generation time.

Priority for datetime resolution:
1. **CityJSON `referenceDate`**: Extract from `reader.metadata()["referenceDate"]` (available in CityJSON, CityJSONSeq, FlatCityBuf via `metadata()` trait method)
2. **Config override**: Use `extent.temporal.start` from config YAML
3. **Fallback**: `null` with `start_datetime`/`end_datetime` range if both are specified in config

Implementation:
- `src/reader/mod.rs`: Add `reference_date()` default method to `CityModelMetadataReader` trait
- `src/stac/item.rs` `new()`: Default `datetime` to `null` (JSON null), not `Utc::now()`
- `src/stac/item.rs` `cityjson_metadata()`: Extract `referenceDate` → set as `datetime`
- `src/stac/item.rs`: Add `start_datetime()`, `end_datetime()` builder methods
- `src/cli/mod.rs`: When config has `extent.temporal.start/end`, propagate to items as fallback
- Logic: If `referenceDate` found → use as `datetime`. Else if config has only `start` → use as `datetime`. If config has both `start` and `end` → set `datetime=null`, `start_datetime=start`, `end_datetime=end`. Otherwise → `datetime=null`.

**Step 1.3: Fix collection boolean summaries**
- `src/stac/collection.rs` `aggregate_cityjson_metadata()`: Collect unique boolean values as `Vec<bool>` → `[true]`, `[true, false]`, `[false]`
- `src/stac/collection.rs` `aggregate_from_metadata()`: Same fix
- `src/stac/collection.rs` `aggregate_from_items()`: Same fix
- All three methods need the same pattern for `semantic_surfaces`, `textures`, `materials`

**Step 1.4: Move `file:size` to assets**
- `src/stac/item.rs`: Remove `file_size()` method that sets item property
- `src/stac/item.rs` `data_asset()`: Accept optional file_size parameter, set on asset
- `src/stac/models.rs` `Asset`: Already has `file_size` field → populate it in `from_file*()`

**Step 1.5: Fix `file:checksum` to simple string**
- `src/stac/models.rs`: Remove `Checksum` struct, change `file_checksum` to `Option<String>`
- Use multihash format: `"1220{sha256hex}"` or just pass through as string

**Step 1.6: Add `item_assets` to collections**
- `src/stac/models.rs` `StacCollection`: Add `item_assets: Option<HashMap<String, Value>>`
- `src/stac/collection.rs`: Auto-populate `item_assets` with a default data asset template
- `build()`: If `item_assets` present, add Item Assets Extension URL to `stac_extensions`

**Step 1.7: Clean up `city3d:encoding` references**
- `src/stac/accumulator.rs`: Remove `city3d_encoding` field from `ItemMetadata`
- `src/stac/geoparquet.rs`: Remove `"city3d:encoding"` from `city3d_type()`
- Update all test references

**Step 1.8: Add `city-model` rel type**
- `src/stac/item.rs` `from_file*()`: Add a `city-model` link pointing to the data asset href
- This is optional per the spec but good practice

### Phase 2: Migrate to `stac` Crate Types

Replace custom STAC types with `stac::Item`, `stac::Collection`, etc.

**Step 2.1: Create `src/stac/city3d_types.rs`**
- Move `CityObjectsCount` enum here (not a standard STAC type)

**Step 2.2: Rewrite `src/stac/mod.rs`**
- Re-export `stac::{Item, Collection, Catalog, Asset, Link, Provider, Extent}` as primary types
- Keep type aliases for backwards compatibility if needed: `pub type StacItem = stac::Item;`

**Step 2.3: Adapt `StacItemBuilder` to construct `stac::Item`**
- Internal storage: `stac::Item` instead of field-by-field
- Properties: Use `item.properties.additional_fields` for extension props
- `item.properties.datetime` for datetime (native field on `stac::Properties`)
- `stac::Item` has `bbox: Option<Bbox>` (enum: `TwoDimensional([f64;4])` or `ThreeDimensional([f64;6])`)
- `stac::Asset.roles` is `Vec<String>` (not `Option<Vec<String>>`)
- `stac::Asset.r#type` replaces `media_type`
- Extension properties on assets: `asset.additional_fields.insert("file:size", ...)`

**Step 2.4: Adapt `StacCollectionBuilder` to construct `stac::Collection`**
- `stac::Collection.description` is `String` (required, not Optional)
- `stac::Collection.summaries` is `Option<Map<String, Value>>`
- `stac::Collection.item_assets` is `IndexMap<String, ItemAsset>` (native field!)
- Extension URLs go in `collection.extensions`

**Step 2.5: Adapt `StacCatalogBuilder`**
- `stac::Catalog.description` is `String` (required)

**Step 2.6: Delete `src/stac/models.rs`**

**Step 2.7: Update `src/stac/accumulator.rs`**
- `ItemMetadata::from_item()`: Access `item.properties.additional_fields` instead of `item.properties`
- `item.bbox` is `Option<Bbox>` → convert to `Vec<f64>` via match on Bbox variant
- Remove `city3d_encoding` field

**Step 2.8: Update `src/cli/mod.rs`**
- Update all type references: `StacItem` → `stac::Item`, etc.
- JSON serialization of `stac::Item` uses `serde_json::to_value(&item)` (implements Serialize)
- Access patterns change: `item.id`, `item.properties.datetime`, `item.properties.additional_fields["city3d:version"]`

### Phase 3: Migrate GeoParquet to `stac::geoparquet`

Replace the custom 1000-line GeoParquet writer with the `stac` crate's module.

**Step 3.1: Replace `write_geoparquet()` in `src/stac/geoparquet.rs`**
- Use `stac::geoparquet::ItemCollection` or the `IntoGeoparquet` trait
- The `stac` crate's geoparquet module handles schema generation, WKB encoding, metadata embedding
- API: `stac::item_collection::ItemCollection` + `.into_writer(path)` or similar

**Step 3.2: Handle collection metadata**
- The stac-geoparquet spec requires collection metadata in Parquet key-value metadata
- Verify the `stac` crate embeds this correctly

**Step 3.3: Remove custom Arrow/Parquet code**
- Delete `build_arrow_schema()`, `build_record_batch()`, `json_values_to_array()`, etc.
- Remove direct `arrow` and `parquet` dependencies from `Cargo.toml`

### Phase 4: Add E2E Validation Tests

**Step 4.1: Add validation test infrastructure**
- Use `stac-validate` crate (async) with the extension JSON schema at `stac-cityjson-extension/json-schema/schema.json`
- Create test that generates a STAC Item from test data → validates against both STAC core schema and city3d extension schema

**Step 4.2: Validation test cases**
- Item: valid city3d extension fields
- Item: `proj:code` is string format
- Item: `file:size` is on asset, not properties
- Item: `datetime` is null with start/end range, or a valid RFC3339 string
- Collection: boolean summaries are arrays
- Collection: `item_assets` present
- Collection: all required `stac_extensions` URLs present
- Collection: `proj:code` summaries are string arrays

**Step 4.3: Verify with extension JSON schema**
```rust
use stac_validate::Validate;
let item: stac::Item = builder.build()?;
item.validate().await?;
```

### Phase 5: Update Config System

**Step 5.1: datetime propagation from config**
- Config already has `extent.temporal.start/end` — use as fallback when `referenceDate` is absent
- No new config fields needed for datetime (existing `TemporalExtentConfig` suffices)
- `src/cli/mod.rs`: Pass temporal config to item builder as fallback

**Step 5.2: Add `preview_url` to config (optional)**
```yaml
preview_url: "https://viewer.example.com/dataset"
```
- Generates a `preview` rel link on items
- `src/config/mod.rs`: Add `preview_url: Option<String>` to `CollectionConfigFile`

**Step 5.3: No changes needed to existing YAML configs**
- The 53 config files in `opendata/` already have proper `extent.temporal` fields
- The `proj:code` change is internal (CRS struct handles the format)
- datetime: most configs already specify `extent.temporal.start` which will be used as fallback

### Phase 6: Final Cleanup

**Step 6.1: Update all tests** (~40 `proj:epsg` refs, ~15 `city3d:encoding` refs)
**Step 6.2: Run `cargo fmt && cargo clippy -- -D warnings && cargo test`**
**Step 6.3: Verify JSON output matches extension examples**

---

## Verification Plan

1. **Unit tests**: Run `cargo test --lib` after each phase
2. **Schema validation**: Validate generated JSON against `stac-cityjson-extension/json-schema/schema.json`
3. **Integration tests**: Run existing integration tests (update assertions)
4. **Manual check**: Generate STAC Item/Collection from test CityJSON data, compare with extension examples
5. **GeoParquet**: Write and read back a parquet file, verify metadata
6. **CLI dry-run**: `cargo run -- collection --config opendata/berlin-config.yaml --dry-run`

---

## Files to Modify

| File | Changes |
|------|---------|
| `Cargo.toml` | Add `stac`, `stac-validate`, `indexmap`; remove `arrow`, `parquet`, `geozero` |
| `src/metadata/crs.rs` | Add `to_stac_proj_code()` |
| `src/stac/mod.rs` | Re-export `stac` crate types |
| `src/stac/models.rs` | Delete (replaced by `stac` crate) |
| `src/stac/city3d_types.rs` | New: `CityObjectsCount` enum |
| `src/stac/item.rs` | Adapt builder to `stac::Item`, fix proj:code, datetime, file:size |
| `src/stac/collection.rs` | Adapt builder to `stac::Collection`, fix summaries, add item_assets |
| `src/stac/catalog.rs` | Adapt to `stac::Catalog` |
| `src/stac/geoparquet.rs` | Replace with `stac::geoparquet` module |
| `src/stac/accumulator.rs` | Update to use `stac::Item`, remove `city3d_encoding` |
| `src/cli/mod.rs` | Update type references, datetime propagation |
| `src/config/mod.rs` | Add optional `preview_url` field |
| `tests/stac_tests.rs` | Update all assertions |
| `tests/integration_tests.rs` | Update proj:epsg → proj:code assertions |
| `tests/schema_validation_tests.rs` | Update validation assertions |
| `tests/cjseq_e2e_tests.rs` | Update proj assertions |
| `tests/config_tests.rs` | Remove city3d:encoding refs |
| `tests/cli_tests.rs` | Update proj assertions |
