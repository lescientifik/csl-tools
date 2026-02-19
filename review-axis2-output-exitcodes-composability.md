# Revue Axe 2 : Output Design, Codes de Sortie & Composabilite

## Resume

`csl-tools` presente une base solide pour un outil CLI compose d'un seul point de vue : la separation stdout/stderr est correcte pour la commande `process`, les codes de sortie distinguent les erreurs d'usage (code 2, via clap) des erreurs d'execution (code 1), et la sortie est suffisamment sobre pour etre pipee. Cependant, plusieurs lacunes significatives sont identifiees : **absence totale de support JSON** (`--json`), **un unique code de sortie 1 pour toutes les erreurs d'execution** (pas de granularite semantique), **la commande `skill-install` pollue stdout avec des messages informatifs**, et **aucune option de controle de la verbosite** (`--quiet`/`--verbose`). Le respect des recommandations du guide est partiel -- environ 50% des criteres des sections 3, 4 et 8 sont satisfaits.

---

## Analyse detaillee

### 1. Codes de sortie semantiques (Section 3)

#### Cartographie des codes observes

| Scenario | Commande | Code de sortie | Canal d'erreur |
|----------|----------|---------------|----------------|
| Succes | `process article.md --bib refs.json --csl minimal.csl` | **0** | (rien) |
| Succes avec `--help` | `--help` | **0** | (rien) |
| Succes avec `--version` | `--version` | **0** | (rien) |
| Sous-commande manquante | `csl-tools` (sans argument) | **2** | stderr (clap) |
| Flag requis manquant (`--bib`) | `process article.md --csl style.csl` | **2** | stderr (clap) |
| Sous-commande inconnue | `csl-tools unknown-command` | **2** | stderr (clap) |
| Flag inconnu (`--json`) | `process ... --json` | **2** | stderr (clap) |
| Fichier d'entree introuvable | `process nonexistent.md ...` | **1** | stderr |
| Fichier bib introuvable | `process ... --bib nonexistent.json ...` | **1** | stderr |
| Fichier CSL introuvable | `process ... --csl nonexistent.csl` | **1** | stderr |
| JSON invalide dans bib | `process ... --bib invalid.json ...` | **1** | stderr |
| Reference citee non trouvee | `process missing_ref.md ...` | **1** | stderr |
| Ecriture fichier impossible | `process ... -o /proc/1/nope.html` | **1** | stderr |

#### Analyse par rapport au guide

Le guide recommande :

| Code | Signification (guide) | Etat actuel |
|------|----------------------|-------------|
| 0 | Succes | **Conforme** |
| 1 | Erreur au niveau commande (site not found, element missing) | **Partiellement conforme** -- toutes les erreurs d'execution retournent 1, sans distinction |
| 2 | Erreur d'usage (bad args, unknown command) | **Conforme** -- gere par clap automatiquement |
| 3+ | Erreurs specifiques recuperables | **Non implemente** |

**Probleme principal** : le code 1 est utilise pour *toutes* les erreurs d'execution, qu'il s'agisse de :
- Un fichier introuvable (potentiellement corrigeable en ajustant le chemin)
- Du JSON invalide (erreur de donnees, necessite correction manuelle)
- Une reference manquante (erreur logique dans le document)
- Une erreur d'ecriture (probleme de permissions)

Un agent IA recevant le code 1 ne peut pas distinguer ces cas sans parser le texte de l'erreur sur stderr. Cela contredit directement le principe : *"The more semantic the exit code, the less text parsing the agent needs."*

**Sortie terminal observee pour le fichier introuvable :**
```
$ target/debug/csl-tools process nonexistent.md --bib refs.json --csl minimal.csl
Error: Failed to read input file 'nonexistent.md': No such file or directory (os error 2)
$ echo $?
1
```

**Sortie terminal observee pour JSON invalide :**
```
$ target/debug/csl-tools process article.md --bib invalid_refs.json --csl minimal.csl
Error: Failed to load bibliography file 'invalid_refs.json': Invalid JSONL at line 1: key must be a string at line 1 column 18
$ echo $?
1
```

**Sortie terminal observee pour reference manquante :**
```
$ target/debug/csl-tools process missing_ref.md --bib refs.json --csl minimal.csl
Error: Failed to format citations: Reference not found: nonexistent-key
$ echo $?
1
```

Tous retournent le code 1 de maniere indistincte.

#### Code source correspondant

Dans `main.rs`, la fonction `main()` :
```rust
fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        process::exit(1);  // <-- Toujours 1, quelle que soit l'erreur
    }
}
```

Il n'y a aucune logique de mapping entre le type d'erreur et le code de sortie. Les types d'erreur existent pourtant dans `processor.rs` (`ProcessorError::ReferenceNotFound`, `ProcessorError::InvalidJson`, etc.) et `refs.rs` (`RefsError::IoError`, `RefsError::JsonError`, etc.), mais ils sont convertis en `Box<dyn Error>` via les `.map_err()` dans `process_command()`, perdant toute information de typage.

---

### 2. Design de la sortie (Section 4)

#### 2.1 Separation stdout / stderr

**Commande `process` -- Conforme**

En cas de succes, seule la sortie formatee apparait sur stdout. Les erreurs vont sur stderr. Cela permet le piping :

```
$ target/debug/csl-tools process article.md --bib refs.json --csl minimal.csl 2>/dev/null | grep coronavirus
Les coronavirus sont une famille de virus (Hu, Guo).
```

```
$ target/debug/csl-tools process article.md --bib refs.json --csl minimal.csl 2>/dev/null | wc -l
17
```

En cas d'erreur, stdout est vide et l'erreur va sur stderr :
```
$ target/debug/csl-tools process nonexistent.md --bib refs.json --csl minimal.csl 2>/dev/null | wc -c
0
```

**Commande `skill-install` -- Non conforme**

La commande `skill-install` envoie des messages informatifs sur **stdout** :
```
$ target/debug/csl-tools skill-install
Claude Code skill installed successfully!
  Location: .claude/skills/csl-format/SKILL.md

The skill is now available in Claude Code when working in this directory.
Use it by asking Claude to format your citations, or invoke it with:
  /csl-format
```

Ce texte informatif devrait aller sur stderr, conformement au principe *"stdout: Data output only. stderr: Hints, warnings, progress."* Si un agent essaie de capturer la sortie de cette commande pour determiner le chemin d'installation, il recevra 5 lignes de texte narratif au lieu d'un simple chemin.

Le code correspondant dans `main.rs` :
```rust
println!("Claude Code skill installed successfully!");
println!("  Location: {}", skill_path.display());
println!();
println!("The skill is now available in Claude Code when working in this directory.");
// ...
```

Tous ces `println!` devraient etre `eprintln!`, ou mieux : seul le chemin du fichier installe devrait aller sur stdout, et le reste sur stderr.

#### 2.2 Efficacite en tokens

**Points positifs :**
- Pas de bannieres ASCII, logos decoratifs ou bordures
- Le texte d'aide est concis (genere par clap)
- La sortie de `process` est le contenu formate brut, sans enveloppe superflue
- Les messages d'erreur sont sur une seule ligne avec contexte utile

**Points negatifs :**
- La commande `skill-install` est bavarde (5 lignes pour dire "installe")
- Les messages d'erreur incluent parfois des messages systeme redondants : `"Failed to read file: No such file or directory (os error 2)"` -- le `(os error 2)` est du bruit pour un agent

**Sortie de `--help` (top-level) :** Concise et bien structuree :
```
Format citations and bibliographies in Markdown documents

Usage: csl-tools <COMMAND>

Commands:
  process        Process a Markdown file with citations
  skill-install  Install the Claude Code skill for citation formatting
  help           Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

**Sortie de `process --help` :** Egalement concise et informative :
```
Process a Markdown file with citations

Usage: csl-tools process [OPTIONS] --bib <BIB> --csl <CSL> <INPUT>

Arguments:
  <INPUT>  Input Markdown file

Options:
  -b, --bib <BIB>                Bibliography file (CSL-JSON or JSONL)
  -c, --csl <CSL>                CSL style file or builtin style name
  -o, --output <OUTPUT>          Output file (default: stdout)
      --no-bib                   Don't include bibliography
      --bib-header <BIB_HEADER>  Custom bibliography header [default: "## References"]
  -h, --help                     Print help
```

#### 2.3 Support JSON

**Absent.** Il n'existe pas de flag `--json` :
```
$ target/debug/csl-tools process article.md --bib refs.json --csl minimal.csl --json
error: unexpected argument '--json' found
$ echo $?
2
```

Le guide recommande : *"Default to concise, readable text. Provide `--json` when structured data is needed."* avec la nuance : *"if the output is data (records, lists, structured results), offer JSON."*

Pour la commande `process`, la sortie est un document Markdown/HTML transforme -- c'est du contenu narratif, pas des donnees structurees. Le `--json` n'est donc **pas critique** pour cette commande.

En revanche, pour les commandes prevues dans l'`overview.md` mais non encore implementees (`list`, `validate`, `cite`, `bibliography`), le `--json` serait pertinent :
- `list --bib refs.json --json` pourrait retourner `["item-1", "item-2"]`
- `validate --bib refs.json --json` pourrait retourner `{"valid": true}` ou `{"valid": false, "errors": [...]}`
- `cite --json` pourrait retourner `{"formatted": "(Doe, 2021)"}`

#### 2.4 Predictabilite et coherence de la sortie

**Points positifs :**
- Le format de sortie est previsible : Markdown avec citations remplacees + bibliographie HTML
- Les erreurs suivent un format constant : `Error: <contexte>: <detail>`
- Pas de sortie ANSI/couleurs (verifie avec `cat -v`)

**Points negatifs :**
- Pas d'option `--quiet` pour supprimer la sortie informative
- Pas d'option `--verbose` pour obtenir plus de details diagnostiques
- Le format des erreurs de clap (usage) est different du format des erreurs d'execution :
  - Clap : `error: the following required arguments were not provided: --bib <BIB>`
  - Runtime : `Error: Failed to read input file 'nonexistent.md': ...`
  - Le prefixe differe (`error:` minuscule vs `Error:` majuscule) ce qui rend le parsing fragile

---

### 3. Composabilite (Section 8)

#### 3.1 Pipe-friendliness

**Conforme.** La sortie de `process` est directement pipeable :

```
$ target/debug/csl-tools process article.md --bib refs.json --csl minimal.csl 2>/dev/null | grep -c "csl-entry"
2
```

```
$ target/debug/csl-tools process article.md --bib refs.json --csl minimal.csl 2>/dev/null | head -3
# Introduction

Les coronavirus sont une famille de virus (Hu, Guo).
```

La sortie ne contient aucun code ANSI, banniere ou decoration qui perturberait un pipe. L'absence de couleurs est un point positif pour la composabilite.

#### 3.2 Chainage avec &&

**Conforme.** Les codes de sortie permettent le chainage :

```
$ target/debug/csl-tools process article.md --bib refs.json --csl minimal.csl -o output.html 2>/dev/null && echo "CHAIN_SUCCESS"
CHAIN_SUCCESS
```

```
$ target/debug/csl-tools process nonexistent.md --bib refs.json --csl minimal.csl -o output.html 2>/dev/null && echo "CHAIN_SUCCESS"
(rien -- la commande a echoue avec code 1, && n'execute pas echo)
```

Le chainage fonctionne correctement grace a la distinction code 0 (succes) / code non-zero (echec).

#### 3.3 Comportement non-TTY

**Conforme.** Aucun prompt interactif n'est declenche en mode non-TTY :

```
$ echo "" | target/debug/csl-tools
Format citations and bibliographies in Markdown documents
Usage: csl-tools <COMMAND>
...
$ echo $?
2
```

L'outil ne tente jamais de lire stdin de maniere interactive. Il affiche l'aide et retourne le code 2 quand aucune sous-commande n'est fournie, ce qui est le comportement attendu.

**Cependant**, il n'y a pas de support stdin (`-`) pour l'entree Markdown, ce qui limiterait la composabilite dans un pipeline :
```
# Impossible actuellement :
cat article.md | csl-tools process - --bib refs.json --csl style.csl
```

Le guide recommande dans la section 6 : *"Support `-` for stdin on data-heavy inputs."* Bien que cela releve de la section Input Design (hors scope de cette revue), cela impacte la composabilite.

#### 3.4 Sortie vers fichier vs stdout

Quand `-o` est specifie, la sortie va dans le fichier et stdout reste vide. Quand `-o` est omis, la sortie va sur stdout. Ce comportement est coherent et permet les deux usages :

```bash
# Pipeline
csl-tools process article.md --bib refs.json --csl style.csl | further-processing

# Fichier de sortie
csl-tools process article.md --bib refs.json --csl style.csl -o output.html
```

---

## Recommandations prioritaires

### 1. Implementer des codes de sortie semantiques (Priorite haute)

Definir un mapping clair entre les types d'erreur et les codes de sortie :

| Code | Signification |
|------|--------------|
| 0 | Succes |
| 1 | Erreur de fichier (entree/sortie introuvable, permissions) |
| 2 | Erreur d'usage (gere par clap -- deja en place) |
| 3 | Erreur de donnees (JSON invalide, CSL invalide) |
| 4 | Erreur de reference (cle citee non trouvee dans la bibliographie) |
| 5 | Erreur du moteur CSL (erreur interne de `csl_proc`) |

Implementation suggeree dans `main.rs` :
```rust
fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        let code = match e.downcast_ref::</* type specifique */>() {
            // Mapper les types d'erreur aux codes
            ...
        };
        process::exit(code);
    }
}
```

Ou mieux : definir un `enum AppError` au niveau `main.rs` qui encapsule tous les cas et implemente la methode `exit_code()`.

### 2. Corriger la sortie de `skill-install` (Priorite haute)

Deplacer les messages informatifs de stdout vers stderr, et n'envoyer que la donnee utile (le chemin) sur stdout :

```rust
fn skill_install_command(dir: Option<&Path>) -> Result<(), Box<dyn std::error::Error>> {
    // ... installation ...

    // Seule donnee sur stdout : le chemin
    println!("{}", skill_path.display());

    // Messages informatifs sur stderr
    eprintln!("Claude Code skill installed successfully!");
    eprintln!("The skill is now available in Claude Code.");

    Ok(())
}
```

Cela permettrait : `SKILL_PATH=$(csl-tools skill-install)` dans un script.

### 3. Ajouter `--json` pour les futures commandes de donnees (Priorite moyenne)

Quand les commandes `list`, `validate`, `cite` et `bibliography` seront implementees (Phases 2-3 du overview.md), prevoir le flag `--json` :

```bash
csl-tools list --bib refs.json --json
# ["item-1", "item-2"]

csl-tools validate --bib refs.json --json
# {"valid": true}
```

Pour `process`, le `--json` n'est pas prioritaire car la sortie est du contenu narratif.

### 4. Harmoniser le format des messages d'erreur (Priorite moyenne)

Actuellement, deux formats coexistent :
- Clap : `error: the following required arguments...` (minuscule, pas de prefixe "Error:")
- Runtime : `Error: Failed to read...` (majuscule)

Bien qu'on ne puisse pas facilement changer le format de clap, il serait utile de documenter cette convention et de s'assurer que les erreurs runtime sont parsables de maniere fiable. Envisager un flag `--json` pour les erreurs egalement :
```json
{"error": "file_not_found", "path": "nonexistent.md", "message": "No such file or directory"}
```

### 5. Ajouter `--quiet` (Priorite basse)

Un flag `--quiet` / `-q` qui supprimerait tous les messages sur stderr sauf les erreurs fatales. Cela reduirait le bruit pour les agents dans des pipelines complexes et les scripts CI.

---

## Tableau recapitulatif de conformite

| Critere (guide) | Etat | Note |
|-----------------|------|------|
| **Section 3 : Codes de sortie** | | |
| Code 0 pour succes | Conforme | |
| Code 2 pour erreur d'usage | Conforme | Via clap |
| Code 1 pour erreur commande | Partiellement | Pas de granularite |
| Codes 3+ pour erreurs specifiques | Non implemente | |
| **Section 4 : Design de la sortie** | | |
| stdout = donnees uniquement | Partiellement | `skill-install` pollue stdout |
| stderr = erreurs/hints/progress | Conforme | Pour `process` |
| `--json` disponible | Non implemente | |
| Sortie concise, pas de decoration | Conforme | |
| Format predictible et stable | Conforme | |
| **Section 8 : Composabilite** | | |
| Sortie pipeable | Conforme | |
| Chainage && fonctionne | Conforme | |
| Pas de prompt interactif en non-TTY | Conforme | |
| Support stdin (`-`) | Non implemente | |
