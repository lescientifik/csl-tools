# csl-tools

CLI for formatting citations and bibliographies in Markdown documents using CSL (Citation Style Language).

Transform a Markdown document with citation keys into a final document with formatted citations and bibliography:

```
article.md + refs.json + style.csl → article_final.md/html
```

## Features

- Parse `[@citation]` and `[@citation](url)` syntax in Markdown
- Support CSL-JSON and JSONL bibliography formats
- Automatic grouping of adjacent citations (e.g., `[@a] [@b] [@c]` → `(1-3)`)
- Support for Pandoc citation syntax `[@a; @b; @c]`
- Compatible with 10,000+ CSL styles from the [Zotero Style Repository](https://www.zotero.org/styles)
- HTML and Markdown output formats
- Lightweight alternative to Pandoc for citation-only workflows

## Installation

### From source (recommended)

```bash
# Clone the repository
git clone https://github.com/lescientifik/csl-tools.git
cd csl-tools

# Build and install
cargo install --path .
```

### Using cargo install

```bash
cargo install --git https://github.com/lescientifik/csl-tools.git
```

### Build from source without installing

```bash
git clone https://github.com/lescientifik/csl-tools.git
cd csl-tools
cargo build --release

# Binary available at ./target/release/csl-tools
```

### Requirements

- Rust 1.70+ (install via [rustup](https://rustup.rs/))

## Quick Start

1. **Get a CSL style** (e.g., APA):
   ```bash
   curl -sLO https://www.zotero.org/styles/apa
   mv apa apa.csl
   ```

2. **Create your bibliography** (`refs.json`):
   ```json
   [
     {
       "id": "smith2023",
       "type": "article-journal",
       "author": [{"family": "Smith", "given": "John"}],
       "title": "Example Article",
       "container-title": "Journal of Examples",
       "issued": {"date-parts": [[2023]]}
     }
   ]
   ```

3. **Write your document** (`article.md`):
   ```markdown
   # Introduction

   Recent research [@smith2023] shows interesting results.

   ## References
   ```

4. **Process**:
   ```bash
   csl-tools process article.md --bib refs.json --csl apa.csl -o output.html
   ```

## Usage

```bash
# Process a Markdown document
csl-tools process <input.md> --bib <refs.json> --csl <style.csl> [-o output.html]

# Use a builtin style
csl-tools process input.md --bib refs.json --csl minimal -o output.html

# Read from stdin
echo '[@key]' | csl-tools process - --bib refs.json --csl minimal

# List available builtin styles
csl-tools styles
```

### Options

| Option | Description |
|--------|-------------|
| `-o, --output <file>` | Output file (default: stdout) |
| `--no-bib` | Don't include bibliography at the end |
| `--bib-header <text>` | Custom bibliography header (default: `## References`) |

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 2 | Usage error (invalid args, handled by clap) |
| 10 | Input file not found / unreadable |
| 11 | Bibliography file not found / invalid |
| 12 | CSL style not found / invalid |
| 13 | Cited reference not found in bibliography |
| 14 | CSL processing engine error |
| 15 | Output file write error |

Each error includes a contextual hint on stderr to guide the user.

## Citation Syntax

### Basic citations

| Syntax | Description |
|--------|-------------|
| `[@key]` | Simple citation |
| `[@key](url)` | Citation with clickable DOI link (link removed in output) |
| `[@key, p. 42]` | Citation with locator |

### Grouped citations

Adjacent citations are automatically grouped into a single CSL cluster:

| Syntax | Output (numeric style) | Description |
|--------|------------------------|-------------|
| `[@a] [@b] [@c]` | (1-3) | Space-separated |
| `[@a][@b][@c]` | (1-3) | Adjacent |
| `[@a](url) [@b](url)` | (1-2) | With URLs (links ignored) |
| `[@a; @b; @c]` | (1-3) | Pandoc syntax |

**Note:** Citations separated by text or punctuation are NOT grouped:
- `[@a], [@b]` → separate citations (comma is intentional separator)
- `[@a] and [@b]` → separate citations (text between)

### Working with DOI links

The `[@key](url)` syntax keeps your document navigable during writing:

```markdown
Studies show [@pmid:33024307](https://doi.org/10.1038/s41579-020-00459-7) that...
```

The DOI link is clickable in your editor but removed in the final output.

## Integration with pm-tools

[pm-tools](https://github.com/lescientifik/pm-tools) is a companion CLI suite for searching and fetching PubMed articles. Together, they provide a complete workflow for scientific writing.

### Search, cite, and format in one pipeline

```bash
# Search PubMed and generate bibliography
pm-search "CRISPR gene therapy" | pm-fetch | pm-cite > refs.jsonl

# Format your article
csl-tools process article.md --bib refs.jsonl --csl nature.csl -o article.html
```

### Build a bibliography from PMIDs

```bash
# Generate CSL-JSON for specific articles
pm-cite 33024307 29355051 38461394 > refs.jsonl

# Use in your document with [@pmid:33024307] syntax
csl-tools process article.md --bib refs.jsonl --csl apa.csl -o output.html
```

### Interactive article selection with fzf

```bash
# Search, preview, select articles interactively, then generate citations
pm-search "PET imaging biomarkers" | pm-fetch | pm-parse | \
  pm-show --fzf | pm-cite > refs.jsonl
```

### Complete research workflow

```bash
# 1. Search and save parsed results for offline use
pm-search "immunotherapy melanoma" | pm-fetch | pm-parse > articles.jsonl

# 2. Filter by year and journal
pm-filter --year 2023-2025 --journal "Nature" < articles.jsonl > filtered.jsonl

# 3. Generate citations for selected articles
jq -r '.pmid' filtered.jsonl | pm-cite > refs.jsonl

# 4. Format your manuscript
csl-tools process manuscript.md --bib refs.jsonl --csl cell.csl -o manuscript.html
```

## PubMed Workflow (without pm-tools)

If you prefer using the PubMed API directly:

### 1. Fetch references from PubMed API

```bash
curl -sL "https://api.ncbi.nlm.nih.gov/lit/ctxp/v1/pubmed/?format=csl&id=33024307,29355051" > refs.json
```

### 2. Write your article with citation links

```markdown
The results show [@pmid:33024307](https://doi.org/10.1038/s41579-020-00459-7) that...
```

### 3. Generate the final document

```bash
csl-tools process article.md --bib refs.json --csl apa.csl -o article.html
```

## Where to Find CSL Styles

- [Zotero Style Repository](https://www.zotero.org/styles) - 10,000+ styles
- Download directly: `curl -sLO https://www.zotero.org/styles/apa && mv apa apa.csl`

## Reference Formats

| Format | Supported | Notes |
|--------|-----------|-------|
| CSL-JSON | Yes | Native format (PubMed API, Zotero export) |
| JSONL | Yes | One JSON object per line |
| BibTeX | No | Convert with `pandoc --bib2json` |

## Examples

See the `exemples/` directory for complete working examples:

- `exemples/crispr-gene-editing-therapy/` - CRISPR article with APA and Radiology styles
- `exemples/fes_pet/` - FES-PET article with JNM (Journal of Nuclear Medicine) style

## Comparison with Pandoc

| Aspect | csl-tools | Pandoc |
|--------|-----------|--------|
| Binary size | ~5 MB | ~100 MB |
| Dependencies | None (static binary) | Haskell runtime |
| Input formats | Markdown | 40+ formats |
| Output formats | HTML, Markdown | 60+ formats |
| Focus | Citations only | Universal conversion |

csl-tools is intentionally minimalist: it does one thing well (citations) rather than everything.

## License

MIT - see [LICENSE](LICENSE)
