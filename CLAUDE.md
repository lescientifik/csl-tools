# csl-tools

CLI pour formater des citations et bibliographies dans des documents Markdown, utilisant la bibliothèque `csl_proc`.

## Objectif

Transformer un document Markdown contenant des clés de citation en document final avec citations formatées et bibliographie:

```
article.md + refs.json + style.csl → article_final.md/html
```

Voir `overview.md` pour la spécification complète.

## Structure du projet

```
csl-tools/
├── src/
│   ├── main.rs           # CLI (clap)
│   ├── markdown.rs       # Parser citations Markdown
│   ├── processor.rs      # Orchestration
│   └── output.rs         # Génération HTML/Markdown
├── Cargo.toml
├── CLAUDE.md             # Ce fichier
└── overview.md           # Spécification détaillée
```

## Dépendances clés

- `csl_proc` - Moteur CSL (notre bibliothèque)
- `clap` - Parsing des arguments CLI
- `regex` - Extraction des citations du Markdown

## Commandes

```bash
# Build
cargo build

# Run
cargo run -- process article.md --bib refs.json --csl style.csl -o output.html

# Tests
cargo test
```

## Workflow de développement

1. Lire `overview.md` pour comprendre les fonctionnalités à implémenter
2. Implémenter par phases (voir "Phases de développement" dans overview.md)
3. Tester avec des fichiers Markdown réels
4. Commit après chaque fonctionnalité

## Phase 1 - MVP (terminee)

- [x] Parser syntaxe `[@cle]` et `[@cle](url)` dans le Markdown
- [x] Charger references CSL-JSON
- [x] Charger style CSL
- [x] Appeler `csl_proc::process()` pour formater
- [x] Remplacer citations dans le texte
- [x] Ajouter bibliographie a la fin
- [x] Sortie HTML et Markdown
- [x] Groupement automatique des citations adjacentes
- [x] Support syntaxe Pandoc `[@a; @b; @c]`

## Syntaxe des citations

### Citations simples

```markdown
[@pmid:33024307](https://doi.org/10.1038/s41579-020-00459-7)  # avec lien DOI
[@pmid:33024307]                                              # simple
[@pmid:33024307, p. 42]                                       # avec locator
```

### Citations groupees

```markdown
[@a] [@b] [@c]                    # adjacentes avec espace → (1-3)
[@a][@b][@c]                      # collees → (1-3)
[@a](url) [@b](url) [@c](url)     # avec URLs → (1-3), URLs ignorees
[@a; @b; @c]                      # syntaxe Pandoc → (1-3)
```

Les citations adjacentes (separees uniquement par des espaces) sont automatiquement groupees en un seul cluster CSL.

## Notes

- `csl_proc` est en dépendance locale (`path = "../csl_proc"`)
- Passer à `git = "https://github.com/lescientifik/csl_proc.git"` avant publication
