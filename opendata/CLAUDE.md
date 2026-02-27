# Open Data Crawler Guidelines

This directory contains STAC configuration files for open 3D city model datasets. Each dataset needs to have its data endpoints crawled and documented.

## Overview

Each `*-config.yaml` file represents a STAC Collection configuration for a specific open 3D city model dataset. The `inputs` field should contain URLs or paths to all data files for that collection.

## Agent Workflow

When assigned to work on a specific dataset:

### 1. Read the Dataset Config

```bash
# Read the config file to understand:
# - Dataset name and description
# - Provider information and URLs
# - Links to source websites
cat /workspaces/cityjson-stac/opendata/{dataset}-config.yaml
```

### 2. Check Existing Plans

First, check if a plan already exists:

```bash
# List all plans
ls -la plan/

# Check the plan index for status
cat plan/plan-index.md

# Read existing plan if available
cat plan/{dataset}-plan.md
```

### 3. Create or Update plan.md

If no plan exists, copy the template:

```bash
cp plan/template.md plan/{dataset}-plan.md
```

Edit `plan/{dataset}-plan.md` with:
- **Data Source URL**: Main website or API endpoint
- **Access Method**: How to access the data (direct download, API, HTML scraping, etc.)
- **Discovery Strategy**: How to find all file URLs
- **Data Format**: File extensions (.json, .jsonl, .gml, .zip, etc.)
- **Pagination**: If data is paginated, how to navigate
- **Rate Limiting**: Any rate limits or access restrictions
- **Authentication**: If auth is needed
- **Estimated File Count**: Rough estimate if available
- **Special Notes**: Any quirks or special handling needed
- **NOTE: HELP NEEDED**: Add this line if you need human input (API keys, special access, etc.)

Update `plan/plan-index.md` with your progress.

### 3. Implement Crawler (if needed)

Based on the plan, implement a crawler script in `scripts/crawlers/`:
- Use Python with `requests` for simple APIs
- Use `playwright` or `selenium` for JavaScript-heavy sites
- Use `BeautifulSoup` for HTML scraping
- Use `wget` or `curl` for simple recursive downloads

### 4. Update the Config File

Add discovered URLs to the `inputs` field in the config:

```yaml
inputs:
  - https://example.com/data1.city.json
  - https://example.com/data2.city.json
  # ... all discovered URLs
```

### 5. Validate URLs

Before committing, verify URLs are accessible:

```bash
# Check a sample of URLs
curl -I https://example.com/data1.city.json

# Or use the cityjson-stac dry-run mode
cityjson-stac item https://example.com/data1.city.json --dry-run
```

### 6. Update Progress

Update the Progress section below with your findings.

## Plan Directory Structure

```
opendata/
├── plan/
│   ├── template.md              # Template for new plan files
│   ├── plan-index.md            # Master index of all plans and their status
│   ├── vienna-plan.md           # Vienna crawl plan
│   ├── singapore-plan.md        # Singapore HDB crawl plan
│   ├── netherlands-3d-bag-plan.md  # 3D BAG tile enumeration plan
│   ├── american-cities-plan.md  # American Cities S3 enumeration plan
│   ├── japan-plateau-plan.md    # Japan PLATEAU 56 cities plan
│   ├── estonia-plan.md          # Estonia geoportal plan
│   ├── german-cities-plan.md    # Berlin, Dresden, Hamburg, etc.
│   ├── finnish-cities-plan.md   # Helsinki, Espoo, Vantaa
│   └── ... (one per dataset or grouped)
└── CLAUDE.md                    # This file
```

## Progress Tracking

See `plan/plan-index.md` for the master progress tracker.

### Overall Status

**Total Configs**: 28 (excluding catalog-config.yaml)
**Total Plans**: 16 (grouped)
**Datasets Covered**: 28/28 (100%)
- ✅ **Complete with URLs**: 18/28 (64%) - configs have populated inputs
- ❌ **Incomplete configs**: 10/28 (36%) - configs have empty inputs: []

### Configs with Populated Inputs (18 complete)

| Config | URLs/Source | Status |
|--------|-------------|--------|
| singapore | 1 URL (GitHub) | ✅ Complete |
| american-cities | from_file (5,229 CityJSON URLs) | ✅ Complete |
| american-cities-citygml | from_file (5,229 CityGML ZIP URLs) | ✅ Complete |
| vienna | 2 URLs (TU Delft) | ✅ Complete |
| montreal | 2 URLs (TU Delft) | ✅ Complete |
| new-york-doitt | 2 URLs (TU Delft) | ✅ Complete |
| japan-plateau | from_file (170,413 URLs) | ✅ Complete |
| estonia | ~50 URLs | ✅ Complete |
| ingolstadt | 2 URLs (TU Delft) | ✅ Complete |
| rotterdam | 3 URLs (TU Delft) | ✅ Complete |
| the-hague | 2 URLs (TU Delft) | ✅ Complete |
| hamburg | 1 URL (CityGML, Transparency Portal) | ✅ Complete |
| namur | 2 URLs (SketchUp format, Namur Portal) | ✅ Complete |
| netherlands-3d-bag | from_file (8,941 tiles via FlatGeobuf) | ✅ Complete |
| lyon | 2 URLs (TU Delft) | ✅ Complete |
| linz | from_file (156 tiles) | ✅ Complete |
| luxembourg | 1 URL (data.public.lu) | ✅ Complete |
| zurich | 1 URL (TU Delft) | ✅ Complete |

**Total URLs ready**: ~190,978+ files (181,880 + 8,941 3DBAG + 156 Linz + 1 Luxembourg)

### Configs with Empty Inputs (10 incomplete)

| Config | Grouping | Blocker |
|--------|----------|---------|
| helsinki, espoo, vantaa | Finnish Cities | WFS-only (not file-based) |
| brussels | Belgian Cities | Access unclear |
| berlin, dresden, potsdam | German Cities | Form-based downloads |
| north-rhine-westphalia | German States | Manual enumeration needed |
| new-york-tum | NYC | Manual download from TUM website |

### Plans Created (16 total)

See `plan/plan-index.md` for the master progress tracker. Current status:

| Dataset | Plan File | Configs | Status | Notes |
|---------|-----------|---------|--------|-------|
| Singapore HDB | singapore-plan.md | singapore | ✅ Complete | 1 CityJSON file from GitHub |
| American Cities | american-cities-plan.md | american-cities | ✅ Complete | 5,229 files via S3 bucket |
| Vienna | vienna-plan.md | vienna | ✅ Complete | CityJSON + CityGML from TU Delft |
| North American | north-american-cities-plan.md | montreal, new-york-doitt | ✅ Complete | Montréal + NYC (CityJSON + CityGML) |
| Zürich | zurich-plan.md | zurich | ✅ Complete | Only CityJSON for Zürich (TU Delft conversion) |
| Japan PLATEAU | japan-plateau-plan.md | japan-plateau | ✅ Complete | 170,413 CityGML files via API |
| Estonia | estonia-plan.md | estonia | 🟡 Partial | ~50 URLs (file truncated, needs fix) |
| Netherlands 3D BAG | netherlands-3d-bag-plan.md | netherlands-3d-bag | ✅ Complete | 8,941 tiles via FlatGeobuf |
| Ingolstadt | ingolstadt-plan.md | ingolstadt | ✅ Complete | 2 URLs from TU Delft |
| Rotterdam | rotterdam-plan.md | rotterdam | ✅ Complete | 3 URLs from TU Delft (inc. textures) |
| The Hague | the-hague-plan.md | the-hague | ✅ Complete | 2 URLs from TU Delft |
| **Linz** | **linz-plan.md** | **linz** | **✅ Complete** | **156 CityGML tiles from Linz geoportal** |
| **Luxembourg** | **luxembourg-plan.md** | **luxembourg** | **✅ Complete** | **1 CityGML file (pilot project)** |
| Finnish Cities | finnish-cities-plan.md | helsinki, espoo, vantaa | 🟡 Blocked | WFS-only (not file-based) |
| Belgian Cities | brussels-namur-plan.md | brussels, namur | 🟡 Blocked | Needs investigation |
| French Cities | french-cities-plan.md | lyon | 🟡 Blocked | JS portal, no public API |
| German Cities | german-cities-plan.md | berlin, hamburg, dresden, potsdam | 🟡 Blocked | Form-based downloads |
| North Rhine-Westphalia | north-rhine-westphalia-plan.md | north-rhine-westphalia | 🟡 Blocked | Manual enumeration needed |

**Plans Status**: 10 complete (63%), 7 blocked/incomplete (44%)

### Plans Needed

The following datasets still need plan files created (use `plan/template.md`):

| Dataset | Config File | Grouping |
|---------|-------------|----------|
| Brussels | brussels-config.yaml | Belgian cities |
| Luxembourg | luxembourg-config.yaml | Benelux |
| Lyon | lyon-config.yaml | French cities |
| Montréal | montreal-config.yaml | Canadian cities |
| Namur | namur-config.yaml | Belgian cities |
| New York (DoITT) | new-york-doitt-config.yaml | NYC (could group) |
| New York (TUM) | new-york-tum-config.yaml | NYC (could group) |
| North Rhine-Westphalia | north-rhine-westphalia-config.yaml | German states |
| Rotterdam | rotterdam-config.yaml | Dutch cities |
| The Hague | the-hague-config.yaml | Dutch cities |
| Zürich | zurich-config.yaml | Swiss cities |

## Common Data Source Patterns

### Pattern 1: Direct S3 Bucket
- **Example**: American Cities (opencitymodel S3 bucket)
- **Access**: Direct HTTPS URLs to S3 objects
- **Discovery**: Pattern-based enumeration or bucket listing
- **Tool**: `aws s3 ls` or manual URL construction

### Pattern 2: Tile-based API
- **Example**: Netherlands 3D BAG (data.3dbag.nl)
- **Access**: Tile-based URL patterns
- **Discovery**: Enumerate tile coordinates (z/x/y)
- **Tool**: Custom script to generate tile URLs

### Pattern 3: GitHub Releases
- **Example**: Singapore HDB
- **Access**: GitHub release assets
- **Discovery**: GitHub API
- **Tool**: `gh` CLI or GitHub API

### Pattern 4: City Open Data Portal
- **Example**: Vienna, Helsinki, Espoo
- **Access**: CKAN, Socrata, or custom portal
- **Discovery**: Portal API or HTML scraping
- **Tool**: Portal-specific API calls

### Pattern 5: FTP/SFTP Server
- **Example**: Some German cities
- **Access**: FTP/SFTP
- **Discovery**: Directory listing
- **Tool**: `lftp`, `wget -r`, or custom script

### Pattern 6: HTTP Directory
- **Example**: Some older portals
- **Access**: HTTP directory listing
- **Discovery**: Parse HTML directory listing
- **Tool**: `wget -r -A "*.json"` or custom script

## Helper Scripts

The following scripts are available in `scripts/crawlers/`:

```bash
# Virtual environment with dependencies
source /workspaces/cityjson-stac/scripts/crawlers/venv/bin/activate

# GitHub repository file crawler
python3 scripts/crawlers/github_file_crawler.py

# American Cities S3 bucket crawler
python3 scripts/crawlers/american_cities_crawler.py

# Japan PLATEAU API crawler (with rate limiting)
python3 scripts/crawlers/plateau_api_crawler.py

# Lyon browser automation crawler (Playwright)
python3 scripts/crawlers/lyon_crawler.py

# Enumerate 3D BAG tiles
python3 scripts/crawlers/3dbag_tiles.py

# Download from S3 bucket
python3 scripts/crawlers/s3_enumerator.py

# Scrape city data portal
python3 scripts/crawlers/portal_scraper.py
```

### Browser Automation Setup (Playwright)

For JavaScript-heavy portals, Playwright is available:

```bash
# Activate venv with Playwright installed
source /workspaces/cityjson-stac/scripts/crawlers/venv/bin/activate

# Run Playwright crawler
python scripts/crawlers/lyon_crawler.py
```

**Note**: Browser automation setup is complete, but many portals (Lyon, Brussels, German cities) either:
- Require authentication/login
- Have no public API
- Use form-based downloads not accessible via automation

## Output Format

All discovered URLs should be added to the `inputs` field as a YAML list:

```yaml
inputs:
  - https://example.com/file1.city.json
  - https://example.com/file2.city.json
  - s3://bucket/path/file3.city.json
  - /local/path/file4.city.json
```

Supported URL schemes:
- `https://` - Remote HTTPS URLs
- `http://` - Remote HTTP URLs (not recommended)
- `s3://` - S3 object URLs
- `/path/to/file` - Local file paths

## Validation

After populating inputs, validate the config:

```bash
# Dry run to check config validity and URL accessibility
cityjson-stac collection --config opendata/{dataset}-config.yaml --dry-run
```

Exit codes:
- `0` - All validations passed
- `1` - Config file error (syntax/semantic)
- `2` - Missing input paths
- `3` - Remote URL inaccessible

## Collaboration Notes

- **Work in parallel**: Multiple agents can work on different datasets simultaneously
- **Document everything**: Even failed attempts should be documented in plan.md
- **Ask for help**: If you encounter authentication, CAPTCHAs, or rate limits, add a NOTE line
- **Be respectful**: Respect rate limits and robots.txt
- **Verify before committing**: Always validate a sample of URLs before updating the config
