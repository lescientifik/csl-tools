---
name: csl-format
description: Format citations and bibliographies in Markdown documents using csl-tools. Automatically handles citation syntax, CSL styles, and generates properly formatted output. Use when a user wants to format academic documents with citations.
---

# CSL Citation Formatting Skill

Format citations and bibliographies in Markdown documents using the csl-tools CLI.

## When to Use

- User wants to format a Markdown document with citations
- User mentions formatting for a specific journal
- User has a document with `[@citation]` syntax
- User wants to generate a bibliography

## Required Files

Before formatting, ensure you have:

1. **Input Markdown file** - Contains citations in one of these syntaxes:
   - `[@citekey]` - Simple citation
   - `[@citekey](url)` - Citation with DOI link (link is preserved for editing, removed in output)
   - `[@a; @b; @c]` - Pandoc-style grouped citations
   - `[@a] [@b] [@c]` - Adjacent citations (auto-grouped)

2. **Bibliography file** (CSL-JSON or JSONL) - Contains reference data
   - Can be obtained from PubMed API, Zotero export, etc.
   - Each reference needs an `id` field matching the citekey

3. **CSL style file** - Defines citation and bibliography format
   - Download from https://www.zotero.org/styles
   - Common styles: apa, ieee, vancouver, nature, cell, etc.

## Workflow

### Step 1: Identify the target journal/style

Ask the user which journal or citation style they want to use if not obvious from context.

Common medical/scientific styles:
- `apa` - American Psychological Association (author-date)
- `vancouver` - Numbered, common in medicine
- `nature` - Nature journal style
- `cell` - Cell journal style
- `ieee` - IEEE transactions
- `jnm` - Journal of Nuclear Medicine
- `radiology` - Radiology journal

### Step 2: Check for required files

Look for in the current directory or ask the user:
- `.md` file with citations
- `.json` or `.jsonl` bibliography file
- `.csl` style file (or download one)

### Step 3: Download CSL style if needed

If the user doesn't have the CSL file:

```bash
curl -sL "https://www.zotero.org/styles/<style-name>" -o <style-name>.csl
```

For example:
```bash
curl -sL "https://www.zotero.org/styles/nature" -o nature.csl
curl -sL "https://www.zotero.org/styles/apa" -o apa.csl
curl -sL "https://www.zotero.org/styles/vancouver" -o vancouver.csl
```

### Step 4: Format the document

Run csl-tools:

```bash
csl-tools process <input.md> --bib <refs.json> --csl <style.csl> -o <output.md>
```

Options:
- `-o <file>` - Output file (default: stdout)
- `--no-bib` - Don't include bibliography at the end
- `--bib-header "## References"` - Custom bibliography header (default: "## References")

### Step 5: Review output

Check that:
- Citations are properly formatted
- Bibliography is complete and correctly ordered
- No missing references (check for warnings)

## Citation Syntax Reference

| Syntax | Example | Description |
|--------|---------|-------------|
| Simple | `[@smith2024]` | Basic citation |
| With URL | `[@smith2024](https://doi.org/...)` | DOI link for editing |
| With locator | `[@smith2024, p. 42]` | Page/section reference |
| Grouped (Pandoc) | `[@a; @b; @c]` | Multiple refs, one cluster |
| Adjacent | `[@a] [@b]` | Auto-grouped by csl-tools |

## Citation Grouping

Adjacent citations (separated only by spaces) are automatically grouped:

- `[@a] [@b] [@c]` becomes `(1-3)` with numeric styles
- `[@a][@b][@c]` (no spaces) also groups
- `[@a], [@b]` does NOT group (comma separates)
- `[@a] and [@b]` does NOT group (text between)

## Troubleshooting

### Missing reference error
- Check that the citekey in the Markdown matches the `id` field in the bibliography
- Citekeys are case-sensitive

### Style not found
- Download the CSL file from Zotero Style Repository
- Use the full path to the .csl file

### Citations not grouping
- Ensure citations are truly adjacent (only spaces between)
- Commas, text, or newlines prevent grouping
