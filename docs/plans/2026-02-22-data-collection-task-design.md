# Design: Open 3D City Model Data Collection Task

**Date:** 2026-02-22
**Author:** Design brainstorming session
**Status:** Approved

## Overview

Create a comprehensive task document (`task.md`) that guides colleagues through collecting open 3D city model datasets from public sources and creating standardized STAC collection configuration files.

## Objectives

1. **Primary Goal:** Build a comprehensive global STAC catalog of open 3D city model datasets
2. **Secondary Goal:** Establish a reproducible workflow for data collection and configuration
3. **Tertiary Goal:** Enable automatic validation of config files before submission

## Target Audience

- Colleagues who may not be familiar with STAC or CityJSON
- Need step-by-step guidance (assume no prior knowledge)
- Will work independently to collect datasets and create configs

## Data Sources

**Primary Sources:**
- https://github.com/OloOcki/awesome-citygml#World - Curated list of open 3D city models
- https://3d.bk.tudelft.nl/opendata/opencities/ - TU Delft's open cities portal

**Additional Discovery:**
- Municipal open data portals
- National geospatial clearinghouses
- Academic/research institution repositories
- CityGML/CityJSON community resources

## Accepted Formats

- **CityJSON** (`.json`, `.jsonl` for sequences)
- **CityGML** (`.gml`, `.zip`, `.xml`)
- Other 3D city model formats (document in comments)

## Access Methods

1. **Direct download links** - Simple URLs to downloadable files
2. **API endpoints** - Programmatic access requiring specific calls
3. **Data portals** - Multi-file downloads or web interfaces

## Config File Requirements

### Structure

All configuration files use **YAML format** with the following comprehensive field set:

```yaml
# Core Metadata (Required)
id: unique-identifier
title: Human Readable Title
description: |
  Detailed description of the dataset,
  including coverage, completeness, and notable features.
license: SPDX-LICENSE-IDENTIFIER

# Categorization
keywords:
  - 3d city model
  - city name
  - country
  - additional tags

# Providers (At least one required)
providers:
  - name: Organization Name
    url: https://example.com
    roles:
      - producer
      - licensor
    description: Brief description of organization

# Data Access (Required)
inputs:
  - https://example.com/data.city.json
  # or API endpoint documented in comments

# Spatial/Temporal Coverage (Required when available)
extent:
  spatial:
    bbox: [minx, miny, minz, maxx, maxy, maxz]
    crs: EPSG:XXXX
  temporal:
    start: "YYYY-MM-DDTHH:MM:SSZ"
    end: "YYYY-MM-DDTHH:MM:SSZ"  # or null for open-ended

# Additional Links (Optional)
links:
  - rel: license
    href: https://license-url
    type: text/html
    title: License Text
  - rel: about
    href: https://project-page
    type: text/html
    title: Project Homepage

# Custom Summaries (Optional)
summaries:
  custom:field: value
```

### Required Fields

- `id` - Unique collection identifier
- `title` - Human-readable title
- `description` - Detailed description
- `license` - SPDX license identifier
- `providers` - At least one provider with name and roles
- `inputs` - At least one data source (URL or API endpoint)

### Handling Unknown/Missing Metadata

When metadata is unavailable:
- **Spatial extent:** Document known location in description, omit `extent.spatial`
- **Temporal extent:** Use known year in `start` field, document uncertainty in description
- **CRS:** If unknown, document in description (will be auto-detected from data during processing)
- **Contact:** Use generic provider information from source page

### Special Cases

**API Endpoints:**
```yaml
inputs:
  - https://api.example.com/cityjson endpoint
# Document API call pattern in comments:
# GET /api/v1/cityjson?city=Rotterdam&format=json
# Requires: API key in header (see https://example.com/docs)
```

**Multiple Files:**
```yaml
inputs:
  - https://example.com/district1.city.json
  - https://example.com/district2.city.json
  # or use glob pattern if same base URL:
  - https://example.com/data/*.city.json
```

## File Organization

### Directory Structure

```
cityjson-stac/
├── collections/              # All collection configs go here
│   ├── delft-3dbag.yaml
│   ├── rotterdam-3dbag.yaml
│   ├── berlin-citygml.yaml
│   └── ...
├── task.md                   # This document
└── ...
```

### Naming Conventions

**Config files:** `<city-or-region>-<provider>.yaml`
- Use lowercase, hyphen-separated names
- Include geographic identifier
- Include source/provider identifier
- Examples:
  - `delft-3dbag.yaml`
  - `rotterdam-opendata.yaml`
  - `singapore-ura.yaml`
  - `zurich-statistics.yaml`

**Collection IDs:** `<provider>-<city>-<year>` (when year is known)
- Examples:
  - `3dbag-delft-2023`
  - `opendata-rotterdam-2022`
  - `ura-singapore-2024`

## Workflow

### Step-by-Step Process

For each dataset discovered:

1. **Identify dataset format**
   - Verify it's CityJSON or CityGML
   - Note file extensions and structure
   - Determine access method (direct/API/portal)

2. **Extract metadata**
   - Title and description from source page
   - License from download page or data license document
   - Provider information (organization, URL, contact)
   - Spatial coverage (city name, bbox if available)
   - Temporal coverage (year, date range if available)

3. **Create config file**
   - Use appropriate template (direct download vs API)
   - Fill in all required fields
   - Add comments for complex access methods
   - Save with appropriate naming convention

4. **Validate config**
   - Run: `cityjson-stac collection --config <file>.yaml --dry-run`
   - Fix any validation errors
   - Ensure all required fields are present

5. **Submit file**
   - Place in `collections/` directory
   - Hand over files for review

## Quality Assurance

### Validation Command

Use the `--dry-run` flag to validate config files without processing data:

```bash
cityjson-stac collection --config collections/delft-3dbag.yaml --dry-run
```

Expected output:
- ✓ Config file is valid YAML
- ✓ All required fields present
- ✓ URLs are well-formed
- ✗ Issues found (if any)

### Pre-Submission Checklist

Before submitting a config file, verify:

- [ ] YAML syntax is valid (use online YAML validator if needed)
- [ ] All required fields are present (`id`, `title`, `description`, `license`, `providers`, `inputs`)
- [ ] URL is accessible (test in browser or with curl)
- [ ] License is properly identified (SPDX identifier)
- [ ] Provider has at least a name and role
- [ ] File follows naming convention
- [ ] Config passes `--dry-run` validation
- [ ] Comments document any special access requirements

## Common Scenarios

### Scenario 1: Simple Direct Download

Source: https://example.com/delft.city.json

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

### Scenario 2: API Endpoint

Source: https://api.example.com/docs

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
# API Documentation: https://api.example.com/docs
# Example call: GET /v1/cityjson?bbox=8.3,48.9,8.5,49.1
# Returns: CityJSON FeatureCollection
# Rate limit: 100 requests/minute
links:
  - rel: about
    href: https://api.example.com/docs
    type: text/html
    title: API Documentation
```

### Scenario 3: Multiple District Files

Source: https://example.com/citydata/

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

### Scenario 4: CityGML Format

Source: https://example.com/newyork.gml.zip

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
# Format: CityGML 2.0
# Compression: ZIP containing GML files
# Projection: EPSG:2263 (NAD83 / New York Long Island)
```

## Deliverables

**Expected Output:**
- Multiple YAML configuration files in `collections/` directory
- Each file represents one open 3D city model dataset
- Files follow naming convention and pass validation
- Comments document any special access requirements

**Submission Format:**
- Hand over files directly for review
- Create pull request to `collections/` directory
- Include summary of datasets collected (optional)

## Next Steps

After task document is created:
1. Implement `--dry-run` validation flag for config files
2. Create `collections/` directory structure
3. Review and finalize task document
4. Distribute task to colleagues
