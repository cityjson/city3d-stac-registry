# Scientific Paper Notes: CityJSON-STAC Extension

This document provides guidance and notes for writing an academic paper about the CityJSON STAC Extension project. It is not the paper itself, but a structured set of ideas, comparisons, and evaluation strategies.

---

## 1. Potential Academic Contributions

### 1.1 Primary Contribution: Bridging 3D City Models and Cloud-Native Geospatial Infrastructure

**Claim**: This work is the first to propose and implement a formal STAC extension for CityJSON, bridging the gap between the 3D city modeling domain and the cloud-native geospatial ecosystem.

**Significance**:

- **For the 3D City Modeling Community**: Provides a standardized, machine-readable way to catalog and discover CityJSON datasets, addressing the "data silo" problem identified by Biljecki et al. (3D City Index, 2023).
- **For the Cloud-Native Geospatial Community**: Extends STAC's domain coverage into 3D urban data, a rapidly growing area due to digital twins and smart city initiatives.

### 1.2 Secondary Contribution: A Lightweight Metadata Profile for 3D Geospatial Data

**Claim**: The `cj:` extension properties define a JSON-native metadata profile that captures the essential discovery and fitness-for-use attributes of 3D city models (LOD, Object Types, Feature Counts) without requiring the full complexity of ISO 19115 or CityGML ADEs.

**Significance**:

- Aligns with the industry trend (seen in OGC API standards) of favoring JSON-native metadata over legacy XML profiles.
- Reduces the barrier to entry for web developers who want to build 3D city model catalogs.

### 1.3 Tertiary Contribution: Cloud-Optimized Format Awareness

**Claim**: By explicitly supporting `CityJSONSeq` and `FlatCityBuf` as first-class encoding types, the extension encourages the adoption of streaming-friendly formats over monolithic JSON files.

**Significance**:

- Provides a clear path for migrating existing CityJSON archives to cloud-optimized storage.
- Enables clients to make intelligent format choices based on their access patterns (e.g., random access vs. sequential streaming).

---

## 2. Comparison with Related Work

| Aspect                         | This Work (CityJSON-STAC)                  | CityGML Metadata ADE (Labetski, 2018) | 3D City Index (Biljecki, 2023) | ISO 19115 / DCAT           |
| ------------------------------ | ------------------------------------------ | ------------------------------------- | ------------------------------ | -------------------------- |
| **Metadata Locus**             | External catalog (STAC JSON)               | Embedded in data file (XML)           | Survey/Report                  | External catalog (XML/RDF) |
| **Format**                     | JSON                                       | XML (CityGML extension)               | N/A (Methodology)              | XML or RDF                 |
| **Primary Use Case**           | Data Discovery & API Querying              | Data Archival & Transfer              | Quality Assessment             | Archival & SDI Compliance  |
| **Supported Data Types**       | CityJSON, CityJSONSeq, FlatCityBuf         | CityGML only                          | Any 3D city model format       | Generic geospatial         |
| **Coordinate System Handling** | `proj:epsg` from STAC standard             | Native CRS element                    | N/A                            | `RS_Identifier`            |
| **LOD Representation**         | `cj:lods` (array of strings)               | `lod` element                         | Evaluated as quality criterion | Not specific to LOD        |
| **City Object Type List**      | `cj:co_types` (array)                      | Implicit in schema                    | Evaluated                      | Not specific               |
| **Streaming Support**          | First-class (`CityJSONSeq`, `FlatCityBuf`) | Not applicable                        | N/A                            | Not applicable             |
| **Query API**                  | STAC API (OGC compliant)                   | None (file-based)                     | N/A                            | OGC CSW (legacy)           |

### Key Differentiators to Emphasize

1.  **"Cloud-Native" vs. "File-Embedded"**: Labetski's ADE stores metadata _inside_ the data file. Our approach stores it _alongside_ the data in a separate, lightweight JSON document designed for web APIs. This is analogous to the shift from GeoTIFF's embedded metadata to STAC's sidecar approach in the raster community.
2.  **Actionable vs. Descriptive**: The 3D City Index (Biljecki) provides a _framework for evaluation_. Our work provides an _implementable specification and tool_ that enables the kind of automated quality assessment the Index calls for.
3.  **Interoperability with Existing Ecosystem**: Unlike a new standalone standard, STAC is already widely adopted. A STAC catalog of CityJSON files can be immediately ingested by existing tools like STAC Browser, PySTAC, and cloud platforms (e.g., Microsoft Planetary Computer).

---

## 3. Evaluation Methodology

### 3.1 Functional Validation

**Goal**: Demonstrate the extension correctly captures CityJSON metadata.

**Method**:

1.  Select a diverse corpus of CityJSON files (varying LODs, object types, CRS, file sizes). Sources: 3DBAG (Netherlands), NYC Open Data, Berlin 3D Model.
2.  Run the `cityjson-stac` CLI tool to generate STAC Items.
3.  Validate each generated STAC Item against the JSON Schema (`stac-extension/schema.json`).
4.  Manually verify a sample of Items to ensure metadata accuracy (e.g., `cj:city_objects` count matches actual count).

**Metrics**:

- Schema validation pass rate (should be 100%).
- Metadata accuracy rate (sample-based).

### 3.2 Scalability & Performance Evaluation

**Goal**: Assess the tool's performance on large-scale datasets.

**Method**:

1.  Prepare a test directory with 10, 100, 1000, and 10,000 CityJSON files (can use tiled 3DBAG data).
2.  Measure time and peak memory usage for the `collection` command.
3.  Compare with a baseline (e.g., naive full-file parsing in Python with `cjio`).

**Metrics**:

- Wall-clock time per file.
- Peak memory consumption.
- Scalability curve (linear, sub-linear, or super-linear?).

**Expected Finding**: The current implementation may show super-linear memory growth due to the identified OOM issue. This can be presented as a "known limitation" with a proposed solution (streaming aggregation).

### 3.3 Interoperability Demonstration

**Goal**: Show that generated STAC catalogs work with the broader ecosystem.

**Method**:

1.  Generate a STAC Catalog from a sample CityJSON collection.
2.  Serve it using a standard STAC API server (e.g., `stac-fastapi`).
3.  Query the catalog using standard tools:
    - **STAC Browser**: Visualize the catalog in a web UI.
    - **PySTAC Client**: Write a Python script to query by `cj:lods` or `cj:co_types`.
    - **QGIS STAC Plugin**: Demonstrate loading in a desktop GIS.

**Metrics**:

- Success/Failure of each integration.
- Qualitative: Screenshots and workflow descriptions.

### 3.4 Comparative User Study (Optional, for Stronger Paper)

**Goal**: Show that the STAC-based approach improves data discovery compared to traditional file-system browsing.

**Method**:

1.  Recruit participants (e.g., GIS students or urban planners).
2.  Task: "Find all 3D building models with LOD2 or higher in the Rotterdam area."
3.  Group A: Uses a file-server with CityJSON files (must download/open files to inspect).
4.  Group B: Uses a STAC Browser interface.
5.  Measure: Time to complete task, number of files incorrectly downloaded (false positives), subjective satisfaction.

**Metrics**:

- Task completion time.
- Error rate.
- System Usability Scale (SUS) score.

---

## 4. Suggested Paper Structure

1.  **Introduction**
    - Problem: 3D city models are increasingly available, but discovering and assessing their fitness-for-use is difficult.
    - Solution: A STAC extension for CityJSON to enable cloud-native cataloging.
    - Contributions: (List the 2-3 main contributions).

2.  **Background & Related Work**
    - CityJSON and its ecosystem (cjio, CityJSONSeq, FlatCityBuf).
    - Geospatial metadata standards (ISO 19115, DCAT, OGC API Records).
    - STAC: Origins, adoption, extension mechanism.
    - Prior work on 3D city model metadata (Labetski ADE, 3D City Index).

3.  **The CityJSON STAC Extension**
    - Specification overview (Item properties, Collection summaries).
    - Design rationale (Section 4 from `review.md`).
    - Schema definition.

4.  **Implementation**
    - Architecture (Reader trait, Builder pattern).
    - Support for multiple encodings.
    - CLI tool.

5.  **Evaluation**
    - Functional validation results.
    - Performance benchmarks.
    - Interoperability demonstration.

6.  **Discussion**
    - Limitations (OOM, datetime handling, WKT2 support).
    - Comparison with embedding metadata in files.
    - Future work (FlatCityBuf reader, CityParquet, streaming aggregation).

7.  **Conclusion**

---

## 5. Target Venues

- **ISPRS International Journal of Geo-Information (IJGI)**: Open access, good fit for CityJSON/3D city model work. Accepts software papers.
- **Transactions in GIS**: Strong for methodological contributions.
- **Journal of Open Source Software (JOSS)**: If the primary contribution is the tool itself. Requires a short paper + open-source code review.
- **AGILE Conference / GIScience Conference**: Peer-reviewed conferences with proceedings.
- **Open Geospatial Data, Software and Standards**: Springer journal, explicitly for this type of work.

---

## 6. Potential Weaknesses & Counterarguments

| Potential Criticism                                  | Counterargument                                                                                                                                                                                                                                                                      |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| "This is just metadata mapping, not novel research." | The novelty lies in the _integration_ with a widely-adopted cloud-native standard (STAC) and the formal specification of domain-specific fields (`cj:lods`, `cj:co_types`) that enable new query capabilities. The 3D City Index emphasizes metadata availability as a critical gap. |
| "Why not use existing ISO 19115?"                    | ISO 19115 is verbose and requires XML tooling. Our approach provides a minimal, JSON-native profile that is directly usable by web APIs. This mirrors the industry shift from OGC CSW (XML) to OGC API Records (JSON).                                                               |
| "The tool has scalability issues (OOM)."             | Acknowledged as a known limitation with a clear path to resolution. The architectural pattern (trait-based readers) supports a fix without changing the core specification.                                                                                                          |
| "FlatCityBuf is not a widely adopted format."        | Including it demonstrates forward-thinking design. The extension is designed to accommodate new cloud-optimized formats as they emerge.                                                                                                                                              |

---

## 7. Figures to Include

1.  **Conceptual Diagram**: Show the relationship between CityJSON files, the STAC Extension, and a STAC API.
2.  **UML-like Class Diagram**: Show the Rust `CityModelMetadataReader` trait and its implementations.
3.  **Example STAC Item JSON**: A well-formatted JSON snippet showing key `cj:` properties.
4.  **Scalability Chart**: Bar chart of processing time/memory for increasing file counts.
5.  **Screenshot of STAC Browser**: Showing a CityJSON collection being explored.

---

## 8. Code & Data Availability Statement (Important for Reproducibility)

> The CityJSON STAC Extension specification, JSON Schema, and reference implementation (`cityjson-stac` CLI tool) are available as open-source software under the [MIT/Apache 2.0] license at: `https://github.com/cityjson/cityjson-stac`. The test datasets used in the evaluation are derived from publicly available sources: 3DBAG (https://3dbag.nl), [other sources].
