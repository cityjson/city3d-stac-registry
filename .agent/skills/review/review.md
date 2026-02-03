---
name: code-review
description: Perform code reviews for Rust and CityJSON projects. Use when reviewing pull requests, examining code changes, or providing feedback on code quality. Covers memory safety, performance, CityJSON spec compliance, and spatial data handling.
---

# Rust & CityJSON Code Review

Follow these guidelines when reviewing code for Rust projects working with CityJSON and 3D city models.

## Review Checklist

### Identifying Problems

Look for these issues in code changes:

- **Memory safety**: Unnecessary `unwrap()`, missing error handling, unsafe blocks without justification
- **Performance**: Excessive cloning, inefficient iterators, unnecessary allocations in hot paths
- **Concurrency**: Data races, deadlock potential, improper `Send`/`Sync` bounds
- **CityJSON compliance**: Spec violations, invalid geometry types, incorrect CRS handling
- **I/O efficiency**: Blocking operations in async contexts, unbuffered reads/writes for large files
- **Spatial indexing**: Inefficient spatial queries, missing index utilization for range requests

### CityJSON-Specific Checks

- Validate geometry types match CityObject types per spec
- Check LOD (Level of Detail) consistency
- Verify vertex indices are within bounds of the vertices array
- Ensure semantic surfaces reference valid geometry boundaries
- Confirm transform (scale/translate) is applied correctly
- Check metadata completeness (CRS, extent, version)

### Design Assessment

- Does the change align with zero-copy and streaming principles?
- Are abstractions appropriate for cloud-optimized access patterns?
- Do component interactions support HTTP range request workflows?
- Is the code structured for efficient partial file access?

### Test Coverage

Every PR should have appropriate test coverage:

- Unit tests for geometry operations and transformations
- Property-based tests for parsing/serialization roundtrips
- Integration tests with real CityJSON files (various LODs and city object types)
- Benchmark tests for performance-critical operations

Verify tests cover edge cases: empty geometries, large coordinate values, deeply nested hierarchies.

### Long-Term Impact

Flag for careful review when changes involve:

- CityJSON schema version updates
- Spatial index structure modifications
- Binary format layout changes (FlatBuffers schemas, etc.)
- Coordinate transformation pipelines
- HTTP range request handling logic

## Feedback Guidelines

### Tone

- Be polite and constructive
- Provide actionable suggestions with code examples
- Phrase as questions when uncertain: "Have you considered...?"

### Approval

- Approve when only minor issues remain
- Don't block PRs for stylistic preferences covered by `rustfmt`
- Remember: the goal is correctness and maintainability

## Common Patterns to Flag

### Error Handling

```rust
// Bad: Panics on invalid input
let vertex = vertices[index].unwrap();

// Good: Propagate errors with context
let vertex = vertices
    .get(index)
    .ok_or_else(|| Error::InvalidVertexIndex { index, max: vertices.len() })?;
```

### Memory Efficiency

```rust
// Bad: Unnecessary allocation
let coords: Vec<f64> = vertices.iter().map(|v| v.x).collect();
let sum: f64 = coords.iter().sum();

// Good: Lazy iteration
let sum: f64 = vertices.iter().map(|v| v.x).sum();
```

### CityJSON Geometry Validation

```rust
// Bad: No bounds checking
fn get_vertex(&self, index: usize) -> [f64; 3] {
    self.vertices[index]  // May panic
}

// Good: Validate vertex indices
fn get_vertex(&self, index: usize) -> Result<[f64; 3]> {
    self.vertices
        .get(index)
        .copied()
        .ok_or(CityJsonError::VertexIndexOutOfBounds {
            index,
            vertex_count: self.vertices.len(),
        })
}
```

### Transform Application

```rust
// Bad: Incorrect transform order
fn transform_vertex(v: [i64; 3], scale: [f64; 3], translate: [f64; 3]) -> [f64; 3] {
    [
        v[0] as f64 + translate[0] * scale[0],  // Wrong order
        v[1] as f64 + translate[1] * scale[1],
        v[2] as f64 + translate[2] * scale[2],
    ]
}

// Good: Scale then translate per CityJSON spec
fn transform_vertex(v: [i64; 3], scale: [f64; 3], translate: [f64; 3]) -> [f64; 3] {
    [
        v[0] as f64 * scale[0] + translate[0],
        v[1] as f64 * scale[1] + translate[1],
        v[2] as f64 * scale[2] + translate[2],
    ]
}
```

### Async I/O

```rust
// Bad: Blocking read in async context
async fn load_cityjson(path: &Path) -> Result<CityJson> {
    let data = std::fs::read(path)?;  // Blocks the runtime
    serde_json::from_slice(&data)
}

// Good: Use async file operations
async fn load_cityjson(path: &Path) -> Result<CityJson> {
    let data = tokio::fs::read(path).await?;
    // Parse in blocking task if CPU-intensive
    tokio::task::spawn_blocking(move || serde_json::from_slice(&data)).await?
}
```

### HTTP Range Requests

```rust
// Bad: Load entire file for partial access
async fn get_city_object(url: &str, id: &str) -> Result<CityObject> {
    let full_file = reqwest::get(url).await?.bytes().await?;
    parse_and_find(&full_file, id)
}

// Good: Use range requests with spatial index
async fn get_city_object(url: &str, id: &str, index: &SpatialIndex) -> Result<CityObject> {
    let range = index.get_byte_range(id)?;
    let response = client
        .get(url)
        .header("Range", format!("bytes={}-{}", range.start, range.end))
        .send()
        .await?;
    parse_city_object(&response.bytes().await?)
}
```

### Serde Deserialization

```rust
// Bad: Rigid structure that breaks on unknown fields
#[derive(Deserialize)]
struct CityObject {
    r#type: String,
    attributes: HashMap<String, Value>,
    geometry: Vec<Geometry>,
}

// Good: Flexible with extension support
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]  // Or explicitly handle extensions
struct CityObject {
    r#type: CityObjectType,
    #[serde(default)]
    attributes: Option<HashMap<String, Value>>,
    #[serde(default)]
    geometry: Vec<Geometry>,
    #[serde(flatten)]
    extensions: HashMap<String, Value>,
}
```

## CityJSON Spec Compliance

### Required Validations

1. **Version**: Must be "2.0" for CityJSON 2.0 files
2. **CRS**: Should use EPSG codes; validate against known CRS definitions
3. **Geometry types**: Must match allowed types for each CityObject type
4. **Vertex indices**: All indices must be valid (0 ≤ index < vertices.len())
5. **Semantic surfaces**: Must reference valid boundary indices

### CityObject Type Constraints

| CityObject Type | Allowed Geometry Types                |
| --------------- | ------------------------------------- |
| Building        | Solid, MultiSurface, CompositeSurface |
| BuildingPart    | Solid, MultiSurface, CompositeSurface |
| Road            | MultiSurface, CompositeSurface        |
| LandUse         | MultiSurface, CompositeSurface        |
| WaterBody       | Solid, MultiSurface, CompositeSurface |
| PlantCover      | Solid, MultiSurface, CompositeSurface |

## Performance Considerations

### For Cloud-Optimized Formats

- Minimize seeks for sequential access patterns
- Align data structures to enable efficient range requests
- Consider compression block boundaries for partial decompression
- Profile memory usage with large datasets (millions of vertices)

### Benchmarking Requirements

When modifying performance-critical code:

```rust
#[bench]
fn bench_transform_vertices(b: &mut Bencher) {
    let vertices: Vec<[i64; 3]> = load_test_vertices();
    let transform = Transform::default();

    b.iter(|| {
        vertices.iter()
            .map(|v| transform.apply(*v))
            .collect::<Vec<_>>()
    });
}
```

## References

- [CityJSON Specifications](https://www.cityjson.org/specs/)
- [CityJSON Schemas](https://www.cityjson.org/schemas/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Error Handling in Rust](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
