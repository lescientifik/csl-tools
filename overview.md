# csl-tool - CLI pour citations académiques

Outil en ligne de commande pour formater des citations et bibliographies dans des documents Markdown.

## Objectif

Transformer un document Markdown contenant des clés de citation en document final avec citations formatées et bibliographie, en utilisant n'importe quel style CSL.

```
article.md + refs.json + style.csl → article_final.md/html
```

## Exemple d'utilisation

### Entrée

**article.md**
```markdown
# Introduction

Les coronavirus sont une famille de virus [@pmid:33024307](https://doi.org/10.1038/s41579-020-00459-7).
Des études récentes montrent que [@pmid:29355051](https://doi.org/10.1177/1534735417753544)
la transmission est principalement aérienne.

Selon [@pmid:33024307](https://doi.org/10.1038/s41579-020-00459-7), le virus
SARS-CoV-2 présente des caractéristiques uniques.

## Conclusion

Blah blah.

## Références
```

**refs.json** (CSL-JSON, directement depuis l'API PubMed)
```json
[
  {
    "id": "pmid:33024307",
    "type": "article-journal",
    "author": [
      {"family": "Hu", "given": "Ben"},
      {"family": "Guo", "given": "Hua"}
    ],
    "title": "Characteristics of SARS-CoV-2 and COVID-19",
    "container-title": "Nature reviews. Microbiology",
    "volume": "19",
    "issue": "3",
    "page": "141-154",
    "issued": {"date-parts": [[2021, 3]]},
    "PMID": "33024307",
    "DOI": "10.1038/s41579-020-00459-7"
  }
]
```

### Commande

```bash
csl-tool process article.md --bib refs.json --csl apa.csl -o article_final.html
```

### Sortie

**article_final.html**
```html
<h1>Introduction</h1>

<p>Les coronavirus sont une famille de virus (Hu et al., 2021).
Des études récentes montrent que (Deng et al., 2018)
la transmission est principalement aérienne.</p>

<p>Selon (Hu et al., 2021), le virus SARS-CoV-2 présente des
caractéristiques uniques.</p>

<h2>Conclusion</h2>

<p>Blah blah.</p>

<h2>Références</h2>

<div class="csl-bib-body">
  <div class="csl-entry">Deng, X., Luo, S., Luo, X., Hu, M., Ma, F., Wang, Y., ... Huang, R. (2018).
    Fraction from <i>Lycium barbarum</i> polysaccharides reduces immunotoxicity...
    <i>Integrative Cancer Therapies</i>, 17(3), 860–866.</div>
  <div class="csl-entry">Hu, B., Guo, H., Zhou, P., &#38; Shi, Z.-L. (2021). Characteristics of
    SARS-CoV-2 and COVID-19. <i>Nature Reviews Microbiology</i>, 19(3), 141–154.</div>
</div>
```

## Commandes

```bash
# Traiter un document Markdown
csl-tool process <input.md> --bib <refs.json> --csl <style.csl> [-o output.html]

# Générer uniquement une bibliographie
csl-tool bibliography --bib <refs.json> --csl <style.csl>

# Formater une citation inline (pour scripts/intégrations)
csl-tool cite --bib <refs.json> --csl <style.csl> --ids "pmid:33024307,pmid:29355051"

# Valider un fichier CSL-JSON
csl-tool validate --bib <refs.json>

# Lister les clés disponibles dans un fichier de références
csl-tool list --bib <refs.json>
```

## Options globales

| Option | Description |
|--------|-------------|
| `-o, --output <file>` | Fichier de sortie (défaut: stdout) |
| `--format <fmt>` | Format de sortie: `html` (défaut), `markdown` |
| `--locale <code>` | Locale pour les termes: `en-US`, `fr-FR`, etc. |
| `--no-bib` | Ne pas ajouter la bibliographie à la fin |
| `--bib-header <text>` | Titre de la section bibliographie (défaut: "References") |

## Syntaxe des citations

### Format de base avec lien DOI (recommande)

```markdown
[@pmid:33024307](https://doi.org/10.1038/s41579-020-00459-7)
```

Le lien DOI est **cliquable pendant la redaction** (pour consulter l'article) et **supprime au post-processing** (seule la citation formatee reste).

### Syntaxes supportees

| Syntaxe | Rendu (APA) | Description |
|---------|-------------|-------------|
| `[@pmid:33024307](url)` | (Hu et al., 2021) | Citation avec lien DOI |
| `[@pmid:33024307]` | (Hu et al., 2021) | Citation simple (sans lien) |
| `[@pmid:33024307, p. 42](url)` | (Hu et al., 2021, p. 42) | Avec locator |

### Groupement automatique des citations

Les citations adjacentes (separees uniquement par des espaces ou collees) sont **automatiquement groupees** en un seul cluster CSL. Cela permet d'obtenir un formatage correct avec les styles numeriques (ex: `(1-3)` au lieu de `(1) (2) (3)`).

#### Syntaxes de groupement

| Syntaxe | Rendu (numerique) | Description |
|---------|-------------------|-------------|
| `[@a] [@b] [@c]` | (1-3) | Citations adjacentes avec espaces |
| `[@a](url) [@b](url) [@c](url)` | (1-3) | Avec URLs (document navigable) |
| `[@a][@b][@c]` | (1-3) | Citations collees |
| `[@a; @b; @c]` | (1-3) | Syntaxe Pandoc standard |

#### Comportement du groupement

- **Citations adjacentes = groupees**: Separees uniquement par des espaces ou collees
  - `[@a] [@b]` → groupe
  - `[@a][@b]` → groupe
  - `[@a](url) [@b](url)` → groupe (URLs ignorees au rendu)

- **Citations separees par du texte ou ponctuation = PAS groupees**:
  - `[@a], [@b]` → pas groupe (virgule = separateur intentionnel)
  - `[@a] and [@b]` → pas groupe (texte entre)
  - `[@a]. [@b]` → pas groupe (point = nouvelle phrase)

#### Exemples de rendu

```markdown
# Entree
Several studies [@pmid:1] [@pmid:2] [@pmid:3] show that...

# Sortie (style numerique avec collapse)
Several studies (1-3) show that...

# Sortie (style auteur-date)
Several studies (Smith, 2020; Jones, 2021; Brown, 2022) show that...
```

Avec des numeros non-consecutifs:
```markdown
# Si ref1=1, ref3=3, ref4=4, ref6=6
[@ref1] [@ref3] [@ref4] [@ref6] → (1, 3-4, 6)
```

#### Avantage du document de travail navigable

La syntaxe avec URLs permet de garder un document navigable pendant la redaction:

```markdown
[@pmid:33024307](https://doi.org/10.1038/s41579-020-00459-7) [@pmid:29355051](https://doi.org/10.1177/1534735417753544)
```

Les liens sont cliquables dans l'editeur, mais ignores au rendu final qui produit simplement `(1-2)`.

### Parsing

La regex pour extraire la cle:

```rust
// Capture [@cle] avec ou sans lien (url)
let re = Regex::new(r"\[@([^\]]+)\](?:\([^)]+\))?").unwrap();
// Groupe 1: "pmid:33024307" ou "pmid:33024307, p. 42"
```

Le lien `(url)` est optionnel et ignore au processing.

## Architecture

```
csl-tool/
├── src/
│   ├── main.rs           # CLI (clap)
│   ├── markdown.rs       # Parser citations Markdown
│   ├── processor.rs      # Orchestration
│   └── output.rs         # Génération HTML/Markdown
├── Cargo.toml
└── README.md
```

## Dépendances

```toml
[dependencies]
csl_proc = { path = "../csl_proc" }  # Notre moteur CSL
clap = { version = "4", features = ["derive"] }  # CLI
regex = "1"                           # Parser citations
pulldown-cmark = "0.9"               # Parser Markdown (optionnel)
```

## Phases de developpement

### Phase 1 - MVP (terminee)
- [x] Parser syntaxe `[@cle]` basique
- [x] Parser syntaxe `[@cle](url)` avec lien DOI
- [x] Charger refs CSL-JSON
- [x] Charger style CSL
- [x] Appeler csl_proc pour formater
- [x] Remplacer citations dans le texte
- [x] Ajouter bibliographie a la fin
- [x] Sortie HTML et Markdown
- [x] Groupement automatique des citations adjacentes
- [x] Support syntaxe Pandoc `[@a; @b; @c]`

### Phase 2 - Fonctionnalites completes
- [ ] Syntaxe complete (locators, suppress-author)
- [ ] Citations narratives (`@cle` sans crochets)
- [ ] Option `--no-bib`
- [ ] Locales personnalisees

### Phase 3 - Polish
- [ ] Messages d'erreur clairs (cle non trouvee, etc.)
- [ ] Commande `validate`
- [ ] Commande `list`
- [ ] Watch mode (`--watch`)
- [ ] Integration CI (GitHub Actions pour builds cross-platform)

## Différences avec Pandoc

| Aspect | csl-tool | Pandoc |
|--------|----------|--------|
| Taille | ~5 MB | ~100 MB |
| Dépendances | Aucune (binaire statique) | Haskell runtime |
| Formats entrée | Markdown | 40+ formats |
| Formats sortie | HTML, Markdown | 60+ formats |
| Focus | Citations uniquement | Conversion universelle |

csl-tool est volontairement minimaliste: il fait une chose bien (les citations) plutôt que tout faire.

## Exemple d'intégration

### Script de build

```bash
#!/bin/bash
csl-tool process paper.md --bib refs.json --csl ieee.csl -o paper.html
```

### Makefile

```makefile
paper.html: paper.md refs.json
	csl-tool process $< --bib refs.json --csl apa.csl -o $@
```

### GitHub Action

```yaml
- name: Build paper
  run: |
    csl-tool process paper.md --bib refs.json --csl apa.csl -o paper.html
```

## Workflow avec PubMed

### 1. Récupérer les références

L'API PubMed renvoie directement du CSL-JSON:

```bash
curl -sL "https://api.ncbi.nlm.nih.gov/lit/ctxp/v1/pubmed/?format=csl&id=33024307,29355051" > refs.json
```

### 2. Écrire l'article

Dans ton éditeur Markdown, utilise la syntaxe avec lien DOI:

```markdown
Les résultats montrent [@pmid:33024307](https://doi.org/10.1038/s41579-020-00459-7) que...
```

Le lien est cliquable pour consulter l'article pendant la rédaction.

### 3. Générer le document final

```bash
csl-tool process article.md --bib refs.json --csl apa.csl -o article.html
```

### Workflow complet

```
PubMed ──API──► refs.json
                    │
article.md ─────────┼──► csl-tool ──► article.html
                    │         ▲
style.csl ──────────┘         │
                              │
                    Citations formatées +
                    Bibliographie
```

## Formats de références supportés

| Format | Supporté | Notes |
|--------|----------|-------|
| CSL-JSON | ✅ | Format natif (PubMed API, Zotero export) |
| BibTeX | ❌ | Utiliser `pandoc-citeproc --bib2json` pour convertir |
| RIS | ❌ | Idem |

## Où trouver des styles CSL

- [Zotero Style Repository](https://www.zotero.org/styles) - 10,000+ styles
- Télécharger directement: `curl -O https://www.zotero.org/styles/apa`
