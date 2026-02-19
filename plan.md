# Plan : Refonte agent-friendly de csl-tools

Objectif : rendre `csl-tools` conforme aux guidelines de `cli-design-for-agents.md` en corrigeant les 7 problèmes identifiés par la revue, et en supprimant le système de skills devenu inutile.

---

## Étape 1 — Supprimer le skill-install et tout le système de skills

**Fichiers touchés :** `src/main.rs`, `src/skill.md`, `.claude/skills/csl-format/SKILL.md`

**Actions :**
- Supprimer la variante `SkillInstall` de l'enum `Commands` (lignes 53-58)
- Supprimer le match arm `Commands::SkillInstall` dans `run()` (lignes 89-91)
- Supprimer la constante `SKILL_CONTENT` (ligne 178)
- Supprimer la fonction `skill_install_command()` (lignes 180-218)
- Supprimer le fichier `src/skill.md`
- Supprimer le répertoire `.claude/skills/` (fichier généré lors des tests)

**Justification :** Avec un CLI agent-friendly, un agent peut découvrir toutes les capacités via `--help`. Le skill est redondant et crée de la maintenance inutile.

---

## Étape 2 — Créer un enum `AppError` avec codes de sortie sémantiques

**Fichiers touchés :** `src/main.rs`

**Actions :**
- Créer un enum `AppError` dans `main.rs` avec des variantes typées :

```rust
#[derive(Debug)]
enum AppError {
    InputFileError(String),      // code 10
    BibFileError(String),        // code 11
    StyleError(String),          // code 12
    ReferenceNotFound(String),   // code 13
    CslProcessingError(String),  // code 14
    OutputError(String),         // code 15
}
```

- Implémenter `Display` et `Error` pour `AppError`
- Ajouter une méthode `fn exit_code(&self) -> i32`
- Refactoriser `process_command()` pour retourner `Result<(), AppError>` au lieu de `Result<(), Box<dyn Error>>`
- Modifier `main()` pour mapper l'erreur vers le bon code de sortie :

```rust
fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        process::exit(e.exit_code());
    }
}
```

- Ajouter des hints contextuels dans le `Display` de chaque variante (ex: `hint: check that the file path is correct`)

**Mapping des codes :**

| Code | Signification |
|------|---------------|
| 0    | Succès |
| 2    | Erreur d'usage (clap, inchangé) |
| 10   | Fichier d'entrée introuvable/illisible |
| 11   | Fichier bibliographie introuvable/invalide |
| 12   | Style CSL introuvable/invalide |
| 13   | Référence citée non trouvée |
| 14   | Erreur du moteur CSL |
| 15   | Erreur d'écriture du fichier de sortie |

---

## Étape 3 — Améliorer les messages d'erreur ("errors that teach")

**Fichiers touchés :** `src/main.rs` (dans `process_command()`)

**Actions :**
- **Style introuvable** : quand `builtin_style()` retourne `None` et `load_style()` échoue, lister les styles builtin disponibles :
  ```
  Error: CSL style 'apa' not found
    'apa' is not a builtin style name and no file with this path exists
    available builtin styles: minimal
    hint: provide a path to a .csl file, or use a builtin style name
  ```

- **Référence manquante** : intercepter `ProcessorError::ReferenceNotFound` et enrichir avec les clés disponibles :
  ```
  Error: reference not found: 'unknown-ref'
    hint: check that this citation key exists in your bibliography file
  ```

- **JSON invalide** : rappeler le format attendu :
  ```
  Error: failed to load bibliography 'refs.json': invalid JSON at line 1
    hint: the file must be a JSON array of CSL-JSON objects, or JSONL (one object per line)
  ```

- **Fichier introuvable** (entrée, bib, sortie) : ajouter un hint basique :
  ```
  Error: failed to read input file 'article.md': No such file or directory
    hint: verify the file path is correct
  ```

---

## Étape 4 — Support stdin via `-`

**Fichiers touchés :** `src/main.rs`

**Actions :**
- Modifier la lecture du fichier d'entrée dans `process_command()` pour accepter `-` comme stdin :

```rust
use std::io::Read;

let markdown = if input == Path::new("-") {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)
        .map_err(|e| AppError::InputFileError(format!("failed to read from stdin: {}", e)))?;
    buf
} else {
    fs::read_to_string(input)
        .map_err(|e| AppError::InputFileError(format!("failed to read '{}': {}", input.display(), e)))?
};
```

- Mettre à jour la description de l'argument dans clap : `"Input Markdown file (use '-' for stdin)"`

---

## Étape 5 — Ajouter des exemples dans le texte d'aide

**Fichiers touchés :** `src/main.rs`

**Actions :**
- Ajouter `after_help` au top-level `Cli` :

```rust
#[command(after_help = "\
Examples:
  csl-tools process article.md --bib refs.json --csl style.csl
  csl-tools process article.md --bib refs.json --csl minimal -o output.html
  echo '[@key]' | csl-tools process - --bib refs.json --csl minimal")]
```

- Ajouter `after_help` à la sous-commande `Process` :

```rust
#[command(after_help = "\
Examples:
  csl-tools process paper.md --bib refs.json --csl minimal
  csl-tools process paper.md -b refs.json -c ieee.csl -o paper.html
  csl-tools process paper.md -b refs.json -c minimal --no-bib

Builtin styles: minimal
Citation syntax: [@key], [@key](url), [@key, p. 42], [@a; @b; @c]")]
```

- Enrichir la description de `--csl` : `"CSL style file or builtin name (available: minimal)"`

---

## Étape 6 — Message de confirmation sur stderr après succès avec `-o`

**Fichiers touchés :** `src/main.rs`

**Actions :**
- Après `fs::write()` réussi, ajouter un message sur stderr :

```rust
if let Some(output_path) = output {
    fs::write(output_path, &result).map_err(/* ... */)?;
    let n_citations = processed.len();
    eprintln!("processed {} citation(s), wrote {}", n_citations, output_path.display());
} else {
    // stdout: pas de message supplémentaire
}
```

- Harmoniser la sortie stdout (remplacer `writeln!` par `write!` pour ne pas ajouter un `\n` supplémentaire par rapport au mode fichier)

---

## Étape 7 — Ajouter une sous-commande `styles` pour la progressive disclosure

**Fichiers touchés :** `src/main.rs`, `src/style.rs`

**Actions :**
- Ajouter une fonction `pub fn builtin_style_names() -> &'static [&'static str]` dans `style.rs` retournant `&["minimal"]`
- Ajouter une variante `Styles` dans l'enum `Commands` :

```rust
/// List available builtin CSL styles
Styles,
```

- Implémenter le handler qui liste les styles disponibles sur stdout (un par ligne, pour faciliter le piping)
- Exporter `builtin_style_names` depuis `lib.rs`

---

## Étape 8 — Tests et validation

**Actions :**
- `cargo test` — vérifier que les tests existants passent
- `cargo build` — vérifier la compilation
- Tests manuels des scénarios d'erreur pour vérifier les nouveaux messages et codes de sortie
- Vérifier que `--help` affiche bien les exemples
- Vérifier que `csl-tools styles` fonctionne
- Vérifier que `echo '[@key]' | csl-tools process - --bib ... --csl ...` fonctionne
- Vérifier que `skill-install` n'existe plus

---

## Étape 9 — Mettre à jour CLAUDE.md et overview.md

**Fichiers touchés :** `CLAUDE.md`, `overview.md`

**Actions :**
- Mettre à jour `CLAUDE.md` :
  - Retirer `skill-install` de la structure et des commandes
  - Ajouter `styles` dans les commandes disponibles
  - Documenter le support stdin
  - Documenter les codes de sortie sémantiques

- Mettre à jour `overview.md` :
  - Marquer `skill-install` comme supprimé
  - Marquer `styles` comme implémenté
  - Préciser que `--format` et `--locale` sont Phase 2 (non implémentés)

---

## Résumé des changements par fichier

| Fichier | Étapes | Type de changement |
|---------|--------|--------------------|
| `src/main.rs` | 1,2,3,4,5,6,7 | Refonte majeure |
| `src/style.rs` | 7 | Ajout `builtin_style_names()` |
| `src/lib.rs` | 1,7 | Mise à jour exports |
| `src/skill.md` | 1 | Suppression |
| `.claude/skills/` | 1 | Suppression |
| `CLAUDE.md` | 9 | Mise à jour documentation |
| `overview.md` | 9 | Mise à jour documentation |
