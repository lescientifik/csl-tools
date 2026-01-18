# Exemple: CRISPR Gene Editing Therapy

Démonstration de la pipeline complète `pm-tools` → `csl-tools`.

## Fichiers

| Fichier | Description |
|---------|-------------|
| `article.md` | Article original avec citations `[@pmid:...]` |
| `refs.jsonl` | Références CSL-JSON (via `pm-cite`) |
| `apa.csl` | Style de citation APA 7th edition |
| `output.md` | Article final avec citations formatées et bibliographie |

## Reproduction

```bash
# 1. Recherche PubMed
pm-search --max 5 "CRISPR gene editing therapy 2024"

# 2. Récupération des citations
echo -e "41524770\n41524478\n41481737\n41476860\n41465342" | pm-cite > refs.jsonl

# 3. Traitement avec csl-tools
csl-tools process article.md --bib refs.jsonl --csl apa.csl -o output.md
```

## Résultat

Les citations sont transformées:

- `[@pmid:41524770]` → `(Basit, … Zheng, 2026)`
- `[@pmid:41524478]` → `(Kim, … Joo, 2026)`
- etc.

Et une bibliographie formatée APA est ajoutée à la fin.
