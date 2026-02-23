# Task: Collect Open 3D City Model Datasets

**Objective:** Create configuration files for open 3D city model datasets to build a comprehensive global STAC catalog.

**Deliverable:** Multiple YAML configuration files (one per dataset) in the `collections/` directory.

---

## Table of Contents

1. [Background](#1-background)
2. [Prerequisites](#2-prerequisites)
3. [Understanding the Config File](#3-understanding-the-config-file)
4. [Data Sources](#4-data-sources)
5. [Step-by-Step Workflow](#5-step-by-step-workflow)
6. [Common Scenarios](#6-common-scenarios)
7. [Quality Checklist](#7-quality-checklist)
8. [Naming Conventions](#8-naming-conventions)
9. [Submitting Your Work](#9-submitting-your-work)

---

## 1. Background

### What is STAC?

[STAC](https://stacspec.org/) (SpatioTemporal Asset Catalog) is a specification for describing geospatial data. Think of it as a standardized way to catalog and search for spatial datasets.

### What are we building?

We're creating a **global catalog of open 3D city models**. These are digital representations of cities in 3D, including buildings, roads, vegetation, and more. Common formats include:

- **CityJSON** - A JSON format for 3D city models
- **CityGML** - An XML/GML format for 3D city models

### Why this matters

Currently, finding open 3D city model datasets is difficult - they're scattered across various websites, portals, and repositories. By creating a standardized STAC catalog, we make it easy for:

- Researchers to discover and access 3D city data
- Developers to build applications using this data
- Cities to share their 3D models with the world

### Your role

You'll be **finding open 3D city model datasets** and **creating configuration files** that describe each dataset. These configs will be used to automatically generate STAC metadata.

---

## 2. Prerequisites

### What you need

- **Text editor** - Any code editor (VS Code, Sublime, etc.) or even a simple text editor
- **Web browser** - To access data sources and verify URLs
- **Basic YAML understanding** - YAML is a simple configuration format (see examples below)

### Helpful but not required

- Familiarity with geospatial data concepts
- Understanding of coordinate reference systems (CRS)
- Knowledge of open data licenses

### Setup

1. Clone or access this repository
2. Navigate to the project directory
3. Create a `collections/` directory if it doesn't exist:
   ```bash
   mkdir -p collections
   ```

---

## 3. Understanding the Config File

Each dataset needs a **YAML configuration file**. Here's the complete structure:

### Required Fields

Every config file MUST have:

```yaml
id: unique-identifier
title: Human Readable Title
description: Detailed description
license: SPDX-LICENSE-IDENTIFIER
providers:
  - name: Organization Name
    roles:
      - producer
inputs:
  - https://example.com/data.city.json
```

### Complete Field Reference

| Field             | Required? | Description                                   | Example                          |
| ----------------- | --------- | --------------------------------------------- | -------------------------------- |
| `id`              | Yes       | Unique identifier for the collection          | `3dbag-delft-2023`               |
| `title`           | Yes       | Human-readable name                           | `3DBAG Delft`                    |
| `description`     | Yes       | Detailed description (use `\|` for multiline) | See below                        |
| `license`         | Yes       | SPDX license identifier                       | `CC-BY-4.0`                      |
| `keywords`        | No        | List of tags for categorization               | `["3d city model", "buildings"]` |
| `providers`       | Yes       | List of organizations involved                | See below                        |
| `inputs`          | Yes       | Data source URLs or API endpoints             | See below                        |
| `extent.spatial`  | No        | Bounding box and CRS                          | See below                        |
| `extent.temporal` | No        | Time range of data                            | See below                        |
| `links`           | No        | Related URLs (license, about, etc.)           | See below                        |
| `summaries`       | No        | Custom metadata fields                        | Various                          |

### Provider Roles

Each provider should have at least one role:

| Role        | When to Use                                      |
| ----------- | ------------------------------------------------ |
| `producer`  | Organization that created the data               |
| `licensor`  | Organization that manages the license            |
| `processor` | Organization that processed/transformed the data |
| `host`      | Organization that hosts the data                 |
| `curator`   | Organization that maintains the data catalog     |

### Common License Identifiers

| License                          | SPDX ID       | When to Use                            |
| -------------------------------- | ------------- | -------------------------------------- |
| Creative Commons Attribution 4.0 | `CC-BY-4.0`   | Free to use with attribution           |
| Creative Commons Zero            | `CC0-1.0`     | Public domain, no attribution required |
| Open Data Commons Attribution    | `ODC-BY-1.0`  | Open data with attribution             |
| MIT License                      | `MIT`         | Permissive software license            |
| Not specified                    | `proprietary` | Unknown or proprietary license         |

**Find more:** https://spdx.org/licenses/

---

## 4. Data Sources

### Primary Sources to Explore

#### 1. Awesome CityGML (GitHub)

🔗 https://github.com/OloOcki/awesome-citygml#World

A curated list of open 3D city model datasets organized by country. This is your main source.

**How to use:**

- Navigate to different country sections
- Click through to dataset pages
- Extract metadata from those pages

#### 2. TU Delft Open Cities

🔗 https://3d.bk.tudelft.nl/opendata/opencities/

Academic repository of open 3D city models maintained by TU Delft.

**How to use:**

- Browse by city or country
- Check for download links or API documentation
- Note any special access requirements

### Additional Discovery

Look for datasets in:

- **Municipal open data portals** - Search "[city name] open data 3d city model"
- **National geospatial portals** - Many countries have national clearinghouses
- **Research repositories** - University datasets often available
- **CityGML/CityJSON communities** - Forums and community sites

### What to Include

✅ **Include datasets that are:**

- Openly accessible (no payment required)
- In CityJSON or CityGML format
- Documented with a license
- From verifiable sources

❌ **Don't include:**

- Commercial datasets requiring payment
- Unpublished or internal datasets
- Broken or inaccessible links
- Datasets with unclear licensing

---

## 5. Step-by-Step Workflow

Follow these steps for each dataset you discover:

### Step 1: Identify the Dataset

1. Visit the dataset page
2. Verify it's a 3D city model (CityJSON/CityGML)
3. Check the file format and size
4. Determine how to access the data:
   - **Direct download** - Simple URL to a file
   - **API endpoint** - Requires API calls
   - **Data portal** - Web interface or multiple files

### Step 2: Extract Metadata

Collect the following information from the dataset page:

**Basic Information:**

- Title (official name of the dataset)
- Description (what's included, coverage, completeness)
- License (look for "license", "terms of use", or "data rights")

**Provider Information:**

- Organization name (who created/published the data)
- Organization URL
- Contact information (if available)

**Data Access:**

- Download URL(s)
- API documentation (if applicable)
- Authentication requirements (API keys, etc.)

**Spatial/Temporal Coverage:**

- Geographic area (city name, country)
- Bounding box coordinates (if provided)
- Year or date range of the data

### Step 3: Create the Config File

1. Create a new YAML file in the `collections/` directory
2. Use the appropriate template from Section 6
3. Fill in all required fields
4. Add comments for any special access requirements
5. Save with the appropriate filename (see Section 8)

### Step 4: Validate the Config

Use the `--dry-run` flag to validate your config:

```bash
cityjson-stac collection --config collections/your-file.yaml --dry-run
```

**Expected output:**

```
✓ Config file is valid YAML
✓ All required fields present
✓ URLs are well-formed
✓ License is valid SPDX identifier
✓ Provider information complete
```

If there are errors, fix them and re-run the validation.

### Step 5: Test the URL (Optional but Recommended)

Verify the data URL is accessible:

```bash
# For direct downloads
curl -I https://example.com/data.city.json

# For API endpoints
curl https://api.example.com/endpoint
```

### Step 6: Submit the File

Once validated:

1. Place the YAML file in the `collections/` directory
2. Ensure it follows naming conventions
3. Submit all files for review

---

## 6. Common Scenarios

### Scenario 1: Simple Direct Download

**Use when:** A single downloadable file (e.g., `.city.json`, `.zip`)

**Example:**

```yaml
id: 3dbag-delft-2023
title: 3DBAG Delft
description: |
  Complete 3D building model of Delft, Netherlands.
  Contains all buildings with LoD2.2 geometry including
  building parts and semantic surfaces.
license: CC-BY-4.0
keywords:
  - 3d city model
  - buildings
  - netherlands
  - delft
  - lod2
providers:
  - name: 3DBAG
    url: https://3dbag.nl
    roles:
      - producer
      - licensor
    description: 3D Basisregistratie Adressen en Gebouwen
inputs:
  - https://example.com/delft.city.json
```

---

### Scenario 2: API Endpoint

**Use when:** Data is accessed via an API (not direct download)

**Example:**

```yaml
id: citylab-karlsruhe-2022
title: CityLab Karlsruhe 3D City Model
description: |
  3D city model of Karlsruhe accessible via API.
  Includes buildings, vegetation, and transportation
  features at multiple Levels of Detail.
license: CC-BY-4.0
keywords:
  - 3d city model
  - germany
  - karlsruhe
  - api
providers:
  - name: CityLab Karlsruhe
    url: https://citylab.example.de
    roles:
      - producer
      - host
    description: Municipal 3D geoinformation lab
inputs:
  - https://api.example.com/v1/cityjson endpoint

# Documentation:
# API Docs: https://api.example.com/docs
# Example call: GET /v1/cityjson?bbox=8.3,48.9,8.5,49.1
# Returns: CityJSON FeatureCollection
# Authentication: None required
# Rate limit: 100 requests/minute

links:
  - rel: about
    href: https://api.example.com/docs
    type: text/html
    title: API Documentation
```

---

### Scenario 3: Multiple Files

**Use when:** Dataset split across multiple files (districts, tiles, etc.)

**Example:**

```yaml
id: opendata-zurich-2023
title: Open Data Zurich 3D
description: |
  Complete 3D city model of Zurich organized by district.
  Each district is provided as a separate CityJSON file.
  Includes buildings (LoD2), vegetation, and transportation.
license: CC0-1.0
keywords:
  - 3d city model
  - switzerland
  - zurich
  - lod2
providers:
  - name: City of Zurich
    url: https://www.stadt-zuerich.ch
    roles:
      - producer
      - licensor
    description: City of Zurich open data portal
inputs:
  - https://example.com/citydata/district-01.city.json
  - https://example.com/citydata/district-02.city.json
  - https://example.com/citydata/district-03.city.json
  - https://example.com/citydata/district-04.city.json
  - https://example.com/citydata/district-05.city.json
  - https://example.com/citydata/district-06.city.json
  - https://example.com/citydata/district-07.city.json
  - https://example.com/citydata/district-08.city.json
  - https://example.com/citydata/district-09.city.json
  - https://example.com/citydata/district-10.city.json
  - https://example.com/citydata/district-11.city.json
  - https://example.com/citydata/district-12.city.json
```

---

### Scenario 4: CityGML Format

**Use when:** Dataset is in CityGML format (not CityJSON)

**Example:**

```yaml
id: opendata-nyc-2021
title: New York City 3D Building Model
description: |
  3D building model of New York City in CityGML format.
  Contains approximately 1 million building footprints
  with height information and building attributes.
license: CC-BY-4.0
keywords:
  - 3d city model
  - citygml
  - usa
  - new york
  - buildings
providers:
  - name: NYC Planning
    url: https://www1.nyc.gov/site/planning
    roles:
      - producer
      - licensor
    description: NYC Department of City Planning
inputs:
  - https://example.com/newyork.gml.zip

# Additional Notes:
# - Format: CityGML 2.0
# - Compression: ZIP containing GML files
# - Projection: EPSG:2263 (NAD83 / New York Long Island)
# - Size: ~500 MB compressed
```

---

### Scenario 5: Unknown/Missing Metadata

**Use when:** Some information is not available

**Example:**

```yaml
id: unknown-montreal-2020
title: Montreal 3D City Model
description: |
  3D city model of Montreal, Canada. Exact temporal coverage
  and coordinate reference system not specified in source.
  Includes buildings and transportation networks.
license: CC-BY-4.0
keywords:
  - 3d city model
  - canada
  - montreal
providers:
  - name: City of Montreal
    url: https://ville.montreal.qc.ca
    roles:
      - producer
    description: Municipal open data portal
inputs:
  - https://data.example.com/montreal.city.json

# Note: CRS will be auto-detected from data during processing
# Note: Year estimated from metadata (actual year may vary)
```

---

## 7. Quality Checklist

Before submitting each config file, verify:

### Content Quality

- [ ] All required fields are present (`id`, `title`, `description`, `license`, `providers`, `inputs`)
- [ ] Description is detailed and informative (not just "3D city model")
- [ ] License is a valid SPDX identifier
- [ ] At least one provider with name and role(s)
- [ ] Keywords are relevant and useful for searching

### Data Access

- [ ] URL is accessible (test in browser or with curl)
- [ ] URL format is correct (https://...)
- [ ] API endpoints are documented in comments
- [ ] Special access requirements (API keys, auth) are noted

### Formatting

- [ ] YAML syntax is valid (indentation with spaces, not tabs)
- [ ] Lists use proper YAML syntax (`-` for items)
- [ ] Multiline strings use `\|` or `>` correctly
- [ ] Comments document any special cases

### Validation

- [ ] Config passes `--dry-run` validation
- [ ] No validation errors or warnings
- [ ] File follows naming convention (Section 8)

### Validation Command

```bash
# Validate a single file
cityjson-stac collection --config collections/your-file.yaml --dry-run

# Validate all files at once (bash loop)
for file in collections/*.yaml; do
  echo "Validating $file..."
  cityjson-stac collection --config "$file" --dry-run
done
```

---

## 8. Naming Conventions

### Config File Names

Format: `<city-or-region>-<provider>.yaml`

**Rules:**

- Use lowercase letters
- Separate words with hyphens (`-`)
- Include geographic identifier (city or region)
- Include source/provider identifier
- Use `.yaml` extension

**Examples:**

| Dataset                          | File Name                 |
| -------------------------------- | ------------------------- |
| Delft from 3DBAG                 | `delft-3dbag.yaml`        |
| Rotterdam from Open Data         | `rotterdam-opendata.yaml` |
| Singapore from URA               | `singapore-ura.yaml`      |
| Zurich from Statistics Office    | `zurich-statistics.yaml`  |
| New York City from Planning Dept | `nyc-planning.yaml`       |

### Collection IDs

Format: `<provider>-<city>-<year>` (when year is known)

**Rules:**

- Use lowercase
- Include provider/source name
- Include city name
- Include year if known
- Use hyphens to separate parts

**Examples:**

| Dataset                  | Collection ID               |
| ------------------------ | --------------------------- |
| 3DBAG Delft 2023         | `3dbag-delft-2023`          |
| Open Data Rotterdam 2022 | `opendata-rotterdam-2022`   |
| URA Singapore 2024       | `ura-singapore-2024`        |
| Unknown year             | `provider-city` (omit year) |

---

## 9. Submitting Your Work

### Deliverables

**Submit:**

- Multiple YAML configuration files
- One file per dataset
- All files in `collections/` directory
- All files validated with `--dry-run`

**Optional but helpful:**

- Summary document listing collected datasets
- Notes on any issues encountered
- Suggestions for additional sources

### How to Submit

**Option 1: Direct Hand-off**

- Place files in `collections/` directory
- Notify that files are ready for review
- Wait for feedback

**Option 2: Pull Request**

- Create a new git branch
- Commit your config files
- Create a pull request
- Address review feedback

### What Happens Next

1. **Review** - Your configs will be reviewed for completeness and accuracy
2. **Testing** - Each config will be tested to ensure it works with `cityjson-stac`
3. **Integration** - Valid configs will be added to the catalog
4. **Processing** - STAC metadata will be generated from your configs

---

## 10. Troubleshooting

### Common Issues

**Issue:** `--dry-run` validation fails

- **Solution:** Check YAML syntax, ensure all required fields are present

**Issue:** URL is not accessible

- **Solution:** Verify the URL is correct, check if it requires authentication

**Issue:** Can't find license information

- **Solution:** Use `proprietary` and document in comments that license is unclear

**Issue:** API endpoint requires authentication

- **Solution:** Document authentication requirements in comments, include API docs link

**Issue:** Multiple conflicting sources for same city

- **Solution:** Create separate configs for each, include version/year in description

### Getting Help

If you encounter issues not covered here:

1. Check the project README and documentation
2. Review example config files in `examples/`
3. Ask a team member for clarification
4. Document the issue for future reference

---

## 11. Quick Reference

### Essential Fields

```yaml
id: unique-id
title: Dataset Title
description: Detailed description
license: CC-BY-4.0
providers:
  - name: Organization
    roles: [producer]
inputs:
  - https://example.com/data.json
```

### Common Licenses

- `CC-BY-4.0` - Creative Commons Attribution
- `CC0-1.0` - Public Domain
- `ODC-BY-1.0` - Open Data Commons
- `proprietary` - Unknown/restricted

### Provider Roles

- `producer` - Created the data
- `licensor` - Manages the license
- `processor` - Transformed the data
- `host` - Hosts the data

### Validation Command

```bash
cityjson-stac collection --config collections/file.yaml --dry-run
```

---

## Appendix A: Field Types Reference

### Strings

Simple text values:

```yaml
title: Delft 3D City Model
license: CC-BY-4.0
```

### Multiline Strings

Use `|` for literal multiline:

```yaml
description: |
  This is a long description
  that spans multiple lines.
  Formatting is preserved.
```

### Lists

Use `-` for list items:

```yaml
keywords:
  - 3d city model
  - buildings
  - netherlands
```

### Nested Objects

Indent with 2 spaces:

```yaml
providers:
  - name: Organization
    url: https://example.com
    roles:
      - producer
      - licensor
```

### Comments

Use `#` for comments:

```yaml
inputs:
  - https://example.com/data.json # Main dataset
  # API endpoint: https://api.example.com/v1/cityjson
```

---

## Appendix B: Resources

### STAC Specification

- https://stacspec.org/ - Core STAC standard
- https://stac-extensions.github.io/ - STAC extensions

### CityJSON / CityGML

- https://www.cityjson.org/ - CityJSON specification

### Licenses

- https://spdx.org/licenses/ - SPDX license list
- https://choosealicense.com/ - License guide

### Coordinate Reference Systems

- https://epsg.io/ - CRS registry
- https://spatialreference.org/ - CRS reference

---

**Good luck with your data collection!** 🏙️

If you have questions, don't hesitate to ask. Your contribution will help make 3D city data more accessible to everyone.
