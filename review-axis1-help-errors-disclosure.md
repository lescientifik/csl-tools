# Revue Axe 1 : Progressive Disclosure, Aide & Erreurs

## Resume

`csl-tools` presente une base solide grace a l'utilisation de `clap` v4 en mode derive, qui fournit automatiquement une gestion correcte des erreurs d'arguments, des suggestions de typo, et une structure d'aide a deux niveaux. Cependant, l'outil reste en deca des recommandations du guide sur plusieurs points critiques : l'absence totale d'exemples dans le texte d'aide, le manque de contexte actionnable dans les erreurs applicatives, l'absence de hints apres succes, et une progressive disclosure limitee a seulement deux niveaux (top-level + process). Le CLI fonctionne, mais n'enseigne pas a l'utilisateur comment l'utiliser efficacement.

**Verdict global : Conformite partielle.** Les bases sont la, mais les aspects "qui enseignent" (exemples, hints, suggestions contextuelles) sont quasi absents.

---

## Analyse detaillee

### 1. Progressive Disclosure (Section 1 du guide)

#### Ce qui fonctionne

**Structure a deux niveaux fonctionnelle.** Le CLI offre une decouverte incrementale basique :

```
$ csl-tools --help
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

Puis au niveau sous-commande :

```
$ csl-tools process --help
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

**Appel sans arguments redirige vers l'aide.** L'invocation sans sous-commande affiche l'aide automatiquement (code de sortie 2, conforme a la convention pour les erreurs d'usage).

**Le sous-systeme `help <subcommand>` fonctionne.** `csl-tools help process` affiche bien l'aide de `process`.

#### Ce qui ne fonctionne pas

**1.1 - Absence de niveaux de decouverte intermediaires.** Le guide preconise un schema a quatre niveaux :

```
tool --help              -> apercu top-level (bref)
tool <command> --help    -> details specifiques
tool <domain> --list     -> enumerer les operations
tool <method> --describe -> schema des parametres
```

`csl-tools` ne propose que les deux premiers niveaux. Il n'y a par exemple aucun moyen de decouvrir les styles builtin disponibles sans lire le code source. Un agent IA ou un utilisateur qui tape `csl-tools process --csl apa` recevra :

```
Error: Failed to load CSL style 'apa': Failed to read file: No such file or directory (os error 2)
```

...sans aucune indication que des styles builtin existent, ni lesquels.

**1.2 - Pas de moyen de lister les capacites.** Les commandes `validate`, `list`, `bibliography` et `cite` documentees dans `overview.md` ne sont pas implementees. Il n'existe aucun moyen de :
- Lister les styles builtin (`csl-tools styles --list` ou `csl-tools process --csl --list`)
- Lister les cles de citation dans un fichier bib (`csl-tools list --bib refs.json`)
- Decouvrir les formats d'entree supportes

**1.3 - La documentation est dans `overview.md`, pas dans l'outil.** Le guide insiste : "The tool itself is the documentation." Or ici, des informations essentielles (syntaxe des citations, workflow PubMed, formats supportes) ne sont disponibles que dans des fichiers Markdown externes. Un agent IA doit lire ces fichiers separement.

#### Recommandations

**R1.1 - Ajouter une commande `styles` ou un flag `--list-styles`.**

```rust
#[derive(Subcommand)]
enum Commands {
    /// List available builtin styles
    Styles,
    // ...
}
```

Ou, plus leger, supporter `csl-tools process --csl ?` pour lister les styles.

**R1.2 - Implementer la commande `list` de overview.md.**

```rust
/// List citation keys available in a bibliography file
List {
    /// Bibliography file (CSL-JSON or JSONL)
    #[arg(short, long)]
    bib: PathBuf,
},
```

Cela permet la decouverte incrementale : l'utilisateur explore d'abord ses references, puis construit sa commande `process`.

**R1.3 - Ajouter un `after_help` ou `after_long_help` avec des exemples.**

```rust
#[command(after_help = "Examples:
  csl-tools process article.md --bib refs.json --csl minimal
  csl-tools process article.md --bib refs.json --csl style.csl -o output.html
  csl-tools process article.md --bib refs.json --csl minimal --no-bib")]
```

---

### 2. Erreurs qui enseignent (Section 2 du guide)

#### Inventaire des scenarios d'erreur testes

##### 2.1 Typo sur le nom de sous-commande

```
$ csl-tools proess
error: unrecognized subcommand 'proess'

  tip: a similar subcommand exists: 'process'

Usage: csl-tools <COMMAND>

For more information, try '--help'.
```

**Verdict : CONFORME.** Clap fournit automatiquement la suggestion de typo, conforme au pattern du guide :
```
err: unknown command 'navigat'
Did you mean: navigate
```

##### 2.2 Typo sur un flag

```
$ csl-tools process test.md --bibliography refs.json --csl apa.csl
error: unexpected argument '--bibliography' found

  tip: a similar argument exists: '--bib'

Usage: csl-tools process --bib <BIB> --csl <CSL> <INPUT>

For more information, try '--help'.
```

**Verdict : CONFORME.** Suggestion pertinente et usage rappele.

##### 2.3 Arguments requis manquants

```
$ csl-tools process
error: the following required arguments were not provided:
  --bib <BIB>
  --csl <CSL>
  <INPUT>

Usage: csl-tools process --bib <BIB> --csl <CSL> <INPUT>

For more information, try '--help'.
```

**Verdict : CONFORME.** Tous les arguments manquants sont listes, avec l'usage correct.

##### 2.4 Fichier d'entree inexistant

```
$ csl-tools process nonexistent.md --bib refs.json --csl minimal
Error: Failed to read input file 'nonexistent.md': No such file or directory (os error 2)
```

**Verdict : PARTIELLEMENT CONFORME.** L'erreur identifie bien le fichier problematique. Cependant, elle ne propose aucune action suivante. Le guide recommande :

> Every failed interaction must answer "what now?"

Amelioration suggeree :
```
Error: Failed to read input file 'nonexistent.md': No such file or directory
  hint: check that the file path is correct and the file exists
```

##### 2.5 Fichier de references invalide (JSON malform e)

```
$ csl-tools process article.md --bib bad_refs.json --csl minimal
Error: Failed to load bibliography file '/tmp/bad_refs.json': Invalid JSONL at line 1: expected ident at line 1 column 2
```

**Verdict : PARTIELLEMENT CONFORME.** Le message identifie le fichier et la ligne du probleme. Il manque cependant une indication du format attendu. L'utilisateur doit deviner que le fichier doit contenir du CSL-JSON ou du JSONL.

Amelioration suggeree :
```
Error: Failed to load bibliography file 'bad_refs.json': invalid JSON at line 1, column 2
  hint: the bibliography file must be a JSON array of CSL-JSON objects, or JSONL (one object per line)
  see: https://citeproc-js.readthedocs.io/en/latest/csl-json/markup.html
```

##### 2.6 Style CSL invalide

```
$ csl-tools process article.md --bib refs.json --csl bad_style.csl
Error: Failed to format bibliography: CSL processing error: No bibliography layout in style
```

**Verdict : NON CONFORME.** Le message est technique et non actionnable. "No bibliography layout in style" n'aide ni un humain ni un agent. Pas de suggestion.

Amelioration suggeree :
```
Error: Invalid CSL style 'bad_style.csl': missing <bibliography> element
  hint: ensure the file is a valid CSL style (.csl). Download styles from https://www.zotero.org/styles
```

##### 2.7 Nom de style builtin invalide / style introuvable

```
$ csl-tools process article.md --bib refs.json --csl nonexistent_style
Error: Failed to load CSL style 'nonexistent_style': Failed to read file: No such file or directory (os error 2)
```

**Verdict : NON CONFORME.** C'est l'erreur la plus problematique. L'utilisateur ne sait pas que `--csl` accepte a la fois des noms builtin et des chemins de fichier. Le message laisse penser que seuls les fichiers sont acceptes. Il n'y a aucune indication des styles builtin disponibles.

Amelioration suggeree :
```
Error: CSL style 'nonexistent_style' not found
  'nonexistent_style' is not a builtin style and no file with this name exists
  Available builtin styles: minimal
  hint: provide a path to a .csl file, or use a builtin style name
```

C'est critique pour la decouverte. Un agent IA qui essaie `--csl apa` n'a aucun moyen de savoir que `minimal` est la seule option builtin.

##### 2.8 Cle de citation introuvable dans la bibliographie

```
$ csl-tools process test.md --bib refs.json --csl minimal
Error: Failed to format citations: Reference not found: unknown-ref
```

**Verdict : PARTIELLEMENT CONFORME.** La cle manquante est identifiee. Cependant, il manque des informations cruciales pour le diagnostic :
- Quelles cles sont disponibles dans le fichier bib ?
- Y a-t-il une cle similaire (typo) ?

Amelioration suggeree :
```
Error: Reference not found: 'unknown-ref'
  Available keys in 'refs.json': item-1, item-2, item-3
  hint: check the citation key in your Markdown matches an 'id' field in your bibliography file
```

Ou au minimum, si la liste est longue :
```
Error: Reference not found: 'unknown-ref'
  Did you mean: 'unknown-ref-2'?
  hint: use 'csl-tools list --bib refs.json' to see available citation keys
```

##### 2.9 Chemin de sortie inexistant

```
$ csl-tools process article.md --bib refs.json --csl minimal -o /nonexistent/path/output.html
Error: Failed to write output file '/nonexistent/path/output.html': No such file or directory (os error 2)
```

**Verdict : PARTIELLEMENT CONFORME.** Le fichier problematique est identifie. Il manque un hint :

```
Error: Cannot write output file '/nonexistent/path/output.html': directory '/nonexistent/path' does not exist
  hint: create the output directory first, or use a different path
```

##### 2.10 Succes sans feedback

```
$ csl-tools process article.md --bib refs.json --csl minimal -o output.html
(aucune sortie)
```

**Verdict : NON CONFORME.** Le guide recommande explicitement :

> Next steps after success:
> ```
> Navigated to https://example.com (loaded in 2.3s)
> hint: use 'snapshot' to inspect content, 'eval' to extract data
> ```

Quand la sortie va dans un fichier (`-o`), l'utilisateur ne recoit aucune confirmation. Un agent IA ne sait pas si l'operation a reussi (il doit verifier le code de sortie uniquement).

Amelioration suggeree (sur stderr) :
```
Processed 3 citations, wrote output to 'output.html'
```

#### Resume des erreurs

| Scenario | Verdict | Suggestion de typo | Action suivante | Format attendu |
|----------|---------|-------------------|----------------|----------------|
| Typo sous-commande | CONFORME | oui | oui | n/a |
| Typo flag | CONFORME | oui | oui | n/a |
| Args manquants | CONFORME | n/a | oui | oui |
| Fichier introuvable | PARTIEL | n/a | non | n/a |
| JSON invalide | PARTIEL | n/a | non | non |
| Style invalide | NON CONFORME | n/a | non | non |
| Style inconnu | NON CONFORME | n/a | non | non |
| Ref introuvable | PARTIEL | non | non | non |
| Sortie impossible | PARTIEL | n/a | non | n/a |
| Succes (fichier) | NON CONFORME | n/a | non | n/a |

---

### 3. Design du texte d'aide (Section 5 du guide)

#### Aide top-level

**Le guide attend :**
1. Description en une ligne
2. 2-3 exemples d'invocation
3. Sous-commandes groupees par categorie
4. Pointeur vers `--help` pour details

**Sortie actuelle :**
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

**Analyse :**
- [x] Description en une ligne : presente ("Format citations and bibliographies in Markdown documents")
- [ ] 2-3 exemples d'invocation : **ABSENT** - aucun exemple
- [ ] Sous-commandes groupees par categorie : les 2 commandes ne necessitent pas de groupement, mais il n'y a pas de categorisation semantique
- [x] Pointeur vers `--help` : present implicitement via `help` dans la liste

**Probleme majeur : aucun exemple.** Un agent IA qui decouvre l'outil ne sait pas comment l'utiliser sans explorer `process --help`. Le guide est explicite sur ce point. L'aide top-level devrait montrer au minimum :

```
Examples:
  csl-tools process article.md --bib refs.json --csl style.csl
  csl-tools process article.md --bib refs.json --csl minimal -o output.html
```

#### Aide de sous-commande (`process`)

**Le guide attend :**
1. Ce que ca fait (une phrase)
2. Arguments requis vs optionnels
3. Un exemple avec des valeurs realistes

**Sortie actuelle :**
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

**Analyse :**
- [x] Ce que ca fait : present ("Process a Markdown file with citations")
- [x] Arguments requis vs optionnels : la syntaxe d'usage distingue `[OPTIONS]` des args requis, et les noms des flags requis sont en gras dans le format clap. C'est correct.
- [ ] Un exemple avec des valeurs realistes : **ABSENT**

**Probleme : aucun exemple.** Le guide demande explicitement "One example with realistic values". C'est particulierement important pour `process` qui est la commande principale et a 3 arguments requis de types differents (fichier Markdown, fichier JSON, fichier/nom CSL).

#### Aide de sous-commande (`skill-install`)

```
Install the Claude Code skill for citation formatting

Usage: csl-tools skill-install [OPTIONS]

Options:
  -d, --dir <DIR>  Installation directory (default: .claude/skills in current directory)
  -h, --help       Print help
```

**Analyse :** Correct et minimal. Pas de besoin d'exemple complexe ici.

#### Searchabilite

Le guide mentionne :

> If the tool has many commands/domains, provide search:
> ```
> tool --search cookie    -> lists commands related to cookies
> ```

Avec seulement 2 sous-commandes, ce n'est pas encore necessaire. Cependant, si les commandes `validate`, `list`, `bibliography`, `cite` de `overview.md` sont ajoutees, une fonctionnalite de recherche deviendrait pertinente.

#### Recommendations pour le texte d'aide

**R3.1 - Ajouter `after_help` au niveau top-level et `process`.**

Pour le top-level dans `main.rs` :
```rust
#[derive(Parser)]
#[command(name = "csl-tools")]
#[command(about = "Format citations and bibliographies in Markdown documents")]
#[command(version)]
#[command(after_help = "\
Examples:
  csl-tools process article.md --bib refs.json --csl style.csl
  csl-tools process article.md --bib refs.json --csl minimal -o output.html
  csl-tools process article.md --bib refs.json --csl minimal --no-bib")]
struct Cli {
```

Pour la sous-commande `process` :
```rust
/// Process a Markdown file with citations
#[command(after_help = "\
Examples:
  csl-tools process paper.md --bib refs.json --csl minimal
  csl-tools process paper.md -b refs.json -c ieee.csl -o paper.html
  csl-tools process paper.md -b refs.json -c minimal --no-bib

Builtin styles: minimal
Citation syntax: [@key], [@key](url), [@key, p. 42], [@a; @b; @c]")]
Process {
```

**R3.2 - Enrichir les descriptions des options.**

L'option `--csl` devrait preciser les valeurs acceptees :
```rust
/// CSL style: path to a .csl file, or builtin name (available: "minimal")
#[arg(short, long)]
csl: String,
```

L'option `--bib` devrait mentionner les formats supportes dans la description :
```rust
/// Bibliography file in CSL-JSON (array) or JSONL (one object per line) format
#[arg(short, long)]
bib: PathBuf,
```

---

## Recommandations prioritaires

### 1. Ajouter des exemples dans le texte d'aide (impact : eleve, effort : faible)

C'est le manque le plus flagrant. Le guide exige "2-3 example invocations" au top-level et "one example with realistic values" par sous-commande. L'implementation est triviale avec l'attribut `#[command(after_help = "...")]` de clap. Cela beneficie directement a la fois aux humains et aux agents IA.

### 2. Ameliorer l'erreur "style introuvable" avec la liste des styles builtin (impact : eleve, effort : faible)

Quand `--csl` recoit une valeur qui n'est ni un fichier existant ni un style builtin reconnu, le message actuel est trompeur. Il faut :
- Indiquer clairement que la valeur n'est ni un fichier ni un style builtin
- Lister les styles builtin disponibles
- Suggerer ou trouver des fichiers CSL

Implementation suggeree dans `main.rs`, fonction `process_command` :
```rust
let style_csl = if let Some(builtin) = builtin_style(csl) {
    builtin.to_string()
} else {
    let style_path = PathBuf::from(csl);
    load_style(&style_path).map_err(|e| {
        if style_path.exists() {
            format!("Failed to load CSL style '{}': {}", csl, e)
        } else {
            format!(
                "CSL style '{}' not found.\n  \
                 '{}' is not a builtin style name and no file with this path exists.\n  \
                 Available builtin styles: minimal\n  \
                 hint: provide a path to a .csl file, or download one from https://www.zotero.org/styles",
                csl, csl
            )
        }
    })?
};
```

### 3. Ajouter un message de confirmation sur stderr apres succes avec `-o` (impact : moyen, effort : faible)

Quand la sortie est ecrite dans un fichier, l'outil est completement silencieux. Ajouter sur stderr :
```rust
if let Some(output_path) = output {
    fs::write(output_path, &result)?;
    eprintln!(
        "Processed {} citation(s), output written to '{}'",
        clusters.len(),
        output_path.display()
    );
}
```

Cela repond au principe du guide : "every output is a conversation turn, not a dead end."

### 4. Ajouter des hints dans les erreurs applicatives (impact : moyen, effort : moyen)

Chaque erreur applicative devrait repondre a "what now?". Les cas prioritaires :
- **Reference introuvable** : suggerer `csl-tools list --bib <file>` (quand la commande existera) ou lister les cles disponibles
- **JSON invalide** : rappeler le format attendu (CSL-JSON ou JSONL)
- **Fichier introuvable** : verifier que le chemin est correct

### 5. Implementer les commandes `list` et `validate` de overview.md (impact : eleve, effort : moyen)

Ces commandes sont essentielles pour la progressive disclosure. Elles permettent a un utilisateur ou agent de :
- Explorer les donnees disponibles avant de lancer `process` (`list`)
- Verifier la validite des entrees avant de les utiliser (`validate`)

Ce sont des "levels 3 et 4" de la progressive disclosure : enumerer les operations et inspecter les parametres. Sans elles, l'utilisateur doit deviner les cles de citation et esperer que son fichier JSON est valide.

---

## Annexe : Codes de sortie observes

| Scenario | Code | Attendu (guide) | Conforme |
|----------|------|-----------------|----------|
| Succes | 0 | 0 | oui |
| Erreur d'usage (clap) | 2 | 2 | oui |
| Erreur applicative (fichier, ref, style) | 1 | 1 | oui |

Les codes de sortie sont corrects et conformes a la section 3 du guide. Clap utilise le code 2 pour les erreurs d'usage, et le code applicatif dans `main.rs` utilise `process::exit(1)` pour les erreurs runtime. C'est un bon point.
