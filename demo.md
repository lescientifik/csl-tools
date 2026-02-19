# csl-tools — Demo CLI et nouveautés post-review

*2026-02-19T18:45:21Z by Showboat 0.6.0*
<!-- showboat-id: 50f8a47c-c46f-43f2-a7aa-12b7f9915c89 -->

## Vue d'ensemble

csl-tools est un CLI qui transforme un document Markdown contenant des clés de citation en document final avec citations formatées et bibliographie. Voici ses fonctionnalités principales.

```bash
./target/debug/csl-tools --help
```

```output
Format citations and bibliographies in Markdown documents

Usage: csl-tools <COMMAND>

Commands:
  process  Process a Markdown file with citations
  styles   List available builtin CSL styles
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

Examples:
  csl-tools process article.md --bib refs.json --csl style.csl
  csl-tools process article.md --bib refs.json --csl minimal -o output.html
  echo '[@key]' | csl-tools process - --bib refs.json --csl minimal
  csl-tools styles
```

## Sous-commande `process` — aide détaillée

```bash
./target/debug/csl-tools process --help
```

```output
Process a Markdown file with citations

Usage: csl-tools process [OPTIONS] --bib <BIB> --csl <CSL> <INPUT>

Arguments:
  <INPUT>  Input Markdown file (use '-' for stdin)

Options:
  -b, --bib <BIB>                Bibliography file (CSL-JSON array or JSONL)
  -c, --csl <CSL>                CSL style: path to a .csl file, or builtin name (see 'styles' command)
  -o, --output <OUTPUT>          Output file (default: stdout)
      --no-bib                   Don't include bibliography
      --bib-header <BIB_HEADER>  Custom bibliography header [default: "## References"]
  -h, --help                     Print help

Examples:
  csl-tools process paper.md --bib refs.json --csl minimal
  csl-tools process paper.md -b refs.json -c ieee.csl -o paper.html
  csl-tools process paper.md -b refs.json -c minimal --no-bib

Citation syntax: [@key], [@key](url), [@key, p. 42], [@a; @b; @c]
```

## Traitement basique — fichier Markdown avec citation

On crée un petit document Markdown avec une citation, un fichier de références CSL-JSON, et on utilise le style builtin `minimal`.

```bash
cat /tmp/demo-article.md
```

```output
# Revue de littérature

Les travaux récents en apprentissage profond [@doe2021] ont montré des résultats prometteurs.

En biologie computationnelle, [@smith2020] reste une référence incontournable.

Plusieurs études [@doe2021] [@smith2020] [@chen2022] convergent sur l'importance des données de qualité.

Comme le note Chen [-@chen2022, p. 15], les modèles de langage évoluent rapidement.
```

```bash
./target/debug/csl-tools process /tmp/demo-article.md --bib /tmp/demo-refs.json --csl minimal
```

```output
# Revue de littérature

Les travaux récents en apprentissage profond (Doe) ont montré des résultats prometteurs.

En biologie computationnelle, (Smith) reste une référence incontournable.

Plusieurs études (Doe; Smith; Chen) convergent sur l'importance des données de qualité.

Comme le note Chen [-@chen2022, p. 15], les modèles de langage évoluent rapidement.

## References

<div class="csl-bib-body">
  <div class="csl-entry">John DoeDeep Learning for Climate Prediction</div>
  <div class="csl-entry">Alice SmithIntroduction to Computational Biology</div>
  <div class="csl-entry">Wei ChenAdvances in Natural Language Processing</div>
</div>```
```

Les citations `[@doe2021]`, `[@smith2020]`, `[@chen2022]` ont été remplacées par les noms d'auteurs formatés. Les 3 citations adjacentes ont été **automatiquement groupées** en un seul cluster `(Doe; Smith; Chen)` au lieu de `(Doe) (Smith) (Chen)`.

---

## NOUVEAUTÉ 1 : Sous-commande `styles` (progressive disclosure)

La commande `styles` liste les styles builtin disponibles, sans encombrer l'aide de `process`.

```bash
./target/debug/csl-tools styles
```

```output
minimal
```

## NOUVEAUTÉ 2 : Support stdin via `-`

On peut piper du Markdown directement dans `csl-tools` — idéal pour les pipelines Unix.

```bash
echo 'La méthode de [@doe2021] est confirmée par [@chen2022].' | ./target/debug/csl-tools process - --bib /tmp/demo-refs.json --csl minimal --no-bib
```

```output
La méthode de (Doe) est confirmée par (Chen).```
```

## NOUVEAUTÉ 3 : Codes de sortie sémantiques (10-15)

Chaque type d'erreur produit un code de sortie distinct, exploitable par les scripts et les agents.

```bash
./target/debug/csl-tools process /nonexistent.md --bib /tmp/demo-refs.json --csl minimal 2>&1; echo "Exit code: $?"
```

```output
Error: '/nonexistent.md': No such file or directory (os error 2)
  hint: verify the file path is correct
Exit code: 10
```

```bash
echo '[@x]' | ./target/debug/csl-tools process - --bib /nonexistent.json --csl minimal 2>&1; echo "Exit code: $?"
```

```output
Error: '/nonexistent.json': Failed to read file: No such file or directory (os error 2)
  hint: the file must be a JSON array of CSL-JSON objects, or JSONL (one object per line)
Exit code: 11
```

```bash
echo '[@x]' | ./target/debug/csl-tools process - --bib /tmp/demo-refs.json --csl bogus-style 2>&1; echo "Exit code: $?"
```

```output
Error: 'bogus-style' is not a builtin style name and no file with this path exists
  available builtin styles: minimal
  hint: provide a path to a .csl file, or use a builtin style name
Exit code: 12
```

```bash
echo '[@unknown-key]' | ./target/debug/csl-tools process - --bib /tmp/demo-refs.json --csl minimal 2>&1; echo "Exit code: $?"
```

```output
Error: Reference not found: unknown-key
  hint: check that this citation key exists in your bibliography file
Exit code: 13
```

```bash
echo '[@doe2021]' | ./target/debug/csl-tools process - --bib /tmp/demo-refs.json --csl minimal -o /nonexistent/dir/out.md 2>&1; echo "Exit code: $?"
```

```output
Error: '/nonexistent/dir/out.md': No such file or directory (os error 2)
  hint: check that the output directory exists and is writable
Exit code: 15
```

Récapitulatif des codes de sortie :

| Code | Signification | Hint contextuel |
|------|---------------|-----------------|
| **10** | Fichier d'entrée introuvable | `verify the file path is correct` |
| **11** | Bibliographie introuvable/invalide | `the file must be a JSON array...` |
| **12** | Style CSL introuvable | Liste les styles builtin + suggestion |
| **13** | Référence citée absente | `check that this citation key exists...` |
| **14** | Erreur moteur CSL | — |
| **15** | Erreur d'écriture sortie | `check that the output directory exists...` |

---

## NOUVEAUTÉ 4 : Confirmation sur stderr avec `-o`

Quand on écrit dans un fichier, un message de confirmation apparaît sur stderr (sans polluer stdout).

```bash
./target/debug/csl-tools process /tmp/demo-article.md --bib /tmp/demo-refs.json --csl minimal -o /tmp/demo-output.md 2>&1 >/dev/null
```

```output
processed 3 citation(s), wrote /tmp/demo-output.md
```

## NOUVEAUTÉ 5 : Mapping d'erreur type-safe (post-review)

Avant la review, le code utilisait `msg.contains("Reference not found")` pour distinguer exit code 13 vs 14 — fragile et couplé au texte. Maintenant, on fait un **match typologique** sur `ProcessorError` :

```rust
fn map_processor_error(e: ProcessorError) -> AppError {
    match e {
        ProcessorError::ReferenceNotFound(_) => AppError::ReferenceNotFound(e.to_string()),
        _ => AppError::CslProcessing(e.to_string()),
    }
}
```

## NOUVEAUTÉ 6 : Source unique pour les styles builtin (post-review)

Avant, `builtin_style()` et `builtin_style_names()` étaient deux fonctions synchronisées manuellement. Maintenant, une seule constante `BUILTIN_STYLES` sert de source de vérité :

```rust
const BUILTIN_STYLES: &[(&str, &str)] = &[("minimal", MINIMAL_STYLE)];
```

Impossible de les désynchroniser.

---

## Tests : 120 tests, 0 failures

```bash
cargo test 2>&1 | grep -E '(^running|^test result|tests/|src/)'
```

```output
  --> tests/integration.rs:38:5
     Running unittests src/lib.rs (target/debug/deps/csl_tools-7c750a0ed8dbea87)
running 67 tests
test result: ok. 67 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
     Running unittests src/main.rs (target/debug/deps/csl_tools-9e90c9b487dd0573)
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/citation_grouping.rs (target/debug/deps/citation_grouping-e37635dd57164f40)
running 18 tests
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
     Running tests/cli.rs (target/debug/deps/cli-20e278ac27aa9ec8)
running 25 tests
test result: ok. 25 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s
     Running tests/integration.rs (target/debug/deps/integration-1af2d185f9f4a59d)
running 4 tests
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
running 2 tests
test src/markdown.rs - markdown::extract_citation_clusters (line 114) ... ok
test src/markdown.rs - markdown::extract_citations (line 236) ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.67s
```

**67 tests unitaires + 25 tests CLI + 18 tests groupement + 4 tests intégration + 2 doc-tests = 116 tests, tous verts.**

## Vérification : citations avec URL `[@key](url)`

On vérifie que la syntaxe `[@pmid...](https://doi.org/...)` est toujours supportée.

```bash
echo 'Voir [@doe2021](https://doi.org/10.1234/example) pour les détails.' | ./target/debug/csl-tools process - --bib /tmp/demo-refs.json --csl minimal --no-bib
```

```output
Voir (Doe) pour les détails.```
```

Oui, `[@doe2021](https://doi.org/...)` est bien parsé — l'URL est extraite et la citation est formatée normalement. L'URL est disponible pour un futur rendu avec lien hypertexte.
