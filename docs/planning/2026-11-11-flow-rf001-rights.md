# Flow — RF-001 : décisions déjà consignées et droits restants (inventaire, **BLOCKED FOR IMPLEMENTATION**)

Statut : **inventaire documentaire.** Aucune décision normative n'est prise ici ; `H2` reste
partiellement `OPEN` (MPL-2.0 choisie, réutilisation externe non tranchée). `RF-001` est
`BLOCKED FOR IMPLEMENTATION` : ouvrir ou lire cette fiche n'autorise ni code, ni exécution sur Ravum, ni
modification de licence.

Base : `Rwanbt/Ravum-flow` @ `feat/flow-production-candidate` `2ad5bf5`, 2026-11-11. Aucun test, build
ou extraction n'a été exécuté ; `Rwanbt/Ravum` n'a pas été ouvert.

## 1. Nom, visibilité et licence — qualifiés, sans nouveau choix

| Fait | Valeur observée | Comment | Étiquette |
|---|---|---|---|
| Nom réel du dépôt | `Ravum-flow` (`Rwanbt/Ravum-flow`) | `gh repo view` | `MEASURED` (API) |
| Visibilité | **`PUBLIC`** | idem | `MEASURED` (API) |
| Branche par défaut | `main` | idem | `MEASURED` (API) |
| Licence déclarée par la forge | `MPL-2.0` (Mozilla Public License 2.0) | `licenseInfo` de l'API | `MEASURED` (API) |
| Fichier `LICENSE` | texte MPL-2.0 complet | lecture | `SOURCE_READ` |
| Licence du crate | `license = "MPL-2.0"`, `description = "… (no dependencies)"` | `crates/flow/Cargo.toml` | `CODE_READ` |
| Décision `H2` | **HUMAN_DECIDED 2026-11-09** : MPL-2.0 pour le matériel Flow original (PR #49 `8071f26` sur `main`, #50 `63d5f98` sur `dev`, **fusionnées**) ; copie/dérivation externe **OPEN** | registre + `gh pr view` | `SOURCE_READ` |

Cette fiche **ne rouvre pas** ce choix et **ne republie rien** : elle constate.

## 2. Décisions réellement consignées (état, pas décision)

| ID | État consigné | Preuve | Ne pas rouvrir ? |
|---|---|---|---|
| `H2` | MPL-2.0 pour le matériel original ; réutilisation externe `OPEN` | PR #49/#50 fusionnées | oui (constat) |
| `H5` | GO borné du 2026-11-09 **consommé** | issue #5, commentaire du 2026-11-09 ; [qualification H5](2026-11-11-flow-h5-consumed-go.md) | oui (constat) |
| `H1`, `H3`, `H4`, `H6`, `H7`, `H8` | `OPEN` | [registre](../DECISION_REGISTER.md) | — |
| 13 ADR Flow (`FLOW-ADR-000..011`) | `PROPOSED` | [adr/](../adr/README.md) | oui (aucun `ACCEPTED`) |
| 42 ADR Ravum (`RAVUM-ADR-000..041`) | `PROPOSED`, reproduits en V6 | V6 | oui |
| `D0` / `D1` / `D2` / `D3` / `D4` | `PARTIAL` / `DOCUMENT-CONDITIONAL` / `PROPOSED` / `DRAFT` / `PARTIAL` (V6 §7.2 écrit `NOT GIVEN` : écart constaté) | registre, V6 | — |

## 3. Provenance, auteurs et dépendances (mesuré le 2026-11-11)

| Question | Fait | Étiquette |
|---|---|---|
| Qui a contribué ? | `git shortlog`: **une identité humaine** `Rwanbt <barat.erwan@gmail.com>` (131) + `Erwan Barat <barat.erwan@gmail.com>` (17) — même adresse, donc **un seul titulaire** ; les autres entrées sont des identités d'agents créées par les sessions (`Flow Agent`, `Flow Quality`, `qa`, `Flow Wave 6`, adresses `@example.invalid`) | `MEASURED` (git) |
| Dépendances tierces | `Cargo.lock` ne contient **que le crate `flow`** ; `[dependencies]` vide → aucune bibliothèque tierce embarquée | `MEASURED` (fichiers) |
| Code dérivé de Ravum ? | aucune référence `ravum::` / `use ravum` dans `crates/flow/src` et `tests` ; aucun répertoire `vendor/` ; les seuls répertoires racine sont `crates`, `docs`, `examples`, `experiments`, `scripts`, `dist`, `.github` | `CODE_READ` |
| En-têtes de copyright tiers ? | aucun `Copyright`/`SPDX` dans les sources (la licence est portée par `LICENSE` et `Cargo.toml`) | `CODE_READ` |
| Code Ravum lui-même | **non audité ici** : lecture seule, hors périmètre ; ancre V6 `5408d9b3…` non revérifiée | `UNKNOWN` |

**Limite de provenance** : une adresse e-mail n'est pas un acte d'attribution de droits. L'affirmation
« un seul titulaire » repose sur l'historique git et sur la déclaration du registre, pas sur un
document signé. Les contributions générées par agents appartiennent, en droit d'auteur applicable, à
celui qui les a commandées — question à confirmer par l'humain, pas par ce document.

## 4. Droits restants, renvoyés à `H2`

| Sujet | État | Ce qu'il faudrait | Étiquette |
|---|---|---|---|
| Réutilisation de code Ravum (`ravum-snapshot`, `ravum-workspace-coordinator`, `ravum-daemon`, `ravum-protocol`, `ravum-unit-graph`, `ravum-collab-oracle`, `ravum-batch`, `ravum-rust-adapter`) | `OPEN` | pour chaque candidat : SHA amont, signatures d'API, deps, tests, **auteur**, licence/transfert, propriétaire des évolutions ; `ravum-store` **n'est pas** un CAS réutilisable | `PROPOSED` |
| Trois voies | (a) réemploi légal sous licence ; (b) bibliothèque/adaptateur indépendant ; (c) clean-room étroit | aucune ne peut être choisie sans `H2` | `PROPOSED` |
| Déclaration tierce par fichier | inexistante aujourd'hui | registre `fichier → auteur → licence → source` avant tout import | `PROPOSED` |
| Licence des outils comparés (jj, GitButler FSL-1.1-MIT, Scion Apache-2.0…) | non vérifiée dans ce dépôt | audit licence/version pour l'option `O1` | `UNKNOWN` |
| Extraction inter-dépôts | interdite sans ratification explicite (V6 §7.4) | ratification `H2` **avant** toute copie | `SOURCE_READ` |

## 5. Registre des permissions (état)

| Autorisation | Accordée ? | Portée | Consommée ? |
|---|---|---|---|
| `GO_PLANNING` | oui (docs/issues/recherche) | documents uniquement | en cours |
| `GO_R0_FIXTURES` | oui, 2026-11-09, bornée | fixtures synthétiques jetables, canaris fictifs | oui |
| Plus petit noyau utile | oui, via le même GO | Flow seulement, tests/CI/PR brouillons | oui |
| `GO_PRODUCTION` | **non** | — | — |
| Écriture sur `Rwanbt/Ravum` | **non** (lecture seule) | — | — |
| Modification de licence | **non** (MPL-2.0 déjà choisie, ne pas republier) | — | — |

## 6. Alternatives et contre-exemples (pour la question des droits)

- `O0` (aucune réutilisation) : zéro risque juridique, coût de réimplémentation.
- `O1` (outils existants) : licences hétérogènes (FSL, Apache-2.0, MIT) ; une licence ne rend pas une
  garantie technique ; `O1` n'a pas été audité ici.
- `O2` (Flow original MPL-2.0) : c'est l'état actuel ; aucun code tiers embarqué mesuré.
- `O3` (hybride avec adaptateurs) : acceptable **si** l'interface est un protocole, pas une copie.
- Contre-exemples documentés : `ravum-store` confondu avec un CAS (V6 §1.3) ; V1 `AUTO_SAFE` promu à
  tort (V6 §5.2) ; une extraction non versionnée serait une migration sans double validation.

## 7. Tests spécifiés, `NOT_RUN`

| Cas | Positif | Négatif | Statut |
|---|---|---|---|
| En-têtes/licences | tout fichier source sans `SPDX` est couvert par `LICENSE` + `Cargo.toml` | un fichier portant un en-tête GPL/FSL est refusé par la revue | `NOT_RUN` |
| Dépendances | `Cargo.lock` = 1 entrée (crate `flow`) | l'ajout d'une dépendance sans déclaration échoue | `NOT_RUN` |
| Provenance | chaque commit auteur ∈ {titulaire, identités d'agents déclarées} | un auteur externe inconnu déclenche une revue | `NOT_RUN` |
| Import externe | un import déclaré `fichier → licence → source → SHA` passe | un import sans déclaration échoue | `NOT_RUN` |

Aucun de ces tests n'a été exécuté ; ils sont spécifiés pour une future revue, pas pour cette fiche.

## 8. Faits, recommandation, objections

**Faits** : nom et visibilité `PUBLIC` mesurés ; MPL-2.0 déjà décidée et publiée ; un seul titulaire dans
l'historique ; zéro dépendance ; aucun code Ravum détecté dans les sources ; droits externes `OPEN`.

**Recommandation non contraignante** (`PROPOSED`) : ne rien copier de Ravum avant `H2` ; tenir le
registre par fichier avant tout import ; consigner la question « contributions d'agents » comme point
d'attention juridique. Objections : (a) le coût d'un registre par fichier est différé ; (b) « un seul
titulaire » reste une inférence ; (c) `O1` n'est pas audité, donc l'alternative n'est pas comparable.

## 9. Limites

- `Rwanbt/Ravum` non ouvert ; aucun fichier amont inspecté.
- Aucune vérification des licences des outils concurrents.
- Aucun test de §7 exécuté ; tout est `NOT_RUN`.
- Document produit par un agent ; aucune valeur juridique.

## Annexe — liens

Issue #7 (RF-001) · #2 (`H2`) · #49/#50 (MPL-2.0) · [registre](../DECISION_REGISTER.md) ·
[RAVUM_BOUNDARY](../RAVUM_BOUNDARY.md) · [TRACEABILITY](../TRACEABILITY.md) · [BACKLOG](../BACKLOG.md) ·
[H5](2026-11-11-flow-h5-consumed-go.md) · [H1](2026-11-11-flow-h1-options.md)
