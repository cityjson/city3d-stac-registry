# Implementation Plan

## Project Timeline

**Total Estimated Duration:** 3-4 weeks
**Team Size:** 1-2 developers
**Technology:** Rust

## Phase Breakdown

### Phase 1: Foundation (Week 1)

#### 1.1 Project Setup (2 days)
- [x] Design documentation complete
- [ ] Initialize Rust project with Cargo
- [ ] Set up project structure (modules)
- [ ] Configure dependencies in Cargo.toml
- [ ] Set up CI/CD pipeline (GitHub Actions)
- [ ] Configure linting (clippy, rustfmt)

**Deliverables:**
- Compiling project skeleton
- All modules stubbed out
- CI passing

**Files to Create:**
```
Cargo.toml
src/main.rs
src/lib.rs
src/cli/mod.rs
src/reader/mod.rs
src/metadata/mod.rs
src/stac/mod.rs
src/traversal/mod.rs
src/error.rs
.github/workflows/ci.yml
```

#### 1.2 Core Data Structures (2 days)
- [ ] Implement error types (`error.rs`)
- [ ] Implement metadata structures:
  - [ ] `BBox3D`
  - [ ] `CRS`
  - [ ] `AttributeDefinition`
  - [ ] `Transform`
- [ ] Write unit tests for data structures
- [ ] Implement serialization/deserialization

**Deliverables:**
- All metadata types with tests
- Serialization working correctly

**Testing:**
```rust
#[test]
fn test_bbox_merge() {
    let bbox1 = BBox3D::new(0.0, 0.0, 0.0, 10.0, 10.0, 10.0);
    let bbox2 = BBox3D::new(5.0, 5.0, 5.0, 15.0, 15.0, 15.0);
    let merged = bbox1.merge(&bbox2);
    assert_eq!(merged, BBox3D::new(0.0, 0.0, 0.0, 15.0, 15.0, 15.0));
}
```

#### 1.3 Reader Trait & Factory (1 day)
- [ ] Define `CityModelMetadataReader` trait
- [ ] Implement reader factory function
- [ ] Add file type detection logic
- [ ] Write trait tests with mock implementations

**Deliverables:**
- Trait definition
- Factory pattern implementation
- Mock reader for testing

### Phase 2: CityJSON Reader (Week 1-2)

#### 2.1 Basic CityJSON Parsing (3 days)
- [ ] Implement `CityJSONReader` struct
- [ ] Lazy loading mechanism
- [ ] Parse core CityJSON structure:
  - [ ] Version field
  - [ ] Metadata object
  - [ ] CityObjects map
  - [ ] Vertices array
  - [ ] Transform (if present)

**Deliverables:**
- CityJSON file loading
- Basic structure parsing

**Test Files Needed:**
- Simple building (LOD2)
- File with transform
- File without metadata

#### 2.2 Metadata Extraction (3 days)
- [ ] Implement `bbox()` method
  - Parse from metadata.geographicalExtent
  - Fallback: compute from vertices
- [ ] Implement `crs()` method
  - Parse referenceSystem
  - Handle EPSG codes
- [ ] Implement `lods()` method
  - Scan geometry objects
  - Collect unique LOD values
- [ ] Implement `city_object_types()` method
- [ ] Implement `city_object_count()` method
- [ ] Implement `attributes()` method
  - Scan all CityObjects
  - Build attribute schema
- [ ] Implement `transform()` method
- [ ] Implement `metadata()` method

**Deliverables:**
- Full metadata extraction working
- Comprehensive unit tests
- Integration test with real CityJSON file

**Testing Strategy:**
```rust
#[test]
fn test_cityjson_metadata_extraction() {
    let reader = CityJSONReader::new("tests/fixtures/simple_building.json").unwrap();

    let bbox = reader.bbox().unwrap();
    assert!(bbox.xmax > bbox.xmin);

    let lods = reader.lods().unwrap();
    assert!(lods.contains(&"2".to_string()));

    let types = reader.city_object_types().unwrap();
    assert!(types.contains(&"Building".to_string()));
}
```

### Phase 3: STAC Generation (Week 2)

#### 3.1 STAC Data Models (2 days)
- [ ] Define STAC Item struct
- [ ] Define STAC Collection struct
- [ ] Define Link struct
- [ ] Define Asset struct
- [ ] Define Extent struct
- [ ] Implement serde serialization
- [ ] Test JSON output format

**Deliverables:**
- STAC models serializing to correct JSON
- Validation against STAC schema

#### 3.2 STAC Builders (3 days)
- [ ] Implement `StacItemBuilder`
  - [ ] Basic fields (id, type, geometry, bbox)
  - [ ] Properties
  - [ ] CityJSON extension integration
  - [ ] Assets
  - [ ] Links
- [ ] Implement `StacCollectionBuilder`
  - [ ] Basic fields
  - [ ] Extent (spatial & temporal)
  - [ ] Summaries
  - [ ] Aggregation methods
- [ ] Integration with metadata readers
- [ ] Tests for builders

**Deliverables:**
- Working Item and Collection builders
- Tests with real data

#### 3.3 Extension Implementation (1 day)
- [ ] Implement CityJSON extension properties
- [ ] Create extension JSON schema
- [ ] Validate against schema
- [ ] Document extension

**Deliverables:**
- Extension schema JSON file
- Validation working

### Phase 4: Additional Format Readers (Week 2-3)

#### 4.1 CityJSON Sequences Reader (3 days)
- [ ] Implement `CityJSONSeqReader`
- [ ] Streaming line-by-line parsing
- [ ] Aggregate metadata across features
- [ ] Handle large files efficiently
- [ ] Tests

**Challenge:** Streaming aggregation of metadata

**Approach:**
```rust
// Process line by line
let file = BufReader::new(File::open(path)?);
let mut all_types = HashSet::new();
let mut bbox: Option<BBox3D> = None;

for line in file.lines() {
    let feature: CityJSONFeature = serde_json::from_str(&line?)?;
    // Update aggregates
    all_types.extend(feature.get_types());
    bbox = Some(match bbox {
        Some(b) => b.merge(&feature.bbox()?),
        None => feature.bbox()?,
    });
}
```

#### 4.2 FlatCityBuf Reader (4 days)
- [ ] Research FlatCityBuf format
- [ ] Add flatbuffers dependency
- [ ] Generate FlatBuffers schema
- [ ] Implement `FlatCityBufReader`
- [ ] Parse header and metadata
- [ ] Extract required properties
- [ ] Tests with FCB files

**Reference:** https://github.com/cityjson/flatcitybuf

**Note:** This may require understanding the FlatBuffers schema first

### Phase 5: CLI Implementation (Week 3)

#### 5.1 CLI Structure (2 days)
- [ ] Set up clap with subcommands
- [ ] Implement argument parsing
- [ ] Global options (verbose, quiet)
- [ ] Help text and documentation
- [ ] Version information

**Deliverables:**
- CLI parsing working
- Help text clear and useful

#### 5.2 Item Command (2 days)
- [ ] Implement `handle_item_command()`
- [ ] File validation
- [ ] Output path generation
- [ ] Error handling
- [ ] User-friendly messages
- [ ] Tests

**Deliverables:**
- Working `item` command
- Integration tests

#### 5.3 Collection Command (3 days)
- [ ] Implement directory traversal
- [ ] File filtering by extension
- [ ] Recursive scanning
- [ ] Progress reporting
- [ ] Parallel processing (optional)
- [ ] Error handling (skip vs. fail)
- [ ] Aggregation logic
- [ ] Output directory structure
- [ ] Tests

**Deliverables:**
- Working `collection` command
- Progress bar implementation
- Error handling

### Phase 6: Directory Traversal (Week 3)

#### 6.1 Traversal Logic (2 days)
- [ ] Implement `find_files()` function
- [ ] Use walkdir for recursion
- [ ] Filter by extensions
- [ ] Respect max-depth
- [ ] Handle symlinks
- [ ] Error handling
- [ ] Tests

#### 6.2 Aggregation (2 days)
- [ ] Collect metadata from multiple readers
- [ ] Merge bounding boxes
- [ ] Aggregate LODs, types, encodings
- [ ] Compute statistics (min, max, total)
- [ ] Temporal extent handling
- [ ] Tests

### Phase 7: Testing & Polish (Week 3-4)

#### 7.1 Integration Tests (3 days)
- [ ] End-to-end test: file → item
- [ ] End-to-end test: directory → collection
- [ ] Test with all formats
- [ ] Test error cases
- [ ] Test with large datasets
- [ ] Performance benchmarks

**Test Fixtures Needed:**
- Small CityJSON file (~1MB)
- Large CityJSON file (~100MB)
- CityJSONSeq file
- FlatCityBuf file
- Directory with mixed formats
- Invalid files

#### 7.2 Documentation (2 days)
- [ ] API documentation (rustdoc)
- [ ] User guide
- [ ] Example outputs
- [ ] CLI usage examples
- [ ] Troubleshooting guide
- [ ] Update README

#### 7.3 Examples & Samples (1 day)
- [ ] Create example STAC outputs
- [ ] Create sample data
- [ ] Usage tutorials
- [ ] Common workflows

#### 7.4 Validation Command (Optional) (2 days)
- [ ] Implement STAC validation
- [ ] JSON schema validation
- [ ] Extension validation
- [ ] Tests

### Phase 8: Release Preparation (Week 4)

#### 8.1 Packaging (1 day)
- [ ] Finalize Cargo.toml metadata
- [ ] Create binary releases
- [ ] Test installation from crates.io
- [ ] Create Docker image (optional)

#### 8.2 CI/CD (1 day)
- [ ] Set up release workflow
- [ ] Automated testing
- [ ] Code coverage reporting
- [ ] Benchmark tracking

#### 8.3 Release (1 day)
- [ ] Version tagging
- [ ] Changelog
- [ ] GitHub release
- [ ] Publish to crates.io
- [ ] Announce

## Development Workflow

### Daily Development Cycle

```
1. Pick task from current phase
2. Write tests first (TDD approach)
3. Implement functionality
4. Run tests: cargo test
5. Run linter: cargo clippy
6. Format: cargo fmt
7. Commit with clear message
8. Push to feature branch
```

### Git Branch Strategy

```
main (protected)
  ├── develop
  │    ├── feature/cli-implementation
  │    ├── feature/cityjson-reader
  │    ├── feature/stac-builder
  │    └── feature/fcb-reader
```

### Definition of Done

For each task:
- [ ] Code written and compiles
- [ ] Unit tests pass
- [ ] Integration tests pass (if applicable)
- [ ] Documentation updated
- [ ] Code reviewed (if team)
- [ ] CI pipeline green

## Risk Management

### High-Risk Areas

1. **FlatCityBuf Format Complexity**
   - **Risk:** Unknown format complexity
   - **Mitigation:** Research first, allocate extra time
   - **Fallback:** Implement basic support, defer advanced features

2. **Performance with Large Files**
   - **Risk:** Memory issues with large CityJSON files
   - **Mitigation:** Implement streaming early, test with large files
   - **Fallback:** Document limitations, recommend splitting files

3. **CityJSON Spec Variations**
   - **Risk:** Real-world files may not conform to spec
   - **Mitigation:** Test with diverse real-world files
   - **Fallback:** Graceful error handling, best-effort parsing

### Medium-Risk Areas

1. **STAC Extension Approval**
   - **Risk:** Custom extension may not be accepted
   - **Mitigation:** Follow STAC extension guidelines
   - **Fallback:** Use as unofficial extension initially

2. **CRS Handling Complexity**
   - **Risk:** Complex CRS transformations needed
   - **Mitigation:** Start with EPSG codes only
   - **Fallback:** Document CRS limitations

## Testing Strategy

### Test Pyramid

```
       /\
      /E2E\         10 tests - Full workflows
     /------\
    /  INT   \      30 tests - Module integration
   /----------\
  /    UNIT    \    100+ tests - Individual functions
 /--------------\
```

### Test Coverage Goals

- **Unit Tests:** 80%+ coverage
- **Integration Tests:** Major workflows covered
- **E2E Tests:** Happy path + key error cases

### Performance Benchmarks

Track performance for:
- Single file processing (< 1s for 10MB file)
- Directory scanning (100 files < 30s)
- Memory usage (< 500MB for typical workload)

## Success Criteria

### Minimum Viable Product (MVP)

- [x] Design documentation complete
- [ ] CityJSON format support
- [ ] STAC Item generation
- [ ] STAC Collection generation
- [ ] CLI with `item` and `collection` commands
- [ ] Basic error handling
- [ ] README with examples
- [ ] Published to crates.io

### Full Release

All MVP items plus:
- [ ] CityJSONSeq support
- [ ] FlatCityBuf support
- [ ] Comprehensive tests
- [ ] User documentation
- [ ] CI/CD pipeline
- [ ] Binary releases

### Future Enhancements

- [ ] CityParquet support
- [ ] Parallel processing
- [ ] Validation command
- [ ] STAC API server mode
- [ ] Cloud storage support

## Resources

### Reference Implementations

- CityJSON spec: https://www.cityjson.org/specs/
- STAC spec: https://stacspec.org/
- FlatCityBuf Rust: https://github.com/cityjson/flatcitybuf/tree/main/src/rust

### Tools

- Rust playground for prototyping
- CityJSON validator
- STAC validator
- JSON Schema validator

### Sample Data Sources

- 3D BAG (Netherlands): https://3dbag.nl
- CityJSON examples: https://github.com/cityjson/cityjson-examples
- Create own test fixtures

## Monitoring Progress

### Weekly Check-ins

- Review completed tasks
- Update timeline if needed
- Identify blockers
- Adjust priorities

### Metrics to Track

- Lines of code
- Test coverage
- Open issues
- Documentation completeness
- Performance benchmarks
