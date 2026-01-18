# Plan TDD - Support des Citations Groupees

## Resume du Probleme

### Comportement actuel

Quand plusieurs citations sont adjacentes dans le Markdown:

```markdown
Several studies [@pmid:1](url1) [@pmid:2](url2) [@pmid:3](url3).
```

Le resultat avec un style numerique (Radiology) est:

```markdown
Several studies (1) (2) (3).
```

### Comportement attendu

```markdown
Several studies (1-3).
```

Ou si les numeros ne sont pas consecutifs:

```markdown
Several studies (1, 3-6).
Several studies (1-2, 4, 6-9).
```

### Cause technique

Dans `src/processor.rs`, chaque citation cree son propre "cluster" CSL. CSL attend plusieurs items dans un seul cluster pour les fusionner correctement.

---

## Solution de Design: Pre-processing des Citations Adjacentes

### Principe

L'utilisateur ecrit son document de travail avec les liens pour pouvoir naviguer vers ses sources:

```markdown
Several studies [@pmid:1](url1) [@pmid:2](url2) [@pmid:3](url3) show that...
```

Au processing, on:
1. Detecte les citations adjacentes (separees uniquement par des espaces)
2. Les fusionne en un seul cluster CSL
3. Ignore les URLs pour le rendu final

Resultat: `(1-3)` proprement groupe (ou `(1, 3-6)` si non consecutifs).

### Avantages

- **Document de travail navigable**: Les liens restent cliquables dans l'editeur
- **Sortie finale propre**: Groupement CSL correct
- **Une seule syntaxe a apprendre**: Pas besoin de `[@a; @b; @c]` pour l'utilisateur
- **Compatible Pandoc**: La syntaxe `[@a; @b]` reste supportee aussi

### Definition de "adjacent"

- Citations collees ou separees uniquement par des espaces/whitespace: **groupees**
  - `[@a][@b]` → groupees (collees)
  - `[@a](url)[@b](url)` → groupees (collees avec URLs)
  - `[@a] [@b]` → groupees (espace)
  - `[@a](url) [@b](url)` → groupees (espace avec URLs)
- Citations separees par du texte ou ponctuation: **pas groupees**
  - `[@a], [@b]` → pas groupees (virgule = separateur intentionnel)
  - `[@a] and [@b]` → pas groupees (texte entre)
  - `[@a]. [@b]` → pas groupees (point = nouvelle phrase)

### Deux syntaxes supportees (mutuellement equivalentes)

1. **Syntaxe avec URLs (document de travail):**
   ```markdown
   [@ref1](url1) [@ref2](url2) [@ref3](url3)
   ```
   Pre-processe vers un cluster groupe, URLs ignorees au rendu.

2. **Syntaxe Pandoc standard (sans URLs):**
   ```markdown
   [@ref1; @ref2; @ref3]
   ```
   Directement parsee comme un cluster groupe.

Les deux produisent le meme resultat: `(1-3)`.

---

## Edge Cases pour le Groupement

### Citations non consecutives dans la numerotation

L'utilisateur cite:
```markdown
[@ref1](url) [@ref3](url) [@ref4](url) [@ref5](url) [@ref6](url)
```

Si `ref1` = numero 1, `ref3` = numero 3, etc., le resultat sera:
```
(1, 3-6)
```

### Citations dans le desordre

L'utilisateur cite:
```markdown
[@ref3](url) [@ref1](url) [@ref2](url)
```

CSL trie automatiquement selon le style. Resultat:
```
(1-3)
```

### Mix de groupes et citations isolees

```markdown
First [@a](url) [@b](url) and then [@c](url) separately, then [@d](url) [@e](url).
```

Trois clusters:
1. `[@a; @b]` → `(1-2)` ou `(1, 2)` selon style
2. `[@c]` → `(3)`
3. `[@d; @e]` → `(4-5)` ou `(4, 5)` selon style

Resultat: `First (1-2) and then (3) separately, then (4-5).`

### Citations avec locators

```markdown
[@book1, p. 10](url) [@book2, ch. 3](url)
```

Les locators sont preserves. CSL gere le formatage selon le style.

---

## Structures de Donnees

### Structure actuelle

```rust
pub struct Citation {
    pub id: String,
    pub locator: Option<String>,
    pub label: Option<String>,
    pub url: Option<String>,
    pub span: (usize, usize),
}
```

### Nouvelle structure

```rust
/// Un element de citation individuel (un seul @id)
#[derive(Debug, Clone, PartialEq)]
pub struct CitationItem {
    pub id: String,
    pub locator: Option<String>,
    pub label: Option<String>,
    pub url: Option<String>,  // Preserve pour reference, ignore au rendu groupe
}

/// Un groupe de citations (un ou plusieurs items dans un seul cluster)
#[derive(Debug, Clone, PartialEq)]
pub struct CitationCluster {
    pub items: Vec<CitationItem>,
    pub span: (usize, usize),  // Couvre tout le groupe dans le texte source
}
```

---

## Tests TDD - Suite de Tests

### Tests de parsing des citations adjacentes

```rust
/// Test 1: Citations adjacentes detectees comme groupe
#[test]
fn test_adjacent_citations_grouped() {
    let markdown = "Studies [@a] [@b] [@c] show that...";
    let clusters = extract_citation_clusters(markdown);

    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].items.len(), 3);
    assert_eq!(clusters[0].items[0].id, "a");
    assert_eq!(clusters[0].items[1].id, "b");
    assert_eq!(clusters[0].items[2].id, "c");
}

/// Test 2: Citations adjacentes avec URLs (espace)
#[test]
fn test_adjacent_citations_with_urls_grouped() {
    let markdown = "Studies [@a](url1) [@b](url2) [@c](url3) show...";
    let clusters = extract_citation_clusters(markdown);

    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].items.len(), 3);
    // URLs preservees dans les items
    assert_eq!(clusters[0].items[0].url, Some("url1".into()));
}

/// Test 2b: Citations collees sans espace
#[test]
fn test_adjacent_citations_no_space_grouped() {
    let markdown = "Studies [@a][@b][@c] show that...";
    let clusters = extract_citation_clusters(markdown);

    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].items.len(), 3);
}

/// Test 2c: Citations collees avec URLs sans espace
#[test]
fn test_adjacent_citations_with_urls_no_space_grouped() {
    let markdown = "Studies [@a](url1)[@b](url2)[@c](url3) show...";
    let clusters = extract_citation_clusters(markdown);

    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].items.len(), 3);
    assert_eq!(clusters[0].items[0].url, Some("url1".into()));
    assert_eq!(clusters[0].items[1].url, Some("url2".into()));
}

/// Test 3: Citations separees par ponctuation = pas groupees
#[test]
fn test_citations_separated_by_punctuation_not_grouped() {
    let markdown = "First [@a], then [@b].";
    let clusters = extract_citation_clusters(markdown);

    assert_eq!(clusters.len(), 2);
    assert_eq!(clusters[0].items.len(), 1);
    assert_eq!(clusters[1].items.len(), 1);
}

/// Test 4: Citations separees par texte = pas groupees
#[test]
fn test_citations_separated_by_text_not_grouped() {
    let markdown = "See [@a] and also [@b].";
    let clusters = extract_citation_clusters(markdown);

    assert_eq!(clusters.len(), 2);
}

/// Test 5: Mix groupees et isolees
#[test]
fn test_mixed_grouped_and_isolated() {
    let markdown = "First [@a] [@b] and then [@c] separately.";
    let clusters = extract_citation_clusters(markdown);

    assert_eq!(clusters.len(), 2);
    assert_eq!(clusters[0].items.len(), 2);  // groupe
    assert_eq!(clusters[1].items.len(), 1);  // isole
}

/// Test 6: Syntaxe Pandoc [@a; @b] aussi supportee
#[test]
fn test_pandoc_syntax_grouped() {
    let markdown = "Studies [@a; @b; @c] show that...";
    let clusters = extract_citation_clusters(markdown);

    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].items.len(), 3);
}

/// Test 7: Locators preserves dans les groupes
#[test]
fn test_grouped_with_locators() {
    let markdown = "See [@book1, p. 10] [@book2, ch. 3] for details.";
    let clusters = extract_citation_clusters(markdown);

    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].items[0].locator, Some("10".into()));
    assert_eq!(clusters[0].items[1].locator, Some("3".into()));
}
```

### Tests de formatage CSL

```rust
/// Test 8: Formatage numerique consecutif -> fusion (1-3)
#[test]
fn test_format_numeric_consecutive() {
    // refs numerotes 1, 2, 3
    // cluster avec ces 3 refs
    // style numerique avec collapse
    // Resultat attendu: contient "1-3" ou "1, 2, 3"
}

/// Test 9: Formatage numerique non-consecutif -> (1, 3-6)
#[test]
fn test_format_numeric_non_consecutive() {
    // refs numerotes 1, 3, 4, 5, 6
    // cluster avec ces refs
    // Resultat attendu: contient "1, 3-6" ou "1, 3, 4, 5, 6"
}

/// Test 10: Formatage avec gaps multiples -> (1-2, 4, 6-9)
#[test]
fn test_format_numeric_multiple_gaps() {
    // refs numerotes 1, 2, 4, 6, 7, 8, 9
    // Resultat attendu: contient les numeros avec gaps geres
}

/// Test 11: Style auteur-date
#[test]
fn test_format_author_date_grouped() {
    // Resultat attendu: (Smith, 2020; Jones, 2021)
}
```

### Tests de regression

```rust
/// Test 12: Citation simple toujours fonctionne
#[test]
fn test_simple_citation_still_works() {
    let markdown = "See [@ref1].";
    let clusters = extract_citation_clusters(markdown);

    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].items.len(), 1);
}

/// Test 13: URL preservee pour citation isolee
#[test]
fn test_isolated_citation_url_preserved() {
    let markdown = "See [@ref1](https://doi.org/xxx).";
    let clusters = extract_citation_clusters(markdown);

    assert_eq!(clusters[0].items[0].url, Some("https://doi.org/xxx".into()));
}
```

---

## Plan d'Implementation par Phases

### Phase 1: Tests RED (15 min)

**Objectif:** Creer les tests qui echouent pour documenter le comportement attendu.

**Fichiers a creer:**
- `tests/citation_grouping.rs`

**Actions:**
1. Creer les 15 tests decrits ci-dessus (incluant 2b et 2c pour les citations collees)
2. Executer `cargo test citation_grouping` - DOIT ECHOUER

**Verification:** Les tests compilent mais echouent.

---

### Phase 2: Refactorer les structures (20 min)

**Objectif:** Introduire `CitationItem` et `CitationCluster` sans casser l'existant.

**Fichiers a modifier:**
- `src/markdown.rs` - Renommer `Citation` → `CitationItem`, creer `CitationCluster`
- `src/lib.rs` - Mettre a jour les exports
- `src/processor.rs` - Adapter les signatures
- `src/output.rs` - Adapter les signatures

**Actions:**
1. Renommer `Citation` en `CitationItem`
2. Creer `CitationCluster { items: Vec<CitationItem>, span }`
3. Adapter `extract_citations()` pour retourner `Vec<CitationCluster>` (1 item par cluster pour l'instant)
4. Adapter le reste du code

**Verification:** `cargo test` - tous les tests existants passent.

---

### Phase 3: Parser les citations adjacentes (30 min)

**Objectif:** Detecter et grouper les citations adjacentes.

**Fichiers a modifier:**
- `src/markdown.rs`

**Actions:**
1. Extraire toutes les citations individuelles avec leurs positions
2. Post-traitement: detecter les citations adjacentes (separees uniquement par whitespace)
3. Fusionner les citations adjacentes en clusters

**Algorithme:**
```
1. Extraire toutes les citations avec span (start, end)
2. Trier par position
3. Pour chaque citation:
   - Si le texte entre cette citation et la precedente est vide ou uniquement whitespace
     → Ajouter au cluster courant
   - Sinon → Nouveau cluster
4. Retourner les clusters
```

**Note:** "vide" = citations collees `[@a][@b]`, "whitespace" = `[@a] [@b]`

**Verification:** Tests `test_adjacent_*` et `test_mixed_*` passent.

---

### Phase 4: Parser la syntaxe Pandoc [@a; @b] (15 min)

**Objectif:** Supporter aussi la syntaxe standard Pandoc.

**Fichiers a modifier:**
- `src/markdown.rs`

**Actions:**
1. Nouvelle regex pour `[@id1; @id2; @id3]`
2. Parser les items separes par `;`
3. Integrer avec le parsing existant

**Verification:** Test `test_pandoc_syntax_grouped` passe.

---

### Phase 5: Formatage avec clusters CSL (30 min)

**Objectif:** Envoyer les clusters corrects a csl_proc.

**Fichiers a modifier:**
- `src/processor.rs`

**Actions:**
1. Modifier `format_citations()` pour utiliser `CitationCluster`
2. Format JSON pour csl_proc:
   ```json
   [[{"id": "a"}, {"id": "b"}, {"id": "c"}]]
   ```
   au lieu de:
   ```json
   [[{"id": "a"}], [{"id": "b"}], [{"id": "c"}]]
   ```

**Verification:** Tests `test_format_*` passent.

---

### Phase 6: Integration et remplacement (20 min)

**Objectif:** Remplacer correctement les citations groupees dans le texte.

**Fichiers a modifier:**
- `src/output.rs`

**Actions:**
1. Un cluster = un seul remplacement
2. Le span couvre de la premiere a la derniere citation du groupe
3. URLs ignorees pour les groupes (pas de lien sur "(1-3)")

**Verification:** Tests d'integration end-to-end.

---

### Phase 7: Documentation et exemples (10 min)

**Actions:**
1. Mettre a jour `article_advanced.md` avec des exemples groupes
2. Mettre a jour `overview.md` avec la nouvelle syntaxe
3. Ajouter des exemples dans le README

---

## Recapitulatif

| Phase | Description | Fichiers | Verification |
|-------|-------------|----------|--------------|
| 1 | Tests RED | tests/citation_grouping.rs | Tests echouent |
| 2 | Structures | markdown.rs, lib.rs, processor.rs, output.rs | Tests existants passent |
| 3 | Parser adjacentes | markdown.rs | Tests adjacent_* passent |
| 4 | Parser Pandoc | markdown.rs | Test pandoc_* passe |
| 5 | Format clusters | processor.rs | Tests format_* passent |
| 6 | Integration | output.rs | Tests E2E passent |
| 7 | Documentation | docs/, exemples/ | Manuel |

---

## Commandes pour chaque phase

### Phase 1
```
Implemente la phase 1: Cree tests/citation_grouping.rs avec les 15 tests decrits.
Les tests DOIVENT compiler mais echouer.
Verification: cargo test citation_grouping
```

### Phase 2
```
Implemente la phase 2: Refactore Citation -> CitationItem et cree CitationCluster.
Adapte tout le code pour utiliser ces nouvelles structures.
Les tests existants DOIVENT continuer de passer.
Verification: cargo test
```

### Phase 3
```
Implemente la phase 3: Detecte et groupe les citations adjacentes.
Citations collees ou separees uniquement par whitespace = groupees.
Verification: cargo test test_adjacent
```

### Phase 4
```
Implemente la phase 4: Supporte la syntaxe Pandoc [@a; @b; @c].
Verification: cargo test test_pandoc
```

### Phase 5
```
Implemente la phase 5: Envoie les clusters corrects a csl_proc.
Un cluster avec plusieurs items = un array avec plusieurs objets.
Verification: cargo test test_format
```

### Phase 6
```
Implemente la phase 6: Remplace correctement les groupes dans le texte.
Verification: cargo test (tous les tests)
```

---

## Annexe: Format csl_proc

### Format actuel (incorrect pour groupement)
```json
[[{"id": "a"}], [{"id": "b"}], [{"id": "c"}]]
```
3 clusters, 1 item chacun → `(1) (2) (3)`

### Format correct pour groupement
```json
[[{"id": "a"}, {"id": "b"}, {"id": "c"}]]
```
1 cluster, 3 items → `(1-3)` ou `(1, 2, 3)` selon le style

### Mix groupe + isole
```json
[[{"id": "a"}, {"id": "b"}], [{"id": "c"}]]
```
2 clusters: premier avec 2 items, second avec 1 → `(1-2)` et `(3)`
