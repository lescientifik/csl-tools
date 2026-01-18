# Problème: Citations groupées non supportées

## Contexte

`csl-tools` est un CLI qui transforme des documents Markdown contenant des citations (`[@id]`) en documents avec citations formatées et bibliographie, en utilisant `csl_proc` comme moteur CSL.

## Le problème

### Comportement actuel

Actuellement, chaque citation `[@id]` est traitée individuellement. Quand plusieurs citations sont adjacentes:

```markdown
Several studies have explored this topic [@pmid:41524770] [@pmid:41524478] [@pmid:41481737].
```

Le résultat avec un style numérique (ex: Radiology) est:

```markdown
Several studies have explored this topic (1) (2) (3).
```

### Comportement attendu (Zotero/Pandoc)

Dans Zotero et Pandoc, la syntaxe pour grouper des citations utilise le point-virgule:

```markdown
Several studies have explored this topic [@pmid:41524770; @pmid:41524478; @pmid:41481737].
```

Le résultat attendu:

```markdown
Several studies have explored this topic (1-3).
```

Ou selon le style:

```markdown
Several studies have explored this topic (1, 2, 3).
```

## Syntaxe CSL/Pandoc pour les citations

### Citations simples
```markdown
[@id]                    → (Author, 2021) ou (1)
[@id, p. 42]             → (Author, 2021, p. 42) ou (1, p. 42)
[@id](url)               → lien préservé (notre extension)
```

### Citations groupées (NON SUPPORTÉ actuellement)
```markdown
[@a; @b]                 → (Author1, 2020; Author2, 2021) ou (1, 2)
[@a; @b; @c]             → (1-3) si consécutifs dans style numérique
[@a, p. 10; @b, ch. 2]   → groupées avec locators individuels
```

## Architecture CSL et csl_proc

### Structure CitationCluster dans csl_proc

```rust
/// Citation cluster for incremental citations format
#[derive(Debug, Deserialize, Clone)]
struct CitationCluster {
    #[serde(rename = "citationID")]
    citation_id: String,
    #[serde(rename = "citationItems")]
    citation_items: Vec<CitationItem>,  // <-- PLUSIEURS items par cluster
    #[serde(default)]
    properties: CitationProperties,
}
```

Un **cluster** peut contenir **plusieurs citation items**. C'est ainsi que CSL gère les citations groupées: un seul cluster avec N items est formaté ensemble, permettant:
- La fusion des numéros consécutifs: `(1, 2, 3)` → `(1-3)`
- Le délimiteur approprié selon le style: `(Author1; Author2)` ou `(1, 2)`
- Le tri interne des citations selon le style

### Implémentation actuelle dans csl-tools

Dans `src/processor.rs`, on crée **un cluster par citation**:

```rust
// Chaque citation devient son propre cluster (PROBLÈME)
let citation_items: Vec<Vec<serde_json::Value>> = citations
    .iter()
    .map(|c| {
        let mut item = serde_json::json!({"id": c.id});
        // ...
        vec![item]  // Un seul item par cluster
    })
    .collect();
```

Cela empêche csl_proc de formater correctement les citations groupées.

## Exemple concret du problème

### Fichier test: `article_advanced.md`

```markdown
## Introduction

Gene editing technologies have transformed modern medicine. Several recent
studies have explored various aspects of CRISPR-Cas9 therapy
[@pmid:41524770] [@pmid:41524478] [@pmid:41481737].
```

### Résultat actuel (style Radiology)

```markdown
...therapy (1) (2) (3).
```

### Résultat attendu si on utilisait `[@a; @b; @c]`

```markdown
...therapy (1-3).
```

## Impact

1. **Lisibilité réduite**: `(1) (2) (3) (4) (5)` vs `(1-5)`
2. **Non-conformité aux standards**: Les journaux scientifiques attendent des citations groupées
3. **Styles numériques particulièrement affectés**: Vancouver, IEEE, Radiology, etc.

## Références

- Syntaxe Pandoc citations: https://pandoc.org/MANUAL.html#citations
- CSL specification: https://docs.citationstyles.org/en/stable/specification.html
- Zotero citation clusters: le même mécanisme est utilisé dans Zotero Word plugin

## Complication supplémentaire: URLs/DOI par citation

### Notre extension `[@id](url)`

On a ajouté une syntaxe non-standard pour permettre des liens DOI cliquables:

```markdown
[@pmid:33024307](https://doi.org/10.1038/s41579-020-00459-7)
```

Cette fonctionnalité est importante car elle permet d'avoir des liens directs vers les articles dans le document final.

### Problème de combinaison

Comment combiner les citations groupées avec les URLs individuelles?

```markdown
# Syntaxe actuelle (citations séparées avec URLs) - FONCTIONNE
[@a](url1) [@b](url2) [@c](url3)

# Syntaxe groupée sans URLs - À IMPLÉMENTER
[@a; @b; @c]

# Syntaxe groupée AVEC URLs - SYNTAXE À DÉFINIR ???
[@a](url1); [@b](url2); [@c](url3)]   # Option 1: URLs dans le groupe?
[@a; @b; @c](???)                      # Option 2: URL unique pour le groupe?
```

### Questions ouvertes

1. **Syntaxe**: Quelle syntaxe pour grouper des citations qui ont chacune leur URL?
2. **Rendu**: Comment afficher les liens quand les citations sont fusionnées en `(1-3)`?
   - Un seul lien sur `(1-3)` pointant où?
   - Trois liens `(1-3)` avec chaque numéro cliquable?
   - Abandonner les liens dans les groupes?
3. **Rétrocompatibilité**: `[@a](url1) [@b](url2)` adjacents doivent-ils être auto-groupés?

### Cas d'usage réel

Dans l'exemple `article.md`:
```markdown
Les coronavirus [@pmid:33024307](https://doi.org/10.1038/s41579-020-00459-7) sont étudiés.
```

Si on veut citer 3 articles avec leurs DOIs ET les grouper:
```markdown
# Actuellement (pas groupé mais liens préservés)
[@pmid:1](https://doi.org/...) [@pmid:2](https://doi.org/...) [@pmid:3](https://doi.org/...)
→ (Author1) (Author2) (Author3)  # chaque parenthèse peut être un lien

# Souhaité (groupé) - mais comment gérer les liens?
[@pmid:1; @pmid:2; @pmid:3]
→ (1-3)  # un seul élément, où mettre 3 liens?
```

## Fichiers concernés

- `src/markdown.rs` - Parser qui ne reconnaît pas `[@a; @b]`
- `src/processor.rs` - Crée un cluster par citation au lieu de grouper
- `src/lib.rs` - Structure `Citation` représente une seule citation, pas un groupe
