# Exemple: CRISPR Gene Editing Therapy

Démonstration de la pipeline complète `pm-tools` → `csl-tools`.

## Fichiers

| Fichier | Description |
|---------|-------------|
| `article.md` | Article original avec citations `[@pmid:...]` |
| `refs.jsonl` | Références CSL-JSON (via `pm cite`) |
| `apa.csl` | Style de citation APA 7th edition |
| `output.md` | Article final avec citations formatées et bibliographie |

## Reproduction

```bash
# 1. Recherche PubMed
pm search --max 5 "CRISPR gene editing therapy 2024"

# 2. Récupération des citations
pm cite 41524770 41524478 41481737 41476860 41465342 > refs.jsonl

# 3. Traitement avec csl-tools
csl-tools process article.md --bib refs.jsonl --csl apa.csl -o output.md
```

## Résultat

Les citations sont transformées:

- `[@pmid:41524770]` → `(Basit, … Zheng, 2026)`
- `[@pmid:41524478]` → `(Kim, … Joo, 2026)`
- etc.

Et une bibliographie formatée APA est ajoutée à la fin.
