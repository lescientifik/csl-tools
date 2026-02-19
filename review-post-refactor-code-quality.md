# Revue de code post-refactoring -- `csl-tools`

Fichiers examines :
- `/home/user/csl-tools/src/main.rs`
- `/home/user/csl-tools/src/style.rs`
- `/home/user/csl-tools/src/lib.rs`
- `/home/user/csl-tools/src/processor.rs`
- `/home/user/csl-tools/src/refs.rs`
- `/home/user/csl-tools/src/output.rs`
- `/home/user/csl-tools/src/markdown.rs`

---

## 1. Risques de bugs

### 1.1 Detection de `ReferenceNotFound` par correspondance de chaine (MAJEUR)

**Fichier** : `src/main.rs`, lignes 234-239 et 251-256.

```rust
let msg = e.to_string();
if msg.contains("Reference not found") {
    AppError::ReferenceNotFound(msg)
} else {
    AppError::CslProcessing(msg)
}
```

Cette approche repose sur le fait que le message `Display` de
`ProcessorError::ReferenceNotFound` contient la sous-chaine
`"Reference not found"`. Actuellement c'est le cas (`#[error("Reference not found: {0}")]`
dans `src/processor.rs`, ligne 17). Mais c'est **fragile** :

- Modifier le message d'erreur dans `processor.rs` (par exemple le traduire, le
  reformuler) casserait silencieusement le mapping, et l'erreur serait alors
  classee comme `CslProcessing` (exit code 14) au lieu de `ReferenceNotFound`
  (exit code 13).
- Aucun test ne verifie ce couplage.

**Correction recommandee** : faire un `match` typologique sur `ProcessorError`
au lieu de passer par `to_string()`. Le type `ProcessorError` est public ;
autant l'utiliser directement :

```rust
// Dans process_command(), etape 5 :
let processed = format_citations_clusters(&clusters, &refs_json, &style_csl).map_err(|e| {
    match e {
        processor::ProcessorError::ReferenceNotFound(ref id) => {
            AppError::ReferenceNotFound(e.to_string())
        }
        _ => AppError::CslProcessing(e.to_string()),
    }
})?;
```

Meme chose pour l'appel a `format_bibliography()` (lignes 250-257). Cela
elimine le couplage avec le texte du message.

**Impact** : si jamais le texte de `ProcessorError` change, le mapping de code
de sortie est incorrectement silencieux. C'est le bug le plus probable de cette
base de code.

### 1.2 `-o -` n'est pas traite comme stdout

**Fichier** : `src/main.rs`, lignes 269-284.

Le flag `input` accepte `-` pour stdin (ligne 196), ce qui est une convention
Unix classique. Cependant, pour `--output` / `-o`, la valeur `-` n'est **pas**
traitee comme stdout : elle serait interpretee comme un chemin de fichier
litteral nomme `-`.

Ce n'est pas un *bug* a proprement parler (la valeur par defaut, `None`,
ecrit deja vers stdout), mais c'est une **inconsistance** par rapport a
l'idiome Unix. Un utilisateur qui ecrit `csl-tools process ... -o -`
s'attendrait a un comportement stdout, pas a la creation d'un fichier `-`.

**Correction recommandee** :

```rust
// Ligne 269
let effective_output = output.filter(|p| p != Path::new("-"));
if let Some(output_path) = effective_output {
    // ... ecrire dans le fichier ...
} else {
    // ... ecrire vers stdout ...
}
```

### 1.3 Lecture stdin sur entree binaire ou tres grande

**Fichier** : `src/main.rs`, lignes 197-201.

`io::stdin().read_to_string(&mut buf)` lit l'integralite de stdin en memoire.
Si un utilisateur pipe accidentellement un fichier binaire, `read_to_string`
renverra une erreur `InvalidData` (car du contenu non-UTF-8 sera rencontre) --
ce qui est le comportement correct. Pas de bug ici.

Pour les entrees tres volumineuses, il n'y a pas de limite de taille. C'est
acceptable pour un outil CLI de ce type, mais une note dans l'aide utilisateur
serait bienvenue.

**Verdict** : risque faible, comportement acceptable.

### 1.4 `--csl` pointant vers un repertoire

**Fichier** : `src/main.rs`, lignes 213-227.

Si `--csl` pointe vers un repertoire, `load_style()` (qui appelle
`fs::read_to_string`) echouera avec une erreur du type "Is a directory" (sur
Linux) ou "Access denied" (sur Windows). Le `map_err` a la ligne 218 teste
`style_path.exists()`, qui retournera `true` pour un repertoire, et le message
sera donc :

> `invalid CSL style '/tmp/somedir': Is a directory (os error 21)`

C'est un message raisonnablement clair. **Pas de bug**, mais on pourrait
ameliorer le message en testant `style_path.is_file()` au lieu de
`style_path.exists()` :

```rust
if style_path.is_file() {
    AppError::Style(format!("invalid CSL style '{}': {}", csl, e))
} else if style_path.is_dir() {
    AppError::Style(format!("'{}' is a directory, not a CSL style file", csl))
} else {
    AppError::Style(format!(
        "'{}' is not a builtin style name and no file with this path exists",
        csl
    ))
}
```

---

## 2. Idiomes Rust et conception de `AppError`

### 2.1 `AppError` n'implemente pas `std::error::Error`

**Fichier** : `src/main.rs`, lignes 79-147.

L'enum `AppError` implemente `Display` manuellement mais pas `std::error::Error`.
Ce n'est **pas bloquant** car `AppError` n'est jamais utilise comme
`Box<dyn Error>` ni dans un contexte generique -- il est strictement interne a
`main.rs`. Toutefois, par souci de completude et de bonne pratique Rust :

- L'ajout de `impl std::error::Error for AppError {}` (corps vide) couterait
  une ligne et rendrait le type plus idiomatique.
- Alternativement, on pourrait utiliser `thiserror` comme dans `ProcessorError`
  et `RefsError` pour la coherence.

**Recommandation** : ajouter `#[derive(thiserror::Error)]` pour la coherence
avec le reste de la codebase qui utilise deja `thiserror`. Cela donnerait
aussi le `Display` gratuitement :

```rust
#[derive(thiserror::Error, Debug)]
enum AppError {
    #[error("{0}\n  hint: verify the file path is correct")]
    InputFile(String),
    // ...
}
```

Cela dit, le `Display` actuel pour `AppError::Style` appelle
`builtin_style_names()` a chaque formatage, ce qui est un calcul dynamique
(meme s'il est trivial). C'est un argument pour garder le `Display` manuel --
`thiserror` ne supporte pas facilement les messages avec appels de fonction
inline. Le choix actuel est donc **defendable**.

### 2.2 `AppError` n'implemente pas `Debug`

C'est volontaire (il n'est pas `#[derive(Debug)]`). Puisque `main()` n'utilise
que `Display` pour l'affichage utilisateur, c'est correct. Si un jour on
voulait utiliser `?` dans un contexte `Result<(), Box<dyn Error>>`, il faudrait
ajouter `Debug`. Mais l'architecture actuelle avec `exit_code()` rend
`Box<dyn Error>` inutilisable de toute facon.

**Verdict** : conception valide pour le cas d'usage.

### 2.3 Code duplique pour le mapping d'erreurs

**Fichier** : `src/main.rs`, lignes 233-240 et 250-257.

Le meme bloc de mapping d'erreur `ProcessorError -> AppError` est duplique
pour `format_citations_clusters` et `format_bibliography`. Cela devrait etre
factorise dans une fonction :

```rust
fn map_processor_error(e: ProcessorError) -> AppError {
    match e {
        ProcessorError::ReferenceNotFound(_) => AppError::ReferenceNotFound(e.to_string()),
        _ => AppError::CslProcessing(e.to_string()),
    }
}
```

Puis :

```rust
let processed = format_citations_clusters(&clusters, &refs_json, &style_csl)
    .map_err(map_processor_error)?;
```

Cela resout aussi le probleme 1.1 ci-dessus en centralisant le mapping.

---

## 3. Coherence entre `builtin_style()` et `builtin_style_names()`

**Fichier** : `src/style.rs`, lignes 46-56.

```rust
pub fn builtin_style(name: &str) -> Option<&'static str> {
    match name {
        "minimal" => Some(MINIMAL_STYLE),
        _ => None,
    }
}

pub fn builtin_style_names() -> &'static [&'static str] {
    &["minimal"]
}
```

Ces deux fonctions sont **manuellement synchronisees**. Rien ne garantit au
moment de la compilation qu'un nouveau style ajoute dans `builtin_style()` sera
aussi present dans `builtin_style_names()`, ou vice versa. Le risque est qu'un
developpeur ajoute par exemple `"ieee"` dans le `match` mais oublie de le
rajouter dans la liste.

**Correction recommandee** : utiliser une structure de donnees unique comme
source de verite :

```rust
/// (nom, contenu CSL)
const BUILTIN_STYLES: &[(&str, &str)] = &[
    ("minimal", MINIMAL_STYLE),
];

pub fn builtin_style(name: &str) -> Option<&'static str> {
    BUILTIN_STYLES.iter()
        .find(|(n, _)| *n == name)
        .map(|(_, content)| *content)
}

pub fn builtin_style_names() -> Vec<&'static str> {
    BUILTIN_STYLES.iter().map(|(n, _)| *n).collect()
}
```

Le type de retour de `builtin_style_names()` change de `&'static [&'static str]`
a `Vec<&'static str>`, ce qui necessite une petite adaptation dans `main.rs`
(le `.join(", ")` sur la ligne 121 fonctionnerait toujours). C'est un tres bon
compromis pour garantir la coherence.

Alternativement, on peut garder `&'static [&'static str]` en retournant
une reference vers une const derivee, mais en pratique le `Vec` est plus simple
et le nombre de styles est minuscule.

On peut aussi ajouter un test unitaire en attendant le refactoring :

```rust
#[test]
fn test_builtin_style_names_in_sync() {
    for name in builtin_style_names() {
        assert!(
            builtin_style(name).is_some(),
            "builtin_style_names() lists '{}' but builtin_style() returns None",
            name
        );
    }
}
```

---

## 4. Cas limites supplementaires

### 4.1 Stdin vide (entree vide)

Si l'utilisateur tape `echo '' | csl-tools process - --bib refs.json --csl minimal`,
`read_to_string` reussit avec un `buf` vide. `extract_citation_clusters("")`
retourne un vecteur vide, `format_citations_clusters` retourne `Ok(vec![])`,
et la sortie sera une chaine vide. C'est un comportement correct et coherent.

### 4.2 Document sans aucune citation

Meme situation : le document est recopie tel quel vers la sortie, sans
bibliographie. Le message `eprintln!("processed 0 citation(s), wrote ...")`
est emis si `-o` est utilise. Correct.

### 4.3 Double appel a `extract_citations`

**Fichier** : `src/main.rs`, lignes 230 et 246.

Le document Markdown est parse **deux fois** : une fois par
`extract_citation_clusters()` (ligne 230) et une fois par
`extract_citations()` (ligne 246, pour la bibliographie). La premiere fonction
appelle deja `extract_citations()` en interne (`src/markdown.rs`, ligne 141).
C'est un gaspillage mineur. On pourrait extraire les citations une seule fois
et les reutiliser. Cependant, pour la taille typique des documents traites, ce
n'est pas un probleme de performance.

**Recommandation** : refactorer pour parser une seule fois quand l'occasion se
presente, mais ce n'est pas urgent.

---

## 5. Code mort et exports inutilises

### 5.1 `StyleError::InvalidStyle` jamais construit

**Fichier** : `src/style.rs`, ligne 16.

Le variant `InvalidStyle(String)` est declare dans `StyleError` mais n'est
jamais instancie nulle part dans le code. Il a probablement ete prevu pour
une future validation du contenu CSL. En l'etat, c'est du code mort.

**Recommandation** : le supprimer ou le marquer `#[allow(dead_code)]` avec un
commentaire expliquant l'intention future. Le compilateur ne le signale pas
comme warning car `StyleError` est public, mais c'est tout de meme du bruit.

### 5.2 `ProcessorError::InvalidStyle` jamais construit

**Fichier** : `src/processor.rs`, ligne 24.

Meme situation : le variant `InvalidStyle(String)` n'est jamais utilise.

### 5.3 `validate_refs()` non exporte et non utilise en dehors des tests

**Fichier** : `src/refs.rs`, ligne 45.

La fonction `pub fn validate_refs()` est publique dans le module mais n'est
**pas re-exportee** dans `src/lib.rs` et n'est utilisee que dans ses propres
tests. C'est du code inutilise du point de vue de l'API. A conserver si une
utilisation est prevue, mais a noter.

### 5.4 `format_citations` (non-cluster) : utilise uniquement dans les tests

**Fichier** : `src/processor.rs`, ligne 47 ; `src/lib.rs`, ligne 20.

La fonction `format_citations()` est exportee depuis `lib.rs` et est re-exportee
publiquement. Elle est utilisee dans les tests d'integration
(`tests/integration.rs`) mais **pas dans `main.rs`** (qui utilise
`format_citations_clusters` a la place). Ce n'est pas strictement du code mort
(elle fait partie de l'API publique et est testee), mais il faut etre conscient
que le chemin de code du CLI ne passe plus par cette fonction.

### 5.5 `CitationItem` exporte mais non utilise dans `main.rs`

**Fichier** : `src/lib.rs`, ligne 16.

`CitationItem` est re-exporte depuis `lib.rs`. Il est utilise dans
`tests/citation_grouping.rs`, donc c'est justifie du point de vue de l'API
publique. Pas de probleme.

---

## 6. Points positifs

- **Structure du `main.rs`** : la separation `main()` -> `run()` ->
  `process_command()` / `styles_command()` est propre et idiomatique. Les codes
  de sortie semantiques (`AppError::exit_code()`) sont une excellente pratique
  pour un outil CLI.

- **Messages d'aide** : les exemples dans `after_help` sont concis et utiles.
  L'aide de la sous-commande `process` documente la syntaxe des citations, ce
  qui est appreciable.

- **Gestion de stdin** : la convention `-` pour stdin est correctement
  implementee pour l'entree. L'utilisation de `read_to_string` avec un
  `map_err` propre est bien faite.

- **`builtin_style_names()`** retourne une slice statique, ce qui evite toute
  allocation. Le type de retour `&'static [&'static str]` est bien choisi
  pour le cas actuel.

- **Pas de warnings** : `cargo clippy` ne signale aucun warning sur le code
  library/binary. Tous les 102 tests passent.

---

## 7. Resume des actions recommandees

| Priorite | Action | Fichier(s) |
|----------|--------|------------|
| Haute | Remplacer `msg.contains("Reference not found")` par un `match` sur `ProcessorError` | `main.rs` |
| Haute | Factoriser le mapping d'erreur `ProcessorError -> AppError` en une fonction dediee | `main.rs` |
| Moyenne | Unifier `builtin_style()` et `builtin_style_names()` via une structure unique | `style.rs` |
| Moyenne | Ajouter un test de coherence entre `builtin_style()` et `builtin_style_names()` | `style.rs` |
| Basse | Traiter `-o -` comme stdout par coherence avec `-` pour stdin | `main.rs` |
| Basse | Tester `style_path.is_file()` au lieu de `.exists()` pour le message d'erreur CSL | `main.rs` |
| Basse | Supprimer les variants `InvalidStyle` non utilises dans `StyleError` et `ProcessorError` | `style.rs`, `processor.rs` |
| Basse | Ajouter `impl std::error::Error for AppError {}` ou `#[derive(thiserror::Error)]` | `main.rs` |
| Basse | Eviter le double parsing des citations (optimization mineure) | `main.rs` |
