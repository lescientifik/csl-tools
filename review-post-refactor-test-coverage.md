# Revue de couverture des tests post-refactoring

Analyse des lacunes de couverture de tests pour les fonctionnalites ajoutees
lors du refactoring de `main.rs` : `AppError` avec codes de sortie semantiques,
support stdin, sous-commande `styles`, messages d'erreur avec hints, et message
de confirmation sur stderr.

---

## 1. Codes de sortie semantiques (10-15)

### Lacune identifiee

Les tests CLI existants (`tests/cli.rs`) verifient uniquement `output.status.success()`
ou `!output.status.success()`. Aucun test ne verifie la **valeur numerique** du code
de sortie. Par exemple, `test_cli_process_missing_input_file` verifie que la commande
echoue, mais ne distingue pas un code 10 (fichier d'entree) d'un code 11 (fichier bib)
ou d'un code 2 (erreur clap).

Les six variantes de `AppError` produisent des codes distincts :
- 10 : `InputFile`
- 11 : `BibFile`
- 12 : `Style`
- 13 : `ReferenceNotFound`
- 14 : `CslProcessing`
- 15 : `OutputFile`

### Pourquoi c'est important

Les codes de sortie semantiques sont destines a etre consommes par des scripts et des
pipelines CI. Si un refactoring futur casse le mapping entre erreur et code, aucun test
ne le detectera. C'est le contrat public du CLI.

### Tests proposes

```rust
// Dans tests/cli.rs

#[test]
fn test_exit_code_10_input_file_not_found() {
    // Given: un fichier d'entree qui n'existe pas
    let refs_file = create_temp_file(TEST_REFS, ".json");
    let style_file = create_temp_file(TEST_STYLE, ".csl");

    // When: on lance la commande process
    let output = Command::new(binary_path())
        .args([
            "process",
            "/nonexistent/path/article.md",
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            style_file.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    // Then: le code de sortie est 10
    assert_eq!(
        output.status.code(),
        Some(10),
        "Missing input file should exit with code 10, got {:?}. stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_exit_code_11_bib_file_not_found() {
    // Given: un fichier bib qui n'existe pas
    let markdown = "See [@item-1].";
    let md_file = create_temp_file(markdown, ".md");
    let style_file = create_temp_file(TEST_STYLE, ".csl");

    // When: on lance la commande process
    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            "/nonexistent/refs.json",
            "--csl",
            style_file.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    // Then: le code de sortie est 11
    assert_eq!(
        output.status.code(),
        Some(11),
        "Missing bib file should exit with code 11, got {:?}. stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_exit_code_12_style_not_found() {
    // Given: un nom de style qui n'est ni builtin ni un fichier existant
    let markdown = "See [@item-1].";
    let md_file = create_temp_file(markdown, ".md");
    let refs_file = create_temp_file(TEST_REFS, ".json");

    // When: on lance la commande process avec un style inexistant
    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            "nonexistent-style-name",
        ])
        .output()
        .expect("Failed to execute command");

    // Then: le code de sortie est 12
    assert_eq!(
        output.status.code(),
        Some(12),
        "Unknown style should exit with code 12, got {:?}. stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_exit_code_13_reference_not_found() {
    // Given: une citation qui reference une cle absente du fichier bib
    let markdown = "See [@unknown-key].";
    let md_file = create_temp_file(markdown, ".md");
    let refs_file = create_temp_file(TEST_REFS, ".json"); // contient seulement "item-1"
    let style_file = create_temp_file(TEST_STYLE, ".csl");

    // When: on lance la commande process
    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            style_file.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    // Then: le code de sortie est 13
    assert_eq!(
        output.status.code(),
        Some(13),
        "Unknown citation key should exit with code 13, got {:?}. stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_exit_code_15_output_dir_not_writable() {
    // Given: un chemin de sortie dans un repertoire qui n'existe pas
    let markdown = "See [@item-1].";
    let md_file = create_temp_file(markdown, ".md");
    let refs_file = create_temp_file(TEST_REFS, ".json");
    let style_file = create_temp_file(TEST_STYLE, ".csl");

    // When: on lance la commande process avec un repertoire de sortie inexistant
    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            style_file.path().to_str().unwrap(),
            "-o",
            "/nonexistent/dir/output.md",
        ])
        .output()
        .expect("Failed to execute command");

    // Then: le code de sortie est 15
    assert_eq!(
        output.status.code(),
        Some(15),
        "Unwritable output path should exit with code 15, got {:?}. stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}
```

---

## 2. Support stdin (lecture depuis `-`)

### Lacune identifiee

Le fichier `main.rs` supporte `process - --bib ... --csl ...` pour lire le Markdown
depuis stdin (lignes 196-201). Aucun test dans la suite ne pipe du contenu vers le
binaire via stdin.

### Pourquoi c'est important

Le support stdin est documente dans le `--help` et dans le `CLAUDE.md`. C'est une
fonctionnalite essentielle pour l'integration dans des pipelines Unix
(`cat article.md | csl-tools process - ...`). Sans test, une regression passerait
inapercue.

### Test propose

```rust
// Dans tests/cli.rs

use std::process::Stdio;

#[test]
fn test_stdin_support() {
    // Given: du Markdown avec une citation, passe via stdin
    let refs_file = create_temp_file(TEST_REFS, ".json");
    let style_file = create_temp_file(TEST_STYLE, ".csl");

    let markdown_input = "Les resultats montrent [@item-1] que la methode fonctionne.";

    // When: on lance la commande process avec '-' comme fichier d'entree
    let mut child = Command::new(binary_path())
        .args([
            "process",
            "-",
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            style_file.path().to_str().unwrap(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn command");

    // Ecrire le markdown dans stdin
    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin
            .write_all(markdown_input.as_bytes())
            .expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to wait on child");

    // Then: la sortie contient la citation formatee
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "Process from stdin should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("(Doe, 2021)"),
        "Output should contain formatted citation from stdin input: {}",
        stdout
    );
}
```

---

## 3. Sous-commande `styles`

### Lacune identifiee

La sous-commande `styles` (lignes 174-176 de `main.rs`) appelle
`builtin_style_names()` et affiche chaque nom sur stdout. Aucun test CLI ne
l'invoque.

### Pourquoi c'est important

C'est une commande publique du CLI. Si elle est retiree ou cassee accidentellement,
aucun test ne le signalera. De plus, elle valide que la liste des styles builtin est
coherente avec ce que `builtin_style()` accepte.

### Test propose

```rust
// Dans tests/cli.rs

#[test]
fn test_styles_subcommand() {
    // Given: le binaire csl-tools

    // When: on lance la sous-commande styles
    let output = Command::new(binary_path())
        .arg("styles")
        .output()
        .expect("Failed to execute command");

    // Then: la commande reussit et affiche au moins le style "minimal"
    assert!(
        output.status.success(),
        "styles subcommand should succeed. stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("minimal"),
        "styles output should list 'minimal' builtin style, got: {}",
        stdout
    );

    // Chaque ligne non vide devrait etre un nom de style valide
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert!(
        !lines.is_empty(),
        "styles should output at least one style name"
    );
}

#[test]
fn test_styles_subcommand_exit_code_zero() {
    // Given/When: on lance la sous-commande styles
    let output = Command::new(binary_path())
        .arg("styles")
        .output()
        .expect("Failed to execute command");

    // Then: le code de sortie est 0
    assert_eq!(
        output.status.code(),
        Some(0),
        "styles subcommand should exit with code 0"
    );
}
```

---

## 4. Messages d'erreur avec hints

### Lacune identifiee

Chaque variante de `AppError::Display` (lignes 107-147 de `main.rs`) ajoute un
texte `hint:` dans le message d'erreur. Par exemple :
- `InputFile` -> `"hint: verify the file path is correct"`
- `BibFile` -> `"hint: the file must be a JSON array..."`
- `Style` -> `"available builtin styles: ..."` + `"hint: provide a path to a .csl file..."`
- `ReferenceNotFound` -> `"hint: check that this citation key exists..."`
- `OutputFile` -> `"hint: check that the output directory exists..."`

Aucun test existant ne verifie la presence de ces hints dans stderr.

### Pourquoi c'est important

Les hints sont une partie de l'experience utilisateur. Ils guident l'utilisateur
vers la resolution du probleme. Si le texte est accidentellement supprime ou
tronque, l'utilisateur perd cette aide contextuelle. De plus, le hint de `Style`
appelle `builtin_style_names()`, ce qui teste l'integration entre ces deux modules.

### Tests proposes

```rust
// Dans tests/cli.rs

#[test]
fn test_error_hint_input_file() {
    // Given: un fichier d'entree inexistant
    let refs_file = create_temp_file(TEST_REFS, ".json");
    let style_file = create_temp_file(TEST_STYLE, ".csl");

    // When: on lance la commande
    let output = Command::new(binary_path())
        .args([
            "process",
            "/nonexistent/article.md",
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            style_file.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    // Then: stderr contient le hint
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("hint: verify the file path is correct"),
        "stderr should contain input file hint, got: {}",
        stderr
    );
}

#[test]
fn test_error_hint_bib_file() {
    // Given: un fichier bib inexistant
    let markdown = "See [@item-1].";
    let md_file = create_temp_file(markdown, ".md");
    let style_file = create_temp_file(TEST_STYLE, ".csl");

    // When: on lance la commande
    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            "/nonexistent/refs.json",
            "--csl",
            style_file.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    // Then: stderr contient le hint sur le format attendu
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("hint: the file must be a JSON array"),
        "stderr should contain bib file format hint, got: {}",
        stderr
    );
}

#[test]
fn test_error_hint_style_lists_builtin_names() {
    // Given: un nom de style invalide
    let markdown = "See [@item-1].";
    let md_file = create_temp_file(markdown, ".md");
    let refs_file = create_temp_file(TEST_REFS, ".json");

    // When: on lance la commande avec un style inexistant
    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            "totally-fake-style",
        ])
        .output()
        .expect("Failed to execute command");

    // Then: stderr contient la liste des styles builtin ET le hint
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("available builtin styles:"),
        "stderr should list available builtin styles, got: {}",
        stderr
    );
    assert!(
        stderr.contains("minimal"),
        "stderr should mention 'minimal' as available style, got: {}",
        stderr
    );
    assert!(
        stderr.contains("hint: provide a path to a .csl file"),
        "stderr should contain style hint, got: {}",
        stderr
    );
}

#[test]
fn test_error_hint_reference_not_found() {
    // Given: une cle de citation absente de la bibliographie
    let markdown = "See [@nonexistent-key].";
    let md_file = create_temp_file(markdown, ".md");
    let refs_file = create_temp_file(TEST_REFS, ".json");
    let style_file = create_temp_file(TEST_STYLE, ".csl");

    // When: on lance la commande
    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            style_file.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    // Then: stderr contient le hint pour les cles manquantes
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("hint: check that this citation key exists"),
        "stderr should contain reference-not-found hint, got: {}",
        stderr
    );
}

#[test]
fn test_error_hint_output_file() {
    // Given: un chemin de sortie dans un repertoire inexistant
    let markdown = "See [@item-1].";
    let md_file = create_temp_file(markdown, ".md");
    let refs_file = create_temp_file(TEST_REFS, ".json");
    let style_file = create_temp_file(TEST_STYLE, ".csl");

    // When: on lance la commande avec un chemin de sortie invalide
    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            style_file.path().to_str().unwrap(),
            "-o",
            "/nonexistent/dir/output.md",
        ])
        .output()
        .expect("Failed to execute command");

    // Then: stderr contient le hint sur le repertoire
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("hint: check that the output directory exists"),
        "stderr should contain output file hint, got: {}",
        stderr
    );
}
```

---

## 5. Message de confirmation sur stderr

### Lacune identifiee

Quand le traitement reussit avec l'option `-o` (ecriture dans un fichier), le binaire
ecrit sur stderr un message de confirmation :
`"processed N citation(s), wrote <path>"` (ligne 273-277 de `main.rs`).

Le test existant `test_cli_process_output_file` verifie que le fichier de sortie contient
la citation formatee, mais ne verifie **pas** le contenu de stderr.

### Pourquoi c'est important

Ce message de confirmation est la seule indication pour l'utilisateur que le traitement
a reussi quand la sortie va dans un fichier. Il affiche aussi le nombre de citations
traitees, ce qui est une information de diagnostic precieuse.

### Test propose

```rust
// Dans tests/cli.rs

#[test]
fn test_success_confirmation_message_on_stderr() {
    // Given: un fichier Markdown avec une citation et un fichier de sortie
    let markdown = "Les resultats montrent [@item-1].";
    let md_file = create_temp_file(markdown, ".md");
    let refs_file = create_temp_file(TEST_REFS, ".json");
    let style_file = create_temp_file(TEST_STYLE, ".csl");
    let output_file = tempfile::Builder::new().suffix(".md").tempfile().unwrap();

    // When: on lance la commande avec -o
    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            style_file.path().to_str().unwrap(),
            "-o",
            output_file.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    // Then: stderr contient le message de confirmation
    assert!(output.status.success(), "Process should succeed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("processed"),
        "stderr should contain 'processed' confirmation, got: {}",
        stderr
    );
    assert!(
        stderr.contains("citation(s)"),
        "stderr should mention citation count, got: {}",
        stderr
    );
    assert!(
        stderr.contains("wrote"),
        "stderr should mention output file path with 'wrote', got: {}",
        stderr
    );
}

#[test]
fn test_success_confirmation_shows_correct_count() {
    // Given: un fichier Markdown avec trois citations
    let refs = r#"[
        {"id": "a", "type": "book", "author": [{"family": "A", "given": "A."}], "title": "Book A", "issued": {"date-parts": [[2020]]}},
        {"id": "b", "type": "book", "author": [{"family": "B", "given": "B."}], "title": "Book B", "issued": {"date-parts": [[2021]]}},
        {"id": "c", "type": "book", "author": [{"family": "C", "given": "C."}], "title": "Book C", "issued": {"date-parts": [[2022]]}}
    ]"#;
    let markdown = "Voir [@a], puis [@b], et enfin [@c].";
    let md_file = create_temp_file(markdown, ".md");
    let refs_file = create_temp_file(refs, ".json");
    let style_file = create_temp_file(TEST_STYLE, ".csl");
    let output_file = tempfile::Builder::new().suffix(".md").tempfile().unwrap();

    // When: on lance la commande
    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            style_file.path().to_str().unwrap(),
            "-o",
            output_file.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    // Then: stderr indique "processed 3 citation(s)"
    assert!(output.status.success(), "Process should succeed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("processed 3 citation(s)"),
        "stderr should report 3 citations processed, got: {}",
        stderr
    );
}

#[test]
fn test_no_confirmation_message_on_stdout_output() {
    // Given: un fichier Markdown avec une citation, sortie sur stdout (pas de -o)
    let markdown = "Les resultats montrent [@item-1].";
    let md_file = create_temp_file(markdown, ".md");
    let refs_file = create_temp_file(TEST_REFS, ".json");
    let style_file = create_temp_file(TEST_STYLE, ".csl");

    // When: on lance la commande SANS -o (sortie sur stdout)
    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            style_file.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    // Then: stderr ne contient PAS de message "processed"
    // (le message n'est emis que quand on ecrit dans un fichier)
    assert!(output.status.success(), "Process should succeed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("processed"),
        "stderr should NOT contain confirmation when output goes to stdout, got: {}",
        stderr
    );
}
```

---

## 6. Test unitaire de `builtin_style_names()`

### Lacune identifiee

La fonction `builtin_style_names()` dans `src/style.rs` (ligne 54-56) n'a aucun test
unitaire. Les tests existants dans `mod tests` de `style.rs` couvrent `load_style()`
et `builtin_style()`, mais pas `builtin_style_names()`.

### Pourquoi c'est important

Cette fonction est utilisee par `AppError::Style::Display` pour afficher les noms
de styles disponibles dans le message d'erreur. Si un nouveau style builtin est
ajoute a `builtin_style()` mais oublie dans `builtin_style_names()`, le message
d'erreur sera incomplet. Un test qui verifie la coherence entre les deux fonctions
previent cette derive.

### Tests proposes

```rust
// Dans src/style.rs, module tests

#[test]
fn test_builtin_style_names_returns_non_empty_list() {
    // Given/When: on appelle builtin_style_names()
    let names = builtin_style_names();

    // Then: la liste n'est pas vide
    assert!(
        !names.is_empty(),
        "builtin_style_names() should return at least one style"
    );
}

#[test]
fn test_builtin_style_names_contains_minimal() {
    // Given/When: on appelle builtin_style_names()
    let names = builtin_style_names();

    // Then: la liste contient "minimal"
    assert!(
        names.contains(&"minimal"),
        "builtin_style_names() should contain 'minimal', got: {:?}",
        names
    );
}

#[test]
fn test_builtin_style_names_all_resolve() {
    // Given: la liste des noms de styles builtin
    let names = builtin_style_names();

    // When/Then: chaque nom doit etre reconnu par builtin_style()
    for name in names {
        assert!(
            builtin_style(name).is_some(),
            "builtin_style_names() lists '{}' but builtin_style('{}') returns None",
            name,
            name
        );
    }
}

#[test]
fn test_builtin_style_names_is_exhaustive() {
    // Verifie que si builtin_style() reconnait un nom connu,
    // alors il est aussi dans builtin_style_names().
    // Ce test sert de garde-fou quand on ajoute un nouveau style.
    let names = builtin_style_names();
    let known_candidates = ["minimal", "apa", "ieee", "vancouver", "chicago"];

    for candidate in known_candidates {
        if builtin_style(candidate).is_some() {
            assert!(
                names.contains(&candidate),
                "'{}' is recognized by builtin_style() but missing from builtin_style_names()",
                candidate
            );
        }
    }
}
```

---

## 7. Test de `AppError::Display` (messages formates)

### Lacune identifiee

Les implementations `Display` de `AppError` (lignes 107-147 de `main.rs`) sont du code
metier non trivial : elles concatenent le message d'erreur avec des hints et, dans le
cas de `Style`, appellent une fonction externe (`builtin_style_names()`). Aucun test
unitaire ne verifie le formatage de ces messages.

Les tests CLI exercent ces messages indirectement via stderr, mais :
- Ils ne testent pas toutes les variantes (il manque `CslProcessing` et `OutputFile`
  dans l'analyse indirecte).
- Ils sont fragiles car ils passent par un sous-processus.
- Ils ne testent pas le **format exact** du message (presence du newline, indentation
  du hint, etc.).

### Pourquoi c'est important

`AppError` est defini dans `main.rs` qui ne fait pas partie de la library crate, donc
il n'est pas testable via `#[cfg(test)]` dans un module de lib. Cependant, on peut
soit deplacer `AppError` dans la lib, soit ecrire des tests d'integration qui verifient
le format exact des messages via stderr. L'approche la plus pragmatique est de renforcer
les tests CLI existants.

### Tests proposes (via CLI)

```rust
// Dans tests/cli.rs

#[test]
fn test_app_error_display_input_file_format() {
    // Given: un fichier d'entree inexistant
    let refs_file = create_temp_file(TEST_REFS, ".json");
    let style_file = create_temp_file(TEST_STYLE, ".csl");

    // When: on lance la commande
    let output = Command::new(binary_path())
        .args([
            "process",
            "/tmp/nonexistent_csl_tools_test.md",
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            style_file.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    // Then: stderr contient "Error:" suivi du message et du hint sur deux lignes
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.starts_with("Error:"),
        "Error message should start with 'Error:', got: {}",
        stderr
    );
    assert!(
        stderr.contains("/tmp/nonexistent_csl_tools_test.md"),
        "Error message should mention the file path, got: {}",
        stderr
    );
    assert!(
        stderr.contains("\n  hint:"),
        "Hint should be on a new line with 2-space indent, got: {:?}",
        stderr
    );
}

#[test]
fn test_app_error_display_style_format_includes_builtin_list() {
    // Given: un nom de style invalide
    let markdown = "See [@item-1].";
    let md_file = create_temp_file(markdown, ".md");
    let refs_file = create_temp_file(TEST_REFS, ".json");

    // When: on lance la commande avec un style invalide
    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            refs_file.path().to_str().unwrap(),
            "--csl",
            "this-style-does-not-exist",
        ])
        .output()
        .expect("Failed to execute command");

    // Then: stderr contient le nom du style, la liste des builtins, et le hint
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("this-style-does-not-exist"),
        "Error should mention the invalid style name, got: {}",
        stderr
    );
    assert!(
        stderr.contains("is not a builtin style name"),
        "Error should explain the style is not builtin, got: {}",
        stderr
    );
    assert!(
        stderr.contains("available builtin styles: minimal"),
        "Error should list available builtin styles, got: {}",
        stderr
    );
}

#[test]
fn test_app_error_display_bib_file_mentions_jsonl() {
    // Given: un fichier bib inexistant
    let markdown = "See [@item-1].";
    let md_file = create_temp_file(markdown, ".md");
    let style_file = create_temp_file(TEST_STYLE, ".csl");

    // When: on lance la commande
    let output = Command::new(binary_path())
        .args([
            "process",
            md_file.path().to_str().unwrap(),
            "--bib",
            "/nonexistent/refs.json",
            "--csl",
            style_file.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute command");

    // Then: le hint mentionne les deux formats supportes (JSON array et JSONL)
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("JSON array") && stderr.contains("JSONL"),
        "Bib error hint should mention both JSON array and JSONL formats, got: {}",
        stderr
    );
}
```

---

## Resume des lacunes

| Fonctionnalite                    | Tests existants | Couverture |
|-----------------------------------|-----------------|------------|
| Codes de sortie semantiques 10-15 | 0               | Aucune     |
| Support stdin (`-`)               | 0               | Aucune     |
| Sous-commande `styles`            | 0               | Aucune     |
| Messages d'erreur avec hints      | 0               | Aucune     |
| Message de confirmation stderr    | 0               | Aucune     |
| `builtin_style_names()` unitaire  | 0               | Aucune     |
| `AppError::Display` format        | 0               | Aucune     |

**Total : 7 lacunes identifiees, 0 test existant pour ces fonctionnalites.**

Toutes les fonctionnalites ajoutees lors du refactoring sont entierement non testees.
Les tests existants couvrent bien le chemin nominal (traitement reussi) et le
groupement des citations, mais n'exercent aucune des nouvelles couches d'experience
utilisateur (codes de sortie, hints, confirmation, stdin, sous-commande `styles`).

Les 20 tests proposes dans ce document couvrent :
- 5 tests pour les codes de sortie (un par code sauf le 14 qui est difficile a
  declencher isolement)
- 1 test pour stdin
- 2 tests pour la sous-commande `styles`
- 5 tests pour les hints d'erreur
- 3 tests pour le message de confirmation stderr
- 4 tests unitaires pour `builtin_style_names()`
- 3 tests pour le format exact de `AppError::Display`
