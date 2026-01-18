# Plan TDD - csl-tools

## Vue d'ensemble

Développer un CLI en Rust pour formater des citations et bibliographies dans des documents Markdown, utilisant `csl_proc` comme moteur CSL.

## Architecture cible

```
Entrée: article.md + refs.json + style.csl
        ↓
    ┌───────────────────────┐
    │  1. Parser Markdown   │  → Extrait [@clé] et [@clé](url)
    │  2. Charger refs      │  → Parse CSL-JSON
    │  3. Charger style     │  → Lit fichier .csl
    │  4. Formater (csl_proc) │  → Citations + Biblio
    │  5. Remplacer dans MD │  → Substitue citations
    │  6. Générer sortie    │  → HTML ou Markdown
    └───────────────────────┘
        ↓
Sortie: article_final.html
```

## Stratégie de tests

### Décisions
- **Format fixtures**: TOML (plus lisible pour markdown multiligne)
- **Styles CSL**: Minimaux pour unit tests, vrais styles (APA, IEEE) pour intégration
- **Sortie**: Remplacement des citations uniquement, pas de conversion MD→HTML

### Fixtures de test

Chaque test est un fichier TOML dans `tests/fixtures/`:

```toml
# tests/fixtures/basic_citation.toml
name = "Citation simple"
markdown = """
Les résultats montrent [@item-1] que la méthode fonctionne.
"""

refs = """
[{"id": "item-1", "type": "book", "author": [{"family": "Doe", "given": "John"}], "title": "Test Book", "issued": {"date-parts": [[2021]]}}]
"""

# Style minimal inline pour tests unitaires
style = """
<style xmlns="http://purl.org/net/xbiblio/csl" class="in-text" version="1.0">
  <info><id/><title/><updated>2024-01-01T00:00:00+00:00</updated></info>
  <citation><layout prefix="(" suffix=")" delimiter="; ">
    <names variable="author"><name form="short"/></names>
    <text prefix=", " variable="issued" date-parts="year"/>
  </layout></citation>
  <bibliography><layout><names variable="author"/><text variable="title"/></layout></bibliography>
</style>
"""

# Résultat attendu: markdown avec citations remplacées (pas de conversion HTML)
expected = """
Les résultats montrent (Doe, 2021) que la méthode fonctionne.

## References

<div class="csl-bib-body">
  <div class="csl-entry">Doe, J. (2021). <i>Test Book</i>.</div>
</div>
"""
```

### Organisation des fixtures

```
tests/
├── fixtures/
│   ├── parsing/           # Tests du parser markdown
│   │   ├── simple.toml
│   │   ├── with_doi.toml
│   │   └── with_locator.toml
│   ├── integration/       # Tests avec vrais styles
│   │   ├── apa_basic.toml
│   │   └── ieee_numeric.toml
│   ├── output/            # Tests de génération
│   │   ├── no_bib.toml
│   │   └── custom_header.toml
│   └── errors/            # Tests d'erreurs
│       ├── missing_ref.toml
│       └── invalid_json.toml
├── styles/                # Vrais styles CSL téléchargés
│   ├── apa.csl
│   └── ieee.csl
└── integration.rs         # Harness de tests
```

### Catégories de tests à créer

1. **Parsing citations** (10+ tests)
   - `[@id]` simple
   - `[@id](url)` avec lien DOI
   - `[@id, p. 42]` avec locator
   - `[@id, pp. 10-20]` plage de pages
   - `[@id]; [@id2]` multiples séparées
   - `[@id; @id2]` groupées (si supporté)
   - Échappement `\[@id]`
   - Clé avec caractères spéciaux `[@pmid:12345]`

2. **Intégration csl_proc** (5+ tests)
   - Mode citation APA
   - Mode citation IEEE (numérique)
   - Mode bibliographie
   - Plusieurs citations même auteur
   - Disambiguation (Doe 2021a, 2021b)

3. **Sortie** (5+ tests)
   - HTML basique
   - HTML avec classes CSS
   - Markdown (si implémenté)
   - Option `--no-bib`
   - Titre bibliographie personnalisé

4. **Erreurs** (5+ tests)
   - Clé non trouvée
   - Fichier refs invalide
   - Fichier style invalide
   - Markdown malformé

---

## Phases de développement

### Phase 1: Infrastructure de tests (Sub-phase 1.1)

**Objectif**: Créer la structure de tests et fixtures

**Fichiers à créer**:
- `tests/integration.rs` - harness de tests
- `tests/fixtures/` - répertoire des fixtures
- `src/lib.rs` - exposer les modules comme lib

**Tests à écrire** (fixtures TOML):
- 5 fixtures de parsing simples
- 2 fixtures d'intégration basiques
- 2 fixtures d'erreurs

**Critère de succès**: `cargo test` exécute les fixtures (échec attendu car pas encore implémenté)

---

### Phase 2: Parser Markdown (Sub-phases 2.1-2.3)

#### Sub-phase 2.1: Parser basique `[@id]`

**Fichiers**:
- `src/markdown.rs` - module de parsing
- `src/lib.rs` - export

**Fonctions**:
```rust
pub struct Citation {
    pub id: String,
    pub locator: Option<String>,
    pub label: Option<String>,
    pub url: Option<String>,
    pub span: (usize, usize),  // position dans le texte
}

pub fn extract_citations(markdown: &str) -> Vec<Citation>;
```

**Tests TDD**:
- `[@item-1]` → `Citation { id: "item-1", ... }`
- `[@pmid:12345]` → `Citation { id: "pmid:12345", ... }`
- Texte sans citation → `[]`

#### Sub-phase 2.2: Parser avec URL `[@id](url)`

**Ajout**:
- Regex étendue pour capturer `(url)` optionnel
- URL stockée mais ignorée pour le rendu

**Tests TDD**:
- `[@id](https://doi.org/...)` → `Citation { url: Some(...), ... }`
- URL préservée dans la structure

#### Sub-phase 2.3: Parser avec locator `[@id, p. 42]`

**Ajout**:
- Parsing du locator après la virgule
- Extraction du label (p., pp., ch., etc.)

**Tests TDD**:
- `[@id, p. 42]` → `Citation { locator: Some("42"), label: Some("page"), ... }`
- `[@id, pp. 10-20]` → pages multiples
- `[@id, ch. 3]` → chapitre

---

### Phase 3: Chargement données (Sub-phases 3.1-3.2)

#### Sub-phase 3.1: Charger CSL-JSON

**Fichiers**:
- `src/refs.rs` - chargement références

**Note**: `pm-cite` produit du JSONL (une ligne JSON par entrée), pas un tableau JSON.
Il faut supporter les deux formats:
- JSONL: `{"id": "1", ...}\n{"id": "2", ...}`
- JSON array: `[{"id": "1", ...}, {"id": "2", ...}]`

**Fonctions**:
```rust
pub fn load_refs(path: &Path) -> Result<String, Error>;
pub fn validate_refs(json: &str) -> Result<(), Error>;
// Convertir JSONL en tableau JSON si nécessaire
fn normalize_refs(content: &str) -> Result<String, Error>;
```

**Tests TDD**:
- Fichier JSON array valide → JSON string
- Fichier JSONL valide → JSON array string
- Fichier manquant → erreur claire
- JSON invalide → erreur avec ligne

#### Sub-phase 3.2: Charger style CSL

**Fichiers**:
- `src/style.rs` - chargement style

**Fonctions**:
```rust
pub fn load_style(path: &Path) -> Result<String, Error>;
// Styles builtin optionnels
pub fn builtin_style(name: &str) -> Option<&'static str>;
```

**Tests TDD**:
- Fichier .csl → XML string
- `apa` builtin → XML APA

---

### Phase 4: Processeur (Sub-phases 4.1-4.2)

#### Sub-phase 4.1: Intégration csl_proc - Citations

**Fichiers**:
- `src/processor.rs` - orchestration

**Fonctions**:
```rust
pub struct ProcessedCitation {
    pub original_span: (usize, usize),
    pub formatted: String,
}

pub fn format_citations(
    citations: &[Citation],
    refs_json: &str,
    style_csl: &str,
) -> Result<Vec<ProcessedCitation>, Error>;
```

**Tests TDD**:
- Une citation → `(Doe, 2021)`
- Deux citations même auteur → `(Doe, 2021a)`, `(Doe, 2021b)`
- Citation avec locator → `(Doe, 2021, p. 42)`

#### Sub-phase 4.2: Intégration csl_proc - Bibliographie

**Ajout**:
```rust
pub fn format_bibliography(
    citations: &[Citation],
    refs_json: &str,
    style_csl: &str,
) -> Result<String, Error>;
```

**Tests TDD**:
- Une entrée → HTML bib
- Multiples entrées → triées selon style
- Clés dupliquées → une seule entrée bib

---

### Phase 5: Remplacement et sortie (Sub-phases 5.1-5.2)

#### Sub-phase 5.1: Remplacement dans le texte

**Fichiers**:
- `src/output.rs` - génération sortie

**Fonctions**:
```rust
pub fn replace_citations(
    markdown: &str,
    processed: &[ProcessedCitation],
) -> String;
```

**Tests TDD**:
- Remplacement simple → texte avec citation formatée
- Plusieurs citations → toutes remplacées
- Préserve le reste du markdown

#### Sub-phase 5.2: Génération HTML finale

**Ajout**:
```rust
pub fn generate_html(
    content: &str,
    bibliography: Option<&str>,
    bib_header: &str,
) -> String;
```

**Tests TDD**:
- Markdown → HTML (via pulldown-cmark ou basique)
- Avec bibliographie → section ajoutée
- Option `--no-bib` → pas de biblio

---

### Phase 6: CLI (Sub-phase 6.1)

#### Sub-phase 6.1: Interface clap

**Fichiers**:
- `src/main.rs` - CLI complet

**Commandes**:
```rust
#[derive(Parser)]
enum Commands {
    Process { ... },
    Bibliography { ... },
    Cite { ... },
    Validate { ... },
    List { ... },
}
```

**Tests TDD**:
- `process article.md --bib refs.json --csl apa.csl` → sortie HTML
- Arguments manquants → erreur claire
- `--help` → aide formatée

---

## Tests end-to-end à créer (fixtures/)

### Parsing
1. `citation_simple.toml` - `[@id]`
2. `citation_with_doi.toml` - `[@id](url)`
3. `citation_with_locator.toml` - `[@id, p. 42]`
4. `citation_with_range.toml` - `[@id, pp. 10-20]`
5. `citation_pmid_format.toml` - `[@pmid:12345]`
6. `multiple_citations_paragraph.toml` - plusieurs dans un paragraphe
7. `same_citation_repeated.toml` - même citation 2x
8. `no_citations.toml` - texte sans citations

### Intégration
9. `apa_style_basic.toml` - style APA
10. `ieee_style_numeric.toml` - style IEEE numérique
11. `multiple_authors.toml` - et al.
12. `disambiguation_year_suffix.toml` - 2021a, 2021b

### Sortie
13. `html_output_basic.toml` - HTML simple
14. `no_bibliography_option.toml` - `--no-bib`
15. `custom_bib_header.toml` - titre personnalisé

### Erreurs
16. `error_missing_ref.toml` - clé non trouvée
17. `error_invalid_json.toml` - JSON malformé
18. `error_invalid_csl.toml` - CSL invalide

---

## Ressources

### Styles CSL pour tests
- APA 7th: https://www.zotero.org/styles/apa
- IEEE: https://www.zotero.org/styles/ieee
- Vancouver: https://www.zotero.org/styles/vancouver

### API csl_proc
```rust
// Citation simple
csl_proc::process(csl_xml, refs_json, "citation")?;

// Bibliographie
csl_proc::process(csl_xml, refs_json, "bibliography")?;

// Avec citation_items pour ordre/locators
csl_proc::process_with_citations(csl_xml, refs_json, mode, Some(citation_items_json))?;
```

---

## Récapitulatif phases

| Phase | Sub-phase | Description | Fichiers | Dépendances |
|-------|-----------|-------------|----------|-------------|
| 1 | 1.1 | Infrastructure tests | tests/, fixtures/ | - |
| 2 | 2.1 | Parser `[@id]` | markdown.rs | 1.1 |
| 2 | 2.2 | Parser `[@id](url)` | markdown.rs | 2.1 |
| 2 | 2.3 | Parser locators | markdown.rs | 2.2 |
| 3 | 3.1 | Charger CSL-JSON | refs.rs | 1.1 |
| 3 | 3.2 | Charger style | style.rs | 1.1 |
| 4 | 4.1 | csl_proc citations | processor.rs | 2.3, 3.1, 3.2 |
| 4 | 4.2 | csl_proc biblio | processor.rs | 4.1 |
| 5 | 5.1 | Remplacement texte | output.rs | 4.1 |
| 5 | 5.2 | Génération sortie | output.rs | 5.1, 4.2 |
| 6 | 6.1 | CLI clap | main.rs | 5.2 |

Chaque sub-phase est autonome et réalisable en une session avec le skill `tdd-workflow`.

---

## Instructions pour les agents

### Commande à utiliser pour chaque sub-phase

```
/tdd-workflow
```

### Template de prompt pour chaque sub-phase

```
Implémente la sub-phase X.Y du plan dans docs/plan.md.

Contexte:
- Lis docs/plan.md pour comprendre la phase
- Utilise l'approche TDD (Red-Green-Refactor)
- Écris d'abord les tests, vérifie qu'ils échouent
- Implémente le minimum pour faire passer les tests
- Refactore si nécessaire

Fichiers à modifier: [selon la phase]
```

### À faire en premier (Phase 0)

1. Corriger `Cargo.toml`: changer `edition = "2024"` en `edition = "2021"`
2. Copier ce plan vers `docs/plan.md` pour qu'il soit accessible aux agents
3. Ajouter `toml = "0.8"` aux dépendances pour les fixtures de tests

---

## Vérification end-to-end

Après toutes les phases, le test final:

```bash
# Créer un fichier de test
cat > /tmp/test.md << 'EOF'
# Introduction

Les coronavirus [@pmid:33024307](https://doi.org/10.1038/s41579-020-00459-7) sont étudiés.

## References
EOF

# Récupérer les références
pm-cite 33024307 > /tmp/refs.jsonl

# Télécharger un style
curl -sL "https://www.zotero.org/styles/apa" > /tmp/apa.csl

# Exécuter l'outil
cargo run -- process /tmp/test.md --bib /tmp/refs.jsonl --csl /tmp/apa.csl

# Sortie attendue:
# # Introduction
#
# Les coronavirus (Hu et al., 2021) sont étudiés.
#
# ## References
#
# <div class="csl-bib-body">
#   <div class="csl-entry">Hu, B., Guo, H., ...</div>
# </div>
```
