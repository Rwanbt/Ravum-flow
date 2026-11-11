# Flow — H5 : le GO borné du 2026-11-09 est consommé (qualification documentaire, **aucune** nouvelle autorisation)

Statut : **document de qualification**, non décisionnel et non autorisant. Le `GO` du 2026-11-09 a été
**accordé une fois, borné et consommé** ; ce document ne le rouvre pas et n'accorde rien. `H5` reste
`OPEN` dans le [registre](../DECISION_REGISTER.md) au sens de la décision historique, et toute nouvelle
campagne exigerait un `GO(scope, repository, branch, allowed_paths, budgets, evidence,
stop_conditions)` **distinct et humain**.

Base : `feat/flow-production-candidate` @ `2ad5bf5`, 2026-11-11. Aucun test, fixture, build, benchmark
ou canari n'a été lancé pour ce document.

## 1. Sources

| Source | Référence | Étiquette |
|---|---|---|
| Issue #5 « [DECISION][H5][D4] » | `OPEN` ; commentaire `IC_kwDOVoK4B88AAAABkkU6-Q` du 2026-11-09 19:12:15Z (auteur `Rwanbt`) | `SOURCE_READ` |
| [DECISION_REGISTER](../DECISION_REGISTER.md) § « Mandate of 2026-11-09 » | SHA-256 `eca2e92f…` ; `H5` = `HUMAN_DECIDED 2026-11-09` | `SOURCE_READ` |
| [DEEPSEEK_GOAL](../DEEPSEEK_GOAL.md) | GOAL de planification : « le fait d'avoir ce GOAL ne donne pas `GO_R0_FIXTURES` » | `SOURCE_READ` |
| [FLOW-FOUNDATION-VERIFIED-REPORT](../reviews/FLOW-FOUNDATION-VERIFIED-REPORT.md) | R0-a/b/c, sortie `FOUNDATION_PARTIAL` | `MEASURED` (rapport) |
| [FLOW-FOUNDATION-P0-HARDENING-REPORT](../reviews/FLOW-FOUNDATION-P0-HARDENING-REPORT.md) | trois défauts bloquants corrigés (PR #52) | `MEASURED` (rapport) |
| [FLOW-MVP-0.1-VERIFICATION-REPORT](../reviews/FLOW-MVP-0.1-VERIFICATION-REPORT.md) §1 | mandat borné et ce qu'il n'autorise pas | `SOURCE_READ` |
| PR #51 `832f499` · #52 `d884e29` · #53 `ea181d4` · #54 `2ad5bf5` | brouillons, non fusionnées | `SOURCE_READ` (gh) |
| PR #49 `8071f26` · #50 `63d5f98` | MPL-2.0 fusionné — décision `H2` **séparée** | `SOURCE_READ` |

## 2. Ce que le GO du 2026-11-09 a accordé (verbatim du commentaire)

> « documentary consolidation, R0-a/b/c on disposable synthetic fixtures, the smallest useful core
> after the R0 gate, commits on a Flow branch and a draft PR. It did **not** authorize merging,
> releases, Ravum modification, a licence choice, or ADR ratification. […] Any new test campaign,
> fixture corpus or benchmark needs a separate, scoped human GO. »

## 3. Matrice : permission → preuve → consommation

| Permission accordée | Preuve produite | Consommée ? | Résiduel |
|---|---|---|---|
| Consolidation documentaire | V6 + `DEEPSEEK-00..07`, `DEEPSEEK-SUMMARY`, ADR/RFC `PROPOSED` | **oui** | aucune ratification ; `NOT_REVIEWED` maintenu |
| `R0-a/b/c` sur fixtures synthétiques jetables | [rapport vérifié](../reviews/FLOW-FOUNDATION-VERIFIED-REPORT.md) §3 (matrice R0), sortie `FOUNDATION_PARTIAL` | **oui** | les scénarios `NOT_RUN` le restent |
| Plus petit noyau utile après le gate | PR #51 `832f499` (foundation vérifiée), puis PR #52 `d884e29` | **oui** | périmètre produit `PRODUCTION_PARTIAL` |
| Commits sur branche Flow | `feat/flow-verified-foundation`, `fix/flow-foundation-proof-gates`, `feat/flow-mvp-0.1`, `feat/flow-production-candidate` | **oui** | `dev`/`main` intacts |
| PR **brouillon** | #51, #52, #53, #54 (toutes `draft`) | **oui** | aucune fusion |
| Tests et CI | CI 5 jobs, vagues 3–8 | **oui, et au-delà** (voir §4) | voir §4 |
| Fusion, release, protection de branche | — | **non accordé, non consommé** | `/` |
| Modification de `Rwanbt/Ravum` | — | **non accordé, non consommé** | lecture seule seulement |
| Choix de licence | #49/#50 (MPL-2.0) = décision **`H2` séparée** du propriétaire | hors de ce GO | droits d'extraction externe toujours `OPEN` |
| Ratification d'ADR | — | **non accordé, non consommé** | 13 ADR Flow et 42 ADR Ravum `PROPOSED` |

## 4. Correction ancienne vs nouvelle campagne — la distinction qui manque au registre

Le commentaire de #5 énonce deux régimes : la **correction** d'un noyau déjà autorisé (PR #52 est
« a continuation of corrective work on that already-authorized core, not a standing permission ») et la
**nouvelle campagne** (tests, corpus, benchmark), qui exige un `GO` distinct.

| Travail 2026-11-11 | Nature au sens de #5 | Preuve | Statut de gouvernance |
|---|---|---|---|
| Portes P0, fidélité du checkout, gitlink, flake `scratch`, `parent_to_flush` | **correction** du noyau autorisé (défauts reproduits, correctifs minimaux + tests) | rapports P0/vagues 3–8 | dans le prolongement de #52 ; pas une permission permanente |
| Couverture 95/90, campagnes de mutation Windows puis Linux, rejeu adverse, démos polyglottes, artefacts de livraison | **nouvelle campagne** au sens du commentaire (`test campaign`, `fixture corpus`) | vagues 3–8, CI 5 jobs | **non qualifiée par le registre** : l'instruction directe du propriétaire du 2026-11-11 (session) n'a pas été consignée sous la forme `GO(scope, …)` |
| `H1`/`H3`/`H4`/`S9` | décisions ou campagnes explicitement non accordées | — | `OPEN` |

Conséquence : les campagnes 2026-11-11 ont produit des preuves réelles et reproductibles, mais leur
**habilitation** n'est pas traçable dans le registre. Deux issues humaines possibles, à trancher par le
propriétaire : (a) ratification *a posteriori* de ce qui a été exécuté (avec la liste des dépenses
CPU/temps), ou (b) constat d'un écart de procédure et adoption d'un `GO` explicite pour la suite. Ce
document ne choisit ni l'une ni l'autre.

## 5. Demande de GO bornée proposée — **PROPOSED, non approuvée**

Formulaire à compléter/refuser par l'humain ; il ne vaut pas autorisation.

```text
GO(scope            = "quality measurement + hardening of the existing Flow core (no merge, no release)",
   repository       = Rwanbt/Ravum-flow,
   branch           = feat/flow-production-candidate (travail) ; aucune écriture sur dev/main,
   allowed_paths    = crates/flow/**, docs/**, experiments/**, .github/workflows/flow-ci.yml
                      (modifications de CI limitées : planchers, gates, cache — pas de nouvelle
                      permission de publication),
   budget           = CPU/RAM/temps à fixer (référence 16 Go ; pas de campagne concurrente),
   evidence         = commits + rapports docs/reviews/FLOW-*.md + experiments/quality/**,
   stop_conditions  = P0/P1 reproduit, secret réel, écriture Ravum, dépassement budget,
                      besoin d'une décision H1/H2/H3/H4 → arrêt et rapport)
```

Ce que cette demande **n'inclut pas** : fusion, tag, release, protection de branche, choix de licence,
ratification d'ADR, sandbox `H3`, S9 `H4`, écriture `Rwanbt/Ravum`.

## 6. Faits, recommandation et objections

**Faits** : le GO a été accordé une fois et consommé ; PR #52 est une continuation, pas une permission ;
`H5` est `HUMAN_DECIDED` mais la **campagne** de 2026-11-11 n'a pas de `GO` tracé ; `H1`/`H3`/`H4`
restent `OPEN` ; rien n'a été fusionné, tagué ou publié.

**Recommandation non contraignante** (`PROPOSED`) : consigner au registre, en une ligne datée, soit la
ratification *a posteriori* des campagnes 2026-11-11, soit l'écart ; puis adopter le formulaire §5 avant
toute nouvelle campagne. Objections : (a) une ratification *a posteriori* banalise le régime `GO` ;
(b) refuser la qualification laisse des preuves réelles sans habilitation ; (c) le budget §5 n'est pas
chiffré.

## 7. Limites

- Aucune vérification de l'horodatage GitHub au-delà de l'affichage de l'API (`createdAt` du commentaire).
- Le périmètre exact de l'instruction du 2026-11-11 n'existe que dans la session ; il n'est pas cité ici
  comme un `GO` au sens de `H5`.
- Document produit par un agent ; aucune valeur de ratification.

## Annexe — liens

Issue #5 · [DECISION_REGISTER](../DECISION_REGISTER.md) · [DEEPSEEK_GOAL](../DEEPSEEK_GOAL.md) ·
[rapport vérifié](../reviews/FLOW-FOUNDATION-VERIFIED-REPORT.md) ·
[P0 hardening](../reviews/FLOW-FOUNDATION-P0-HARDENING-REPORT.md) ·
[MVP 0.1](../reviews/FLOW-MVP-0.1-VERIFICATION-REPORT.md) ·
[gate report](../reviews/FLOW-PRODUCTION-GATE-REPORT.md) · [H1](2026-11-11-flow-h1-options.md) ·
[H3](2026-11-11-flow-h3-sandbox-comparison.md)
