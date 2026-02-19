# Revue Axe 3 : Design des Entrees, Robustesse & Perennite

## Resume

`csl-tools` presente un design CLI globalement solide pour un outil en phase MVP. Les performances sont excellentes (< 20 ms pour toutes les operations), l'idempotence est assuree, et la separation stdout/stderr est correcte. En revanche, plusieurs lacunes meritent attention : l'absence de support stdin via `-`, l'utilisation d'un code de sortie unique (`1`) pour toutes les erreurs applicatives, l'absence de `--format`/`--json` pourtant documentes dans la spec, et le risque d'ecrasement silencieux quand l'entree et la sortie pointent vers le meme fichier. La conformite aux recommandations agent-specifiques (section 10 du guide) est partiellement atteinte : la sortie est concise et sans decoration, mais il manque des mecanismes de controle de la verbosity (`--quiet`), de troncature (`--limit`), et de sortie structuree (`--json`).

**Verdict : 6.5/10** -- Bonne base, mais des ameliorations sont necessaires avant une utilisation agent-first en production.

---

## Analyse detaillee

### 1. Design des entrees (Section 6)

#### 1.1 Positional vs flags

La commande `process` utilise un argument positionnel pour le fichier d'entree et des flags pour les autres parametres :

```
csl-tools process <INPUT> --bib <BIB> --csl <CSL> [-o OUTPUT]
```

**Evaluation :** Conforme a la recommandation du guide. L'argument positionnel est justifie car `process` a un seul argument "evident" (le fichier Markdown). Les flags `--bib` et `--csl` sont obligatoires et explicites, ce qui facilite la generation de commandes par un agent. La commande `skill-install` n'a que des flags optionnels, ce qui est correct.

**Bon point :** Les flags courts sont disponibles et coherents :
- `-b` pour `--bib`
- `-c` pour `--csl`
- `-o` pour `--output`
- `-d` pour `--dir` (skill-install)

```
$ csl-tools process article.md -b refs.json -c style.csl -o output.html
EXIT: 0
```

#### 1.2 Valeurs par defaut

| Parametre | Defaut | Evaluation |
|-----------|--------|------------|
| `--output` | stdout | Correct -- conforme aux conventions Unix |
| `--bib-header` | `"## References"` | Correct -- valeur raisonnable |
| `--no-bib` | `false` | Correct -- la bibliographie est incluse par defaut |
| `--csl` | aucun (requis) | Acceptable mais perfectible (voir recommandation) |
| `--bib` | aucun (requis) | Correct -- pas de defaut logique |

**Point d'attention :** La specification `overview.md` mentionne des options `--format <fmt>` (html/markdown) et `--locale <code>` qui ne sont pas implementees. La sous-commande `process` n'a pas de flag `--format`. Un agent qui suivrait la documentation de `overview.md` genererait des commandes invalides :

```
$ csl-tools process article.md --bib refs.json --csl style.csl --format html
error: unexpected argument '--format' found
EXIT: 2
```

C'est un ecart spec/implementation a corriger prioritairement.

#### 1.3 Support stdin

**Le support stdin via `-` est absent.**

```
$ echo "[@item-1]" | csl-tools process - --bib refs.json --csl style.csl
Error: Failed to read input file '-': No such file or directory (os error 2)
EXIT: 1
```

Le contournement via `/dev/stdin` fonctionne sous Linux :

```
$ echo "[@item-1]" | csl-tools process /dev/stdin --bib refs.json --csl style.csl
Les résultats montrent (Hu, Guo, 2021) que la méthode fonctionne.
EXIT: 0
```

Mais ce contournement n'est pas portable (pas disponible sur Windows) et n'est pas documente. Selon le guide (section 6) : *"Support `-` for stdin on data-heavy inputs. Agents generate multi-line content more easily via stdin than via shell quoting."*

Ceci est particulierement pertinent pour un agent qui genere du Markdown a la volee et veut le formater sans creer de fichier temporaire.

**Code concerne :** `src/main.rs`, ligne 107 :
```rust
let markdown = fs::read_to_string(input)
    .map_err(|e| format!("Failed to read input file '{}': {}", input.display(), e))?;
```

**Correction suggeree :**
```rust
let markdown = if input == Path::new("-") {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)
        .map_err(|e| format!("Failed to read from stdin: {}", e))?;
    buf
} else {
    fs::read_to_string(input)
        .map_err(|e| format!("Failed to read input file '{}': {}", input.display(), e))?
};
```

#### 1.4 Coherence des flags entre sous-commandes

Les deux sous-commandes (`process` et `skill-install`) n'ont aucun flag en commun, donc pas de risque d'incoherence. La specification mentionne des sous-commandes futures (`bibliography`, `cite`, `validate`, `list`) qui partageraient `--bib` et `--csl`. La coherence devra etre maintenue lors de leur implementation.

**Risque identifie :** `--bib` a le short `-b` dans `process`. Si `bibliography`, `cite` et `validate` adoptent le meme pattern, c'est coherent. A surveiller.

---

### 2. Robustesse (Section 7)

#### 2.1 Temps de reponse

Tous les benchmarks ont ete realises avec le build `release` sur la machine de test.

| Commande | Temps reel | Evaluation |
|----------|-----------|------------|
| `process` (2 citations, stdout) | 13 ms | Excellent (< 100 ms) |
| `process` (2 citations, fichier) | 14 ms | Excellent |
| `process` (100 citations) | 17 ms | Excellent |
| `--help` | 10 ms | Excellent |
| `process --help` | 12 ms | Excellent |
| `--version` | 10 ms | Excellent |
| `process` (style builtin) | 13 ms | Excellent |

**Verdict :** Conformite totale avec la recommandation "Print something within 100ms". L'outil est instantane pour toutes les operations testees, y compris avec 100 citations. Aucun appel reseau n'est necessaire, ce qui elimine le risque de latence.

#### 2.2 Idempotence

L'execution repetee de la meme commande produit des sorties identiques :

```
$ diff run1.out run2.out
IDENTICAL (idempotent)
```

Ceci est vrai pour la sortie stdout comme pour la sortie fichier. **Conformite totale.**

#### 2.3 Design crash-only

L'outil est entierement sans etat (stateless). Il n'utilise pas de daemon, pas de fichier de lock, pas de cache, pas de base de donnees. Chaque invocation est independante.

**Verdict :** Conformite totale. Le crash n'a aucune consequence sur les invocations suivantes.

**Point d'attention -- ecrasement du fichier source :** L'outil permet d'ecrire la sortie dans le meme fichier que l'entree :

```
$ csl-tools process article.md --bib refs.json --csl style.csl -o article.md
EXIT: 0
```

Ceci fonctionne car le fichier est lu en entier avant l'ecriture. Cependant, si l'ecriture echoue a mi-parcours (disque plein, permissions), le fichier source est corrompu de maniere irrecuperable. C'est un risque pour la robustesse, surtout en contexte agent ou les chemins peuvent etre mal geres.

**Recommandation :** Soit detecter et interdire le cas `input == output`, soit ecrire dans un fichier temporaire puis faire un rename atomique.

#### 2.4 Gestion des timeouts

L'outil n'a pas de mecanisme de timeout interne, ce qui est acceptable car :
- Il ne fait aucune operation reseau
- Le traitement est purement CPU-bound et tres rapide
- 100 citations sont traitees en 17 ms

Cependant, si un jour l'outil supporte des operations reseau (ex: telecharger un style CSL depuis un URL), un timeout sera necessaire.

**Risque actuel :** Si `--csl` pointe vers un chemin NFS ou un systeme de fichiers distant monte, le `read_to_string` pourrait bloquer indefiniment. Mais ce risque est marginal.

#### 2.5 Codes de sortie

| Situation | Code | Semantique |
|-----------|------|-----------|
| Succes | 0 | OK |
| Arguments manquants | 2 | Erreur d'usage (clap) |
| Sous-commande inconnue | 2 | Erreur d'usage (clap) |
| Fichier introuvable | 1 | Erreur applicative |
| Reference manquante | 1 | Erreur applicative |
| JSON invalide | 1 | Erreur applicative |
| Style invalide | 1 | Erreur applicative |

**Probleme :** Toutes les erreurs applicatives retournent le code `1`. Le guide recommande d'utiliser des codes semantiques :

> | Range | Meaning | Agent action |
> |-------|---------|-------------|
> | 0 | Success | Continue |
> | 1 | Command-level error | Adjust input, retry |
> | 2 | Usage error | Fix invocation |
> | 3+ | Specific recoverable errors | Contextual retry/backoff |

Un agent ne peut pas distinguer "fichier non trouve" (reessayer avec un autre chemin) de "reference manquante" (corriger le Markdown) sans parser le message d'erreur textuel. Des codes distincts (ex: 10 = fichier introuvable, 11 = reference manquante, 12 = JSON invalide, 13 = style invalide) permettraient un branchement sans parsing.

**Code concerne :** `src/main.rs`, lignes 62-65 :
```rust
fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
```

#### 2.6 Messages d'erreur

Les messages d'erreur sont clairs et contextuels :

```
Error: Failed to read input file '/nonexistent/file.md': No such file or directory (os error 2)
Error: Failed to load bibliography file '/nonexistent/refs.json': Failed to read file: ...
Error: Failed to format citations: Reference not found: nonexistent-key
Error: Failed to load CSL style '/nonexistent/style.csl': Failed to read file: ...
```

**Bon point :** Les messages incluent le chemin du fichier concerne, ce qui est excellent pour le diagnostic. Clap fournit egalement des suggestions de correction pour les typos :

```
$ csl-tools proces
error: unrecognized subcommand 'proces'
  tip: a similar subcommand exists: 'process'
```

**Manque :** Les messages d'erreur ne contiennent pas de suggestion "what now?" comme le recommande le guide (section 2). Par exemple, pour une reference manquante :

```
Error: Failed to format citations: Reference not found: nonexistent-key
```

Serait plus utile avec :
```
Error: Reference not found: 'nonexistent-key'
hint: check that this key exists in your bibliography file (--bib)
hint: available keys: item-1, item-2
```

---

### 3. Perennite (Section 9)

#### 3.1 Nommage

| Element | Nom actuel | Evaluation |
|---------|-----------|------------|
| Sous-commande | `process` | Explicite, pas ambigu |
| Sous-commande | `skill-install` | Explicite, kebab-case |
| Flag | `--bib` | Abbreviation claire, sans ambiguite |
| Flag | `--csl` | Acronyme standard du domaine |
| Flag | `--no-bib` | Convention `--no-*` respectee |
| Flag | `--bib-header` | Explicite |
| Flag | `--output` | Standard |
| Flag | `--dir` | Abbreviation de `directory`, standard |

**Verdict :** Le nommage est globalement bon. Aucune abbreviation arbitraire qui bloquerait l'ajout futur de commandes. Le guide avertit specifiquement : *"if `n` means `navigate` today, you can't add `new` later"*. Ici, les short flags `-b`, `-c`, `-o`, `-d` sont suffisamment peu ambigus dans leur contexte respectif.

**Point d'attention :** La sous-commande `skill-install` utilise le kebab-case, ce qui differe du style de `process` (un seul mot). Si des sous-commandes futures sont ajoutees (`validate`, `list`, `cite`, `bibliography`), la convention reste coherente. Mais si une sous-commande composee comme `bib-check` ou `ref-list` est ajoutee, le pattern kebab-case est deja etabli.

#### 3.2 Risques de changements cassants

**Risque eleve : ecart spec/implementation.** La specification (`overview.md`) documente des options qui n'existent pas :

- `--format <fmt>` (html, markdown)
- `--locale <code>`

Et des sous-commandes qui n'existent pas :

- `bibliography`
- `cite`
- `validate`
- `list`

Si ces fonctionnalites sont ajoutees plus tard, il faut s'assurer qu'elles ne modifient pas le comportement existant de `process`. C'est un ajout pur (additive change), donc conforme au guide.

**Risque modere : format de sortie.** La sortie actuelle est du Markdown avec du HTML inline pour la bibliographie. Il n'y a pas de mecanisme pour garantir la stabilite de ce format. Un changement dans `csl_proc` pourrait modifier le HTML genere sans avertissement.

**Risque faible : deprecation.** L'outil n'a pas encore eu besoin de deprecier quoi que ce soit. Aucun mecanisme de deprecation (avertissements sur stderr) n'est en place, mais c'est premature pour un v0.1.0.

#### 3.3 Stabilite de la sortie machine

Il n'y a pas de mode `--json` :

```
$ csl-tools process article.md --bib refs.json --csl style.csl --json
error: unexpected argument '--json' found
EXIT: 2
```

La sortie est toujours du texte (Markdown + HTML). Le guide recommande :

> *"human-readable output can change; `--json` output is a contract."*

Pour `process`, une sortie JSON n'est pas prioritaire car le resultat est un document formatte (narratif). En revanche, pour les sous-commandes futures `list`, `validate` et `cite`, un mode `--json` sera essentiel :

- `list --json` : tableau de cles de citation
- `validate --json` : rapport de validation structure
- `cite --json` : citation formattee + metadata

#### 3.4 Bug de double en-tete de bibliographie

Un bug visible dans la sortie actuelle :

```
## Références         <-- en-tête existant dans le Markdown source
                      <-- ligne vide
## References         <-- en-tête auto-ajouté par csl-tools

<div class="csl-bib-body">
```

L'outil ne detecte pas qu'un en-tete de bibliographie existe deja dans le document source. Il ajoute systematiquement `## References` (ou la valeur de `--bib-header`). Ceci produit un double en-tete dans les documents qui ont deja une section "References" ou "Bibliographie".

**Impact :** Un agent qui traite le meme document deux fois verrait les en-tetes se multiplier (non-idempotence du contenu semantique, meme si les octets sont identiques pour un meme document source).

---

### 4. Considerations specifiques aux agents (Section 10)

#### 4.1 Budget de tokens

| Sortie | Taille | Evaluation |
|--------|--------|------------|
| `--help` | 368 octets (12 lignes) | Excellent -- concis |
| `process --help` | 555 octets (14 lignes) | Bon -- detaille mais pas verbeux |
| Sortie `process` (2 citations) | 530 octets (19 lignes) | Bon -- proportionnel au contenu |
| Sortie `process` (100 citations) | 11 317 octets | Acceptable mais non controlable |

**Probleme :** Il n'y a aucun mecanisme de troncature ou pagination. Le guide recommande :

> *"Offer truncation/pagination flags: `--limit`, `--offset`, `--max-depth`."*

Pour `process`, c'est moins pertinent (on veut le document complet). Mais pour les sous-commandes futures `list` et `bibliography`, des flags de troncature seront necessaires.

**Probleme secondaire :** La sortie de `skill-install` est verbeuse pour un agent :

```
Claude Code skill installed successfully!
  Location: /tmp/skills/csl-format/SKILL.md

The skill is now available in Claude Code when working in this directory.
Use it by asking Claude to format your citations, or invoke it with:
  /csl-format
```

Un agent n'a besoin que de la confirmation de succes et du chemin. Idealement :
```
installed: /tmp/skills/csl-format/SKILL.md
```

Ou encore mieux, rien sur stdout et le chemin retourne uniquement si demande.

#### 4.2 Previsibilite du format de sortie

La sortie de `process` est previsible : c'est le contenu Markdown original avec les citations remplacees, suivi optionnellement d'un en-tete et d'un bloc `<div class="csl-bib-body">`.

**Bon point :** Pas de banniere, pas d'art ASCII, pas de decoration. La sortie est directement pipeable :

```
$ csl-tools process article.md -b refs.json -c style.csl | head -1
# Introduction
```

**Point d'attention :** La sortie stdout inclut un `\n` final supplementaire ajoute par `writeln!` (ligne 172 de `main.rs`). Cela signifie que la sortie stdout et la sortie fichier different d'un octet :

```rust
// stdout: writeln! ajoute \n
writeln!(handle, "{}", result)?;
// fichier: fs::write n'ajoute pas \n
fs::write(output_path, &result)?;
```

Cette incoherence peut surprendre un agent qui compare les deux methodes.

#### 4.3 Placement des hints (stderr vs stdout)

**Bon point :** Les erreurs sont correctement envoyees sur stderr :

```
$ csl-tools process /nonexist.md --bib refs.json --csl style.csl > stdout.txt 2> stderr.txt
$ cat stdout.txt
(vide)
$ cat stderr.txt
Error: Failed to read input file '/nonexist.md': No such file or directory (os error 2)
```

Sur un succes, stderr est completement vide. C'est ideal pour le piping.

**Point d'attention :** La sous-commande `skill-install` envoie ses messages de succes sur stdout (`println!`). Si un agent redirige stdout pour capturer un resultat, il recevra le texte de confirmation melange. Idealement, les messages informatifs iraient sur stderr et seul le chemin du fichier cree irait sur stdout.

#### 4.4 Statefulness

**Conformite totale.** L'outil est entierement stateless :
- Pas de daemon
- Pas de fichier de configuration globale
- Pas de cache
- Pas de fichier de lock
- Pas de variable d'environnement requise
- Chaque invocation est autonome

C'est le scenario ideal pour un agent. Pas de "is the browser running?" failure mode.

Le seul etat implicite est le systeme de fichiers (les fichiers d'entree doivent exister), ce qui est inevitable et minimal.

---

## Recommandations prioritaires

### 1. Ajouter le support stdin via `-` (Impact: eleve)

**Justification :** Un agent generant du Markdown a la volee ne devrait pas avoir a creer un fichier temporaire.

**Effort :** Faible (~10 lignes dans `main.rs`).

```rust
use std::io::Read;

let markdown = if input == Path::new("-") {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)
        .map_err(|e| format!("Failed to read from stdin: {}", e))?;
    buf
} else {
    fs::read_to_string(input)
        .map_err(|e| format!("Failed to read input file '{}': {}", input.display(), e))?
};
```

### 2. Differencier les codes de sortie (Impact: eleve)

**Justification :** Un agent ne peut pas brancher son comportement sans parser le texte d'erreur.

**Proposition :**

| Code | Signification |
|------|--------------|
| 0 | Succes |
| 1 | Erreur generale (reserve) |
| 2 | Erreur d'usage / arguments (gere par clap) |
| 10 | Fichier d'entree introuvable |
| 11 | Fichier de bibliographie introuvable ou invalide |
| 12 | Fichier de style introuvable ou invalide |
| 13 | Reference citee non trouvee dans la bibliographie |
| 14 | Erreur de traitement CSL |

**Effort :** Moyen -- refactoriser `run()` pour retourner un type d'erreur enumere, puis mapper vers le code de sortie dans `main()`.

### 3. Aligner la spec avec l'implementation (Impact: moyen)

**Justification :** Un agent ou un utilisateur qui lit `overview.md` et tente `--format html` obtiendra une erreur inattendue.

**Options :**
- (a) Implementer `--format` et `--locale` dans `process`
- (b) Mettre a jour `overview.md` pour marquer ces options comme Phase 2+
- (c) Les deux

L'option (b) est la plus rapide et la plus honnete.

### 4. Corriger le bug de double en-tete de bibliographie (Impact: moyen)

**Justification :** Un document bien forme ne devrait pas avoir deux en-tetes de bibliographie.

**Proposition :** Detecter si le contenu Markdown contient deja un en-tete correspondant a `--bib-header` (ou un pattern `## Ref*` / `## Bib*`) et le remplacer au lieu d'en ajouter un nouveau. Alternativement, documenter clairement que l'utilisateur doit retirer l'en-tete de bibliographie de son document source.

### 5. Harmoniser la sortie stdout vs fichier (Impact: faible)

**Justification :** La coherence entre les deux modes de sortie est attendue par un agent.

**Correction :** Remplacer `writeln!` par `write!` dans `main.rs` pour eviter le `\n` supplementaire en mode stdout, ou ajouter un `\n` final dans le mode fichier.

```rust
// Au lieu de:
writeln!(handle, "{}", result)?;
// Utiliser:
write!(handle, "{}", result)?;
```

---

## Annexe : Trace complete des tests

### Benchmarks (build release)

```
Basic process (stdout):    real 0m0.013s
Process (output file):     real 0m0.014s
Help text:                 real 0m0.010s
Process subcommand help:   real 0m0.012s
Version:                   real 0m0.010s
Builtin style:             real 0m0.013s
100 citations:             real 0m0.017s
```

### Idempotence

```
Run 1 exit code: 0
Run 2 exit code: 0
Diff: IDENTICAL
File output: IDENTICAL
```

### Codes de sortie

```
Success:         0
Missing args:    2
Bad subcommand:  2
File not found:  1
Ref not found:   1
Invalid JSON:    1
Invalid style:   1
```

### Support stdin

```
Via '-':        ECHOUE (Error: Failed to read input file '-')
Via /dev/stdin: FONCTIONNE (Linux uniquement)
```

### Separation stdout/stderr

```
Succes:  stdout = contenu, stderr = vide
Erreur:  stdout = vide, stderr = message d'erreur
```
