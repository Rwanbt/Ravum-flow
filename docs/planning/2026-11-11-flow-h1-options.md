# Flow — H1 : dossier d'options O0/O1/O2/O3 (document de travail, décision **OPEN**)

Statut : **document de travail, non décisionnel.** `H1` reste **OPEN** dans le
[registre des décisions](../DECISION_REGISTER.md). Aucune option n'est ratifiée, recommandée de façon
contraignante, ni implémentée. Aucun code, test, benchmark, licence, PR Ravum ou CI n'a été lancé pour
ce document.

Base : branche `feat/flow-production-candidate`, tête **`2ad5bf5`** (PR #54, brouillon, non fusionnée).
Sources vérifiées le 2026-11-11 ; les SHA de fichiers ci-dessous sont des SHA-256 du fichier tel qu'il
est à cette tête.

> Rappel de gouvernance : une story (`#46`, `#48`) est un **besoin**, pas une autorisation. `D0`–`D4` ne
> déclenchent **aucune** action ; `R0..R7` exigent un `GO(scope, owner, branch/fixture, budget, tests,
> stop_conditions)` humain distinct. Ce dossier ne demande ni ne présume aucun GO.

## 1. Objet

La décision `H1` porte sur l'**architecture provisoire** de Flow : comparer `O0` (ne pas construire),
`O1` (outil existant), `O2` (Flow mince), `O3` (hybride), `O4` (VFS/CRDT/distribué) restant différée.
Critères d'acceptation d'origine (issue #1) : sources primaires, versions, SHA et alternatives vérifiés ;
arguments et coûts comparés ; **recommandation distinguée des faits** ; statut maintenu `OPEN` tant
qu'un humain n'a pas consigné sa résolution ; aucun code, benchmark lourd, licence, PR Ravum ou CI.

Ce que ce document **ne** fait pas : trancher `H1`, produire une mesure de performance (`S9` dépend de
`H4`), évaluer les droits (`H2`), choisir une sandbox (`H3`), ou modifier un ADR.

## 2. Sources primaires

| Source | Version / SHA | Usage | Étiquette |
|---|---|---|---|
| Issue #1 « [DECISION][H1][D0–D4] » | `OPEN`, aucun commentaire | mandat H1 et critères d'acceptation | `SOURCE_READ` |
| Issue #46 « US-013 — distinguer les preuves des opinions d'IA » | `OPEN` | provenance, 42 ADR `PROPOSED`, `NOT_REVIEWED` visible | `SOURCE_READ` |
| Issue #48 « US-015 — voir décisions et permissions » | `OPEN` | `D0–D4` ne déclenchent pas `R0` ; licence/CI/bench soumis à accord humain | `SOURCE_READ` |
| `docs/reviews/RAVUM-FLOW-V6-POST-CLAUDE-REFERENCE.md` | SHA-256 `0946bda5f08feb4a48c275ce00c95c3b2f68f8b01dbb40d227551301eb47f9d0` | §3.1 options, §7 gates, §10.1 décisions humaines | `SOURCE_READ` |
| `docs/DECISION_REGISTER.md` | SHA-256 `eca2e92fe374c9c55df4fb8a8804dcabfc26a8448a44c5b90411397206d69ead` | état `OPEN` de `H1`/`H3`/`H4`/`H6`/`H7`/`H8`, `H2`/`H5` décidés | `SOURCE_READ` |
| `docs/ROADMAP.md` | SHA-256 `a39c1b87b265281dda281b6983dea40b7f32d6ce5695ac6965ed99b19a6277ea` | gates documentaires et phases `R0..R7` | `SOURCE_READ` |
| `docs/adr/README.md` + `FLOW-ADR-000..011` | 13 fichiers, tous `PROPOSED` | décisions Flow non ratifiées | `SOURCE_READ` |
| PR #49 `chore/license-mpl2-main…` | fusionnée, tête `8071f26` | MPL-2.0 sur `main` (`H2` partiellement décidé) | `SOURCE_READ` |
| PR #50 `chore/license-mpl2-dev…` | fusionnée, tête `63d5f98` | MPL-2.0 sur `dev` | `SOURCE_READ` |
| PR #51 `feat/flow-verified-foundation` | brouillon, tête `832f499` | base vérifiée R0/S9 pilote | `SOURCE_READ` |
| PR #52 `fix/flow-foundation-proof-gates` | brouillon, tête `d884e29` | fermeture des portes P0 | `SOURCE_READ` |
| PR #53 `feat/flow-mvp-0.1` | brouillon, tête `ea181d4` | propositions/candidats/publication | `SOURCE_READ` |
| PR #54 `feat/flow-production-candidate` | brouillon, tête `2ad5bf5` | état mesuré 0.2.0-rc.1 (vagues 5–8) | `SOURCE_READ` |
| `Rwanbt/Ravum` | `5408d9b3…` (lecture seule, revue Claude) | frontière de propriété | `SOURCE_READ`, **non revérifié ici** |

Aucune de ces lectures n'est une revue humaine de la V6 : la V6 est
`DOCUMENT-CORRECTED-AFTER-CLAUDE / HUMAN-DECISIONS-PENDING`, sans seconde signature indépendante.

## 3. Les options (telles que définies par la V6 §3.1)

| Option | Définition (V6) | Critère de conservation (V6) | Ce qui est établi |
|---|---|---|---|
| **O0 — ne pas construire Flow** | Git natif + copies privées ou worktrees éphémères + intégration sérialisée + CI existante | **Baseline prioritaire** : si c'est assez sûr/rapide à coût global inférieur, arrêter l'effort VCS | `SOURCE_READ` ; aucune mesure comparative n'existe (`S9` bloquée par `H4`) |
| **O1 — outil existant** | `jj`, GitButler, orchestrateur existant (Scion cité comme référence), relié à Git et à une CI native | licence, offline, Windows/Linux, API et garanties **effectivement testées** | `SOURCE_READ` ; aucune évaluation licence/offline/garanties n'a été faite dans ce dépôt (`UNKNOWN`) |
| **O2 — Flow mince** | propositions `MANAGED_PATCH`, versions candidates, reçus sur tree Git exact, writer Git contrôlé, journal durable, CLI | doit améliorer qualité/coût vérifié vs O0/O1, sans VFS universel | partiellement **implémenté et mesuré** (vagues 5–8, §8) ; pas de comparaison de coût avec O0/O1 |
| **O3 — hybride** | O2 pour agents intégrés + **vue disque privée matérialisée éphémère** pour hooks/builds/outils non intégrés ; aucun worktree persistant obligatoire | repli sûr, coût disque/CPU/RAM acceptable ; pas de promesse d'absence de matérialisation | implémentation partielle (scratch isolé, cf. vague 6) ; sandbox OS non qualifiée (`H3` `OPEN`) |
| **O4 — VFS/CRDT/distribué** | vues FS projetées, édition collaborative, cache distant, multi-tenant | **DEFER** jusqu'à preuve de besoin et projet de sûreté séparé | hors périmètre de la décision ; reste différé |

## 4. Matrice comparative

Les colonnes de preuve sont volontairement vides là où rien n'a été mesuré : c'est le résultat
principal de ce dossier.

| Critère | O0 | O1 | O2 | O3 |
|---|---|---|---|---|
| Preuve disponible **aujourd'hui** | aucune mesure comparative | `UNKNOWN` (aucune évaluation licence/offline) | `MEASURED` partiel (tests, couverture, mutation ; `PRODUCTION_PARTIAL`) | `MEASURED` partiel, sandbox `UNKNOWN` |
| Coût initial restant | 0 | audit licence + intégration | maintenance d'un produit propre | O2 + moteur de matérialisation + qualification sandbox |
| Coût récurrent (16 Go, Windows/Linux) | outillage existant | dépend de l'outil (inconnu) | builds/tests locaux, CI 5 jobs | + disque temporaire par exécution |
| Propriétés de sûreté | celles de Git/CI, non inventoriées ici | non inventoriées | tree exact, refus explicites, journal durable (mesurés, cf. §8) | idem + isolation **non démontrée** (`H3` `OPEN`) |
| Réversibilité | totale | moyenne (dépendance externe) | l'utilisateur garde Git standard (`FLOW_DISABLED`) | moyenne : matérialisation à qualifier |
| Indépendance Ravum | totale | totale | totale (adaptateur opt-in) | totale |
| Échec typique | conflits/coûts d'agents non adressés | rupture d'outil/licence | périmètre produit à maintenir | isolation surestimée |

## 5. Objections (à l'encontre de chaque option)

- **Contre O0** : O0 est la baseline prioritaire, mais elle ne répond pas aux coûts multi-agents décrits
  en V6 §1.1 (conflits, builds inutiles, temps jusqu'au livrable vérifié) ; or **aucune mesure ne
  chiffre ce coût** tant que `S9` n'est pas autorisée (`H4` `OPEN`). O0 ne peut donc être écarté que par
  un argument **non mesuré**, ce que ce dossier s'interdit.
- **Contre O1** : l'argument décisif exigé par la V6 (licence, offline, Windows/Linux, garanties
  testées) n'a pas été instruit ; `O1` ne peut pas être écarté sur la base d'une préférence.
- **Contre O2** : O2 existe déjà en partie et **mesure** ses limites (`PRODUCTION_PARTIAL`, pas de
  sandbox OS, reçu = digest non signé, writer Git externe non fencé, macOS non mesuré). Rien ne prouve
  encore que son coût total soit inférieur à O0/O1.
- **Contre O3** : O3 suppose une isolation que `H3` n'a pas tranchée. Annoncer O3 avant `H3`, c'est
  vendre une garantie non démontrée (interdit par `AGENTS.md` : « no claims of tests run »).
- **Contre O4** : aucune preuve de besoin ; la V6 le diffère explicitement.

## 6. Faits et recommandation (distingués)

**Faits** (`SOURCE_READ`/`MEASURED`, sans interprétation) :

1. `H1` est `OPEN` ; la V6 recommande — sans ratifier — de **comparer `O0`/`O1` d'abord** et de traiter
   `O3` comme hypothèse à éprouver, pas comme vainqueur désigné (V6 §10.1).
2. Aucune mesure comparative O0/O1/O2 n'existe ; `S9` est bloquée par `H4` (seuil/budget non accordés).
3. `O1` n'a fait l'objet d'aucun audit licence/offline/garanties dans ce dépôt (`UNKNOWN`).
4. `O2` est partiellement implémenté et **mesuré** ; ses limites sont documentées (§8).
5. `O3` dépend de `H3`, `OPEN`.
6. Les 42 ADR Ravum reproduits en V6 restent `PROPOSED` ; les 13 ADR Flow restent `PROPOSED` ;
   `NOT_REVIEWED` est visible sur les pièces non relues.

**Recommandation documentaire, non contraignante** (`PROPOSED`) : ne pas choisir `O2` ou `O3` avant
d'avoir instruit `O0`/`O1` sur des critères écrits, et ne pas écarter `O3` avant `H3`. Cette
recommandation reprend la V6 ; elle est **une opinion documentaire**, pas une décision, et elle a ses
propres objections (§5).

## 7. Ce que les mesures 0.2.0-rc.1 apportent au dossier (`MEASURED`)

| Fait mesuré | Valeur | Conséquence pour H1 |
|---|---|---|
| Suite exécutée | 468 Windows / 470 Linux, 0 ignoré | O2 a un socle testé réel, pas une maquette |
| Couverture produit seul | 95,88 % lignes / 93,56 % branches | coût de maintenance d'O2 non nul mais borné |
| Mutation (4 modules, Linux, périmètres bornés) | `state` 95,7 %, `commands` 96,7 %, `gitcli` 83,0 %, `main` 93,9 % (partiel) | qualité mesurée, mais pas de taux crate entière |
| Verdict produit | `PRODUCTION_PARTIAL` | O2 n'est pas qualifiable en production aujourd'hui |
| Sandbox OS (`H3`) | non implémentée, `OPEN` | O3 non démontrable |
| Reçu | **digest, pas signature** | « preuve » reste bornée ; à ne pas surinterpréter |
| Writer Git externe | non fencé (limite documentée) | O2 ne protège pas un `git` concurrent |
| macOS | non mesuré | portabilité O0/O1 non comparée |

Détail : [vague 8](../reviews/FLOW-QUALITY-WAVE8.md), [CAPABILITIES](../CAPABILITIES.md).

## 8. Gates, ADR et statuts (rappel, non modifiés)

- **D0** `PARTIAL` (inventaire/provenance) ; **D1** `DOCUMENT-CONDITIONAL` (revue Claude, sans seconde
  signature) ; **D2** `PROPOSED` ; **D3** `DRAFT` (protocole S9 sans seuil ni budget) ; **D4**
  `PARTIAL`/`NOT GIVEN` selon le document — **écart de statut constaté** entre
  [DECISION_REGISTER](../DECISION_REGISTER.md) (`PARTIAL`) et V6 §7.2 (`NOT GIVEN`) ; il n'est pas
  tranché ici.
- `D0–D4` ne déclenchent pas `R0` (issue #48).
- **42 ADR Ravum** (`RAVUM-ADR-000..041`) : `PROPOSED`, reproduits en V6 ; **13 ADR Flow**
  (`FLOW-ADR-000..011`) : `PROPOSED`, aucun `ACCEPTED`.
- Un reçu Flow est un **digest**, **pas une signature** : la décision `PRF-01` (signature) n'est pas
  prise et n'est pas tranchée par ce dossier.

## 9. Ce qu'il faudrait pour clore H1 (actes humains)

1. Un acteur humain, une date, la contrainte et la preuve : choix provisoire `O0`/`O1`/`O2`/`O3`
   (ou `O4` explicitement différée) consigné dans le registre, **sans** que cela déclenche `R0`.
2. Les objections de §5 levées ou explicitement acceptées (notamment : mesure `O0`/`O1` ou décision
   assumée de ne pas mesurer).
3. `H3` traité avant toute promesse `O3` ; `H4` avant `S9` ; `H2` avant tout code externe réutilisé.
4. Si `O2`/`O3` est retenu : un `GO` ciblé avec scope, branche, budget, tests et conditions d'arrêt.

## 10. Limites de ce dossier

- Documents lus à la tête `2ad5bf5` ; le dépôt peut avancer. Les SHA de PR et de fichiers sont datés.
- Aucune lecture de `Rwanbt/Ravum` n'a été refaite : la revue Claude de `5408d9b3…` est
  `MEASURED_HISTORICAL`, pas re-mesurée.
- Aucun benchmark, aucun test, aucune mesure nouvelle : tout ce qui est chiffré vient des vagues 5–8 et
  est cité comme tel.
- Ce n'est **pas** une revue indépendante : c'est un dossier préparatoire produit par un agent.

## Annexe A — étiquettes

`CODE_READ` (code lu à un SHA), `SOURCE_READ` (document/issue/PR lu), `MEASURED` / `MEASURED_HISTORICAL`
(chiffre produit par une campagne, avec sa date), `PROPOSED` (idée non ratifiée), `UNKNOWN` (non
instruit), `NOT_REVIEWED` (pièce non relue). Un fait non mesuré n'est jamais présenté ici comme mesuré.

## Annexe B — liens

Issue #1 · #46 · #48 · [V6](../reviews/RAVUM-FLOW-V6-POST-CLAUDE-REFERENCE.md) ·
[DECISION_REGISTER](../DECISION_REGISTER.md) · [ROADMAP](../ROADMAP.md) ·
[ADR Flow](../adr/README.md) · [RAVUM_BOUNDARY](../RAVUM_BOUNDARY.md) · [ARCHITECTURE](../ARCHITECTURE.md) ·
[TRACEABILITY](../TRACEABILITY.md) · [vague 8](../reviews/FLOW-QUALITY-WAVE8.md)
