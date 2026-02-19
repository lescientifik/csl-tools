# Revue post-refactoring : conformite CLI

Date : 2026-02-19
Fichiers analyses : `src/main.rs`, `src/style.rs`, `src/output.rs`, `src/refs.rs`, `src/markdown.rs`, `src/processor.rs`, `src/lib.rs`, `Cargo.toml`
Reference : `overview.md` (specification), `CLAUDE.md` (instructions projet)

---

## 1. Codes de sortie

### 1.1 Mapping des codes (conforme)

| Code | Specification | Implementation (`main.rs` L95-104) | Statut |
|------|---------------|-------------------------------------|--------|
| 0 | Succes | Implicite (pas d'erreur) | OK |
| 2 | Erreur d'usage (clap) | Gere automatiquement par clap | OK |
| 10 | Fichier d'entree introuvable | `AppError::InputFile` | OK |
| 11 | Fichier bibliographie invalide | `AppError::BibFile` | OK |
| 12 | Style CSL introuvable/invalide | `AppError::Style` | OK |
| 13 | Reference non trouvee | `AppError::ReferenceNotFound` | OK |
| 14 | Erreur moteur CSL | `AppError::CslProcessing` | OK |
| 15 | Erreur ecriture sortie | `AppError::OutputFile` | OK |

**Severite : aucun probleme** -- Tous les codes de sortie sont correctement mappes.

### 1.2 Discrimination entre codes 13 et 14

**Severite : mineur**
Fichier : `/home/user/csl-tools/src/main.rs`, lignes 233-240 et 250-257

La discrimination entre `ReferenceNotFound` (code 13) et `CslProcessing` (code 14) repose sur une heuristique fragile : `msg.contains("Reference not found")`. Si `csl_proc` change le texte de son message d'erreur (majuscule, traduction, reformulation), la classification tombera par defaut sur le code 14 au lieu du code 13.

```rust
let msg = e.to_string();
if msg.contains("Reference not found") {
    AppError::ReferenceNotFound(msg)
} else {
    AppError::CslProcessing(msg)
}
```

Ce risque est attenue par le fait que la validation des references (`available_ids.contains`) est faite en amont dans `processor.rs` (L74-78 et L169-175), donc la plupart des cas "reference non trouvee" sont interceptes avant d'atteindre `csl_proc`. Neanmoins, un cas residuel pourrait echapper a cette pre-validation si le format JSON des references est valide mais avec des IDs differents de ceux attendus par `csl_proc` apres normalisation.

**Recommandation** : Documenter cette dependance ou, idealement, que `csl_proc` expose des types d'erreur structures plutot que de simples `String`.

---

## 2. Support stdin

### 2.1 Implementation (conforme)

**Fichier : `/home/user/csl-tools/src/main.rs`, lignes 196-206**

Le support de `-` pour stdin est correctement implemente :

```rust
let markdown = if input == Path::new("-") {
    let mut buf = String::new();
    io::stdin()
        .read_to_string(&mut buf)
        .map_err(|e| AppError::InputFile(format!("failed to read from stdin: {}", e)))?;
    buf
} else {
    fs::read_to_string(input).map_err(|e| {
        AppError::InputFile(format!("'{}': {}", input.display(), e))
    })?
};
```

- La comparaison `Path::new("-")` est correcte.
- L'erreur stdin est bien mappee sur le code 10 (`InputFile`).

### 2.2 Documentation dans le help (conforme)

**Fichier : `/home/user/csl-tools/src/main.rs`, ligne 29 et ligne 47**

Le stdin est documente a deux endroits :
- Help global (L29) : `echo '[@key]' | csl-tools process - --bib refs.json --csl minimal`
- Description du parametre `input` (L47) : `"Input Markdown file (use '-' for stdin)"`

**Severite : aucun probleme.**

---

## 3. Sous-commande `styles`

### 3.1 Implementation (conforme, mais limitee)

**Fichier : `/home/user/csl-tools/src/main.rs`, lignes 290-294**

```rust
fn styles_command() {
    for name in builtin_style_names() {
        println!("{}", name);
    }
}
```

**Fichier : `/home/user/csl-tools/src/style.rs`, lignes 54-56**

```rust
pub fn builtin_style_names() -> &'static [&'static str] {
    &["minimal"]
}
```

La commande fonctionne correctement. Cependant, un seul style builtin (`minimal`) est disponible.

### 3.2 Absence de description des styles

**Severite : mineur**

La commande `styles` affiche uniquement les noms des styles, sans description. Pour un seul style c'est acceptable, mais si d'autres styles sont ajoutes (apa, ieee, vancouver, etc.), il serait utile d'afficher une breve description (ex: `minimal - Minimal style for testing`).

### 3.3 Progressive disclosure (conforme)

La sous-commande `styles` est separee de `process`, ce qui respecte le principe de progressive disclosure specifie dans `overview.md` (L101). L'utilisateur n'a pas besoin de connaitre les styles builtin pour utiliser `process` avec un fichier `.csl`.

---

## 4. Gestion de la sortie

### 4.1 Flag `-o` et confirmation stderr (conforme)

**Fichier : `/home/user/csl-tools/src/main.rs`, lignes 269-284**

```rust
if let Some(output_path) = output {
    fs::write(output_path, &result).map_err(|e| {
        AppError::OutputFile(format!("'{}': {}", output_path.display(), e))
    })?;
    eprintln!(
        "processed {} citation(s), wrote {}",
        processed.len(),
        output_path.display()
    );
} else {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    write!(handle, "{}", result).map_err(|e| {
        AppError::OutputFile(format!("stdout: {}", e))
    })?;
}
```

- Avec `-o` : ecriture dans le fichier + confirmation sur stderr. **Conforme.**
- Sans `-o` : ecriture sur stdout, pas de message parasite sur stderr. **Conforme.**
- L'erreur d'ecriture est bien mappee sur le code 15 (`OutputFile`). **Conforme.**

### 4.2 Sortie stdout sans newline final

**Severite : mineur**

La fonction `generate_output()` dans `output.rs` (L58) applique `trim_end()` sur le contenu, puis la sortie est ecrite avec `write!` (pas `writeln!`). Le resultat sur stdout ne se terminera pas necessairement par un newline. Ce n'est pas un bug, mais c'est une pratique inhabituelle pour les outils CLI Unix. Si la sortie est pipee vers un autre outil, cela peut causer des problemes de parsing (derniere ligne sans `\n`).

**Recommandation** : Ajouter un `\n` final lors de l'ecriture sur stdout :
```rust
writeln!(handle, "{}", result)
```

---

## 5. Messages d'erreur avec hints contextuels

### 5.1 Revue des hints (conforme)

**Fichier : `/home/user/csl-tools/src/main.rs`, lignes 108-146**

| Erreur | Hint fourni | Qualite |
|--------|-------------|---------|
| `InputFile` (L111) | `"verify the file path is correct"` | Correct mais generique |
| `BibFile` (L114-117) | `"the file must be a JSON array of CSL-JSON objects, or JSONL"` | Excellent -- guide l'utilisateur sur le format attendu |
| `Style` (L120-126) | Liste les styles builtin + `"provide a path to a .csl file, or use a builtin style name"` | Excellent -- actionnable |
| `ReferenceNotFound` (L128-132) | `"check that this citation key exists in your bibliography file"` | Correct |
| `CslProcessing` (L135-136) | Aucun hint | Voir ci-dessous |
| `OutputFile` (L138-142) | `"check that the output directory exists and is writable"` | Correct |

### 5.2 Pas de hint pour CslProcessing

**Severite : mineur**
**Fichier : `/home/user/csl-tools/src/main.rs`, lignes 135-136**

```rust
AppError::CslProcessing(msg) => {
    write!(f, "{}", msg)
}
```

Cette erreur (code 14) est la seule sans hint contextuel. Comme elle provient du moteur CSL sous-jacent, il est difficile de donner un conseil generique, mais un hint comme `"this may indicate an issue with the CSL style definition"` serait utile.

### 5.3 Prefixe "Error:" sur stderr (conforme)

**Fichier : `/home/user/csl-tools/src/main.rs`, ligne 155**

```rust
eprintln!("Error: {}", e);
```

Tous les messages d'erreur sont prefixes par `Error:` et ecrits sur stderr. **Bonne pratique CLI respectee.**

---

## 6. Texte d'aide

### 6.1 Exemples dans `--help` (conforme)

**Fichier : `/home/user/csl-tools/src/main.rs`, lignes 25-30 et 39-45**

Exemples presents a deux niveaux :
- Help global (`Cli`, L25-30) : 4 exemples couvrant `process` avec fichier, avec `-o`, avec stdin, et `styles`
- Help de `process` (`Commands::Process`, L39-45) : 3 exemples + rappel syntaxe citations

**Conforme a la specification CLAUDE.md** ("Exemples dans `--help`").

### 6.2 Progressive disclosure (conforme)

L'utilisation de sous-commandes (`process`, `styles`) respecte le principe de progressive disclosure. L'utilisateur peut taper `csl-tools --help` pour une vue d'ensemble, puis `csl-tools process --help` pour les details.

### 6.3 Description des arguments (conforme)

Chaque argument possede une description claire :
- `input` (L47) : `"Input Markdown file (use '-' for stdin)"`
- `--bib` (L51) : `"Bibliography file (CSL-JSON array or JSONL)"`
- `--csl` (L54) : `"CSL style: path to a .csl file, or builtin name (see 'styles' command)"`
- `-o` (L59) : `"Output file (default: stdout)"`
- `--no-bib` (L63) : `"Don't include bibliography"`
- `--bib-header` (L67) : `"Custom bibliography header"` avec valeur par defaut `"## References"`

---

## 7. Flag `--format`

### 7.1 Absence du flag `--format` (attendu)

**Severite : information**

Le flag `--format html|markdown` est mentionne dans `overview.md` (L128-129) comme prevu pour la Phase 2 :

> `--format <fmt>` | Format de sortie: `html` (defaut), `markdown`

Il n'est pas implemente dans le code actuel, ce qui est **conforme** au statut Phase 1 du projet. `CLAUDE.md` (L119) confirme :

> `--format` et `--locale` sont prevus pour Phase 2 (non implementes)

Cependant, `CLAUDE.md` (L85) liste egalement "Sortie HTML et Markdown" comme terminee dans la Phase 1. Ceci est trompeur : la generation du contenu supporte probablement les deux formats en interne, mais l'utilisateur n'a actuellement aucun moyen de choisir le format via la CLI.

### 7.2 Format de sortie implicite

**Severite : important**

Actuellement, la sortie est toujours dans le meme format quel que soit le contexte. En examinant `output.rs`, la fonction `generate_output()` ne fait aucune distinction de format -- elle concatene simplement le contenu avec la bibliographie. Le format depend entierement de ce que retourne `csl_proc` (qui produit du HTML pour la bibliographie).

Le resultat est un document hybride : le corps du texte reste en Markdown (les citations sont remplacees par du texte brut), mais la bibliographie est en HTML (classes `csl-bib-body`, `csl-entry`). Cela peut surprendre un utilisateur qui s'attend a un document entierement Markdown ou entierement HTML.

**Recommandation** : Au minimum, documenter ce comportement dans le `--help` de la sous-commande `process`. Idealement, implementer `--format` en Phase 2 comme prevu.

---

## 8. Cas limites

### 8.1 `--bib` manquant

**Severite : aucun probleme (gere par clap)**

Le champ `bib` est declare comme `PathBuf` sans `Option<>` (L51-52), donc clap le traite comme obligatoire. Si l'utilisateur omet `--bib`, clap affichera automatiquement un message d'erreur et sortira avec le code 2.

### 8.2 `--csl` manquant

**Severite : aucun probleme (gere par clap)**

Meme chose : `csl` est un `String` obligatoire (L54-55). Clap gere l'erreur.

### 8.3 Fichier d'entree introuvable

**Severite : aucun probleme**
**Fichier : `/home/user/csl-tools/src/main.rs`, lignes 202-206**

L'appel `fs::read_to_string(input)` echoue avec une erreur `io::Error` qui est mappee sur `AppError::InputFile` (code 10). Le message inclut le chemin du fichier et la description de l'erreur systeme.

### 8.4 Fichier bibliographie introuvable

**Severite : aucun probleme**
**Fichier : `/home/user/csl-tools/src/main.rs`, lignes 209-210**

`load_refs(bib)` appelle `fs::read_to_string(path)` dans `refs.rs` (L40), qui echoue pour un fichier inexistant. L'erreur est mappee sur `AppError::BibFile` (code 11).

### 8.5 Style CSL introuvable -- distinction fichier/builtin

**Severite : aucun probleme**
**Fichier : `/home/user/csl-tools/src/main.rs`, lignes 213-227**

La logique est bien structuree :
1. D'abord, verifier si c'est un nom de style builtin
2. Sinon, tenter de charger comme fichier
3. Si le fichier existe mais est invalide : message specifique
4. Si le fichier n'existe pas : message indiquant que ce n'est ni un builtin ni un fichier existant

### 8.6 Document sans citations

**Severite : aucun probleme**

Si le document Markdown ne contient aucune citation :
- `extract_citation_clusters()` retourne un vecteur vide
- `format_citations_clusters()` retourne `Ok(Vec::new())` immediatement (L148-150 de `processor.rs`)
- `extract_citations()` retourne un vecteur vide
- `format_bibliography()` retourne `Ok(String::new())` immediatement (L248-249 de `processor.rs`)
- Le document original est rendu tel quel, sans bibliographie

### 8.7 Fichier bibliographie vide

**Severite : aucun probleme**

`normalize_refs("")` retourne `"[]"` (L66-68 de `refs.rs`), ce qui est un tableau JSON vide valide. Si le document contient des citations mais que la bibliographie est vide, l'erreur `ReferenceNotFound` sera levee lors du traitement.

### 8.8 Conflit stdin + fichier de sortie identique au fichier d'entree

**Severite : mineur**

Si l'utilisateur fait `csl-tools process article.md --bib refs.json --csl minimal -o article.md`, le fichier d'entree sera lu en memoire d'abord (L203), puis ecrase par la sortie (L270). Cela fonctionne car la lecture est complete avant l'ecriture, mais c'est un comportement potentiellement dangereux. Aucune protection n'est en place.

**Recommandation** : Ajouter un avertissement ou une protection si le fichier de sortie est identique au fichier d'entree.

---

## 9. Autres observations

### 9.1 Absence de flag `--version` explicite

**Severite : aucun probleme**

Le `#[command(version)]` (L24) sur la struct `Cli` active automatiquement `--version` via clap, qui utilise la version du `Cargo.toml` (0.1.0).

### 9.2 Options `--no-bib` et `--bib-header`

**Severite : information**

Ces options sont implementees dans la CLI (L63-68) mais `--no-bib` est marquee comme Phase 2 dans `overview.md` (L255). C'est une avance sur le planning, ce qui est positif. L'implementation est correcte :
- `--no-bib` saute la generation de bibliographie (L247-263 de `main.rs`)
- `--bib-header` a une valeur par defaut `"## References"` conforme a `overview.md` (L122)

### 9.3 Compilation du style CSL (`load_style` ne valide pas)

**Severite : mineur**
**Fichier : `/home/user/csl-tools/src/style.rs`, lignes 32-35**

```rust
pub fn load_style(path: &Path) -> Result<String, StyleError> {
    let content = fs::read_to_string(path)?;
    Ok(content)
}
```

La fonction `load_style` lit le fichier mais ne valide pas que le contenu est du XML CSL valide. L'erreur `StyleError::InvalidStyle` existe dans l'enum (L15) mais n'est jamais construite. La validation ne se fait qu'implicitement lorsque `csl_proc` tente de traiter le style plus tard, ce qui produit une erreur `CslProcessing` (code 14) au lieu de `Style` (code 12).

Cependant, dans `main.rs` (L218-219), il y a une tentative de distinction : si le fichier existe mais que `load_style` echoue, c'est classifie comme `AppError::Style`. Mais puisque `load_style` ne peut echouer que sur une erreur IO (fichier illisible), un fichier lisible contenant du XML invalide passera `load_style` sans erreur et echouera plus tard dans `csl_proc`.

**Recommandation** : Ajouter une validation basique du XML CSL dans `load_style` (verifier la presence de `<style` et `</style>`) pour mapper correctement l'erreur sur le code 12.

### 9.4 Re-compilation de regex a chaque appel

**Severite : mineur**
**Fichier : `/home/user/csl-tools/src/markdown.rs`, lignes 21 et 248**

```rust
let pandoc_re = Regex::new(r"\[(@[^\]]+;[^\]]*)\]").unwrap();
// ...
let re = Regex::new(r"\[@([^\]\[,]+)(?:,\s*([^\]]+))?\](?:\(([^)]+)\))?").unwrap();
```

Les regex sont recompilees a chaque appel de `extract_pandoc_grouped_citations` et `extract_citations`. Pour un outil CLI execute une seule fois, l'impact est negligeable, mais cela reste une mauvaise pratique. L'utilisation de `lazy_static!` ou `std::sync::OnceLock` serait plus idiomatique.

### 9.5 Nommage de la commande

**Severite : information**

`overview.md` utilise systematiquement `csl-tool` (singulier) dans les exemples, tandis que `Cargo.toml` et `CLAUDE.md` utilisent `csl-tools` (pluriel). Le `#[command(name = "csl-tools")]` dans `main.rs` (L23) utilise le pluriel. Cette incoherence dans la specification n'affecte pas le fonctionnement mais pourrait preter a confusion dans la documentation utilisateur.

---

## Resume

| Categorie | Statut | Problemes |
|-----------|--------|-----------|
| Codes de sortie | Conforme | Heuristique fragile pour code 13 vs 14 (mineur) |
| Support stdin | Conforme | Aucun |
| Sous-commande `styles` | Conforme | Style unique disponible (attendu pour Phase 1) |
| Gestion sortie | Conforme | Pas de newline final sur stdout (mineur) |
| Messages d'erreur | Conforme | Pas de hint pour `CslProcessing` (mineur) |
| Texte d'aide | Conforme | Aucun |
| Flag `--format` | Phase 2 | Sortie hybride Markdown/HTML non documentee (important) |
| Cas limites | Conforme | Ecrasement fichier d'entree possible (mineur) |
| Validation style | Partiel | Code 12 non atteint pour XML invalide (mineur) |

**Verdict global** : L'implementation CLI est solide et conforme a la specification Phase 1. Les problemes releves sont majoritairement mineurs. Le seul point **important** concerne la sortie hybride Markdown/HTML qui n'est pas documentee et pourrait surprendre les utilisateurs.
