# Flow — revue maintenabilité / chaîne d'approvisionnement (G10), **agent, non indépendante**

**Déclaration d'indépendance — à lire avant le verdict.** Auteur : agent `flow-docs`, équipier de
l'équipe qui a produit le code examiné (même Lead, même dépôt, même session de travail). Ce n'est **ni**
un humain, **ni** un relecteur extérieur à l'équipe : **l'exigence d'indépendance de `G10` n'est donc
pas satisfaite par ce document**, qui ne peut pas clore la porte. Il est produit comme *revue interne
adversariale supplémentaire*, pas comme « second reviewer indépendant ».

Portée : revue **statique** uniquement (aucun build, test, benchmark, installation, CI, ni accès à
`Rwanbt/Ravum`). Tête examinée : `2ad5bf5` (PR #54, brouillon) ; la branche a avancé à `8d21b7b`
pendant la revue — signalé, non revu. Sources lues : `crates/flow/**`, `docs/**`, `.github/workflows/`,
`dist/**`, PR #51–#54, issue #7.

## 1. Verdict

| Élément | Verdict |
|---|---|
| Indépendance du reviewer (`G10`) | **`NOT_ACHIEVED`** |
| Maintenabilité | **`PARTIAL`** (structure saine, concentration sur `commands.rs`, dette de documentation) |
| Chaîne d'approvisionnement | **`PARTIAL`** (bonnes pratiques présentes ; artefacts de livraison périmés et non versionnés) |
| Verdict produit | **`PRODUCTION_PARTIAL`** conservé ; **aucun `GO_PRODUCTION`** |

## 2. Prémisse de la carte vs état réel

La carte cite le rapport de porte au blob `4680a03`, où `G10` était `NOT_RUN` (« No second agent was
available in this wave »). **Cette prémisse est périmée** : le commit `0263c35` a produit
[FLOW-PRODUCTION-SECOND-REVIEW.md](FLOW-PRODUCTION-SECOND-REVIEW.md) (relecteur : « independent agent,
no prior context », tête `ccad5e0`, 18 constats : 1 P1, 8 P2, 9 P3), et le rapport de porte actuel
décrit `G10` comme « executed ».

Vérification statique des dispositions de cette revue (présence dans l'arbre actuel) :

| ID | Attendu | Présent ? |
|---|---|---|
| R2-01 | refus d'une valeur inline sur un drapeau booléen + test | oui (`cli.rs:1689`, gate CI `flow-ci.yml:177`) |
| R2-02 | fsync du répertoire parent sur Unix | oui (`state::parent_to_flush`, `state.rs:395`) |
| R2-03 | CI sur `dev` et `main` | oui (`flow-ci.yml:15-18`) |
| R2-04 | toolchain épinglée 1.95.0 | oui (`rust-toolchain.toml`, `flow-ci.yml:50`) |
| R2-05 | gates qui échouent fermé | oui (gates journal-symlink/link-containment dans `flow-ci.yml`) |
| R2-06 | un seul compte de tests par plateforme, tenu par un plancher | oui (`minimum=470`, Windows 468) |
| R2-07 | README honnête sur le verdict | oui (`README.md:2`, `PRODUCTION_PARTIAL`) |
| R2-08/09 | `vars_os`, liveness Linux via `/proc` | oui (`gitcli.rs:337-354`, `lock.rs:358`) |

Ce qui **reste** ouvert n'est donc pas « la seconde revue » mais **l'attestation d'indépendance** : la
revue existante est un agent du même projet. Un texte de modèle répété ne compte pas comme preuve
indépendante (`AGENTS.md`).

## 3. Constats de cette revue (nouveaux, à la tête examinée)

| ID | Sév. | Constat | Preuve | Recommandation |
|---|---|---|---|---|
| N1 | **P2** | Les artefacts de `dist/` sont **périmés** : `PROVENANCE.json` épingle `f5bb271…`, pas la tête candidate (`2ad5bf5`/`8d21b7b`), et le SBOM date du 2026-11-10 | `dist/…/PROVENANCE.json` `source.commit` ; `SBOM.json` `metadata.timestamp` | régénérer les artefacts sur la tête qui serait publiée, et **consigner le hash dans le dépôt** |
| N2 | P2 | `dist/` est **ignoré** par git (`.gitignore` `dist/`) : provenance, SBOM et sommes ne sont pas versionnés ; la chaîne de preuve dépend d'un répertoire local | `git ls-files dist` → 0 | committer un manifeste de livraison (ou le publier comme artefact CI signé) |
| N3 | P3 | `SHA256SUMS.txt` ne couvre que deux fichiers (`flow.exe`, `LICENSE`) ; `PROVENANCE.json` et `SBOM.json` ne sont pas dans la liste d'intégrité | contenu du fichier | inclure tous les fichiers livrés, ou documenter pourquoi les métadonnées en sont exclues |
| N4 | P3 | Dérive documentaire : `D4` vaut `PARTIAL` au registre et `NOT GIVEN` en V6 ; les compteurs de couverture ont déjà dû être corrigés deux fois (vagues 5 et 8) | `DECISION_REGISTER.md` vs V6 §7.2 | une **source unique** de statut, citée par les autres documents |
| N5 | P3 | Cible mouvante : la tête a avancé pendant la revue (`2ad5bf5` → `8d21b7b`) ; PR #54 est empilée sur `feat/flow-mvp-0.1` (3 PR au-dessus de `dev`) | `git branch -avv` ; PR #54 `baseRefName` | figer la tête avant toute revue de porte ; une revue épingle un SHA |
| N6 | P3 | Concentration de maintenance : `commands.rs` = 2 917 lignes (≈31 % du produit), 10 modules, 205 occurrences `unwrap`/`expect` (test inclus, non séparées ici) | comptage statique | extraire des sous-modules de `commands.rs` ; séparer produit/test dans la métrique |
| P1 | — | **Positif** : zéro dépendance (`Cargo.lock` = 1 entrée), zéro bloc `unsafe`, `Cargo.lock` et `rust-toolchain.toml` versionnés, `MPL-2.0` cohérente (LICENSE, `Cargo.toml`, métadonnée GitHub) | fichiers | — |
| P2 | — | **Positif** : chaîne CI saine — actions épinglées par SHA complet avec commentaire de version, `permissions: contents: read`, `concurrency` annulante, toolchain nommée explicitement | `flow-ci.yml:25,45-50,437` | — |
| P3 | — | **Positif** : `unsafe` absent, `TODO/FIXME` absents, SBOM CycloneDX 1.5 valide (composants vides = cohérent avec zéro dépendance) | greps | — |

## 4. Ce qui clôturerait réellement `G10`

1. Un relecteur **hors équipe** (humain, ou agent d'un autre fournisseur/modèle sans contexte partagé —
   ce dernier restant non humain) signe l'axe maintenabilité / chaîne d'approvisionnement.
2. L'attestation porte : identité, SHA examiné, versions d'outils, sources lues, et la déclaration
   explicite que **le texte d'un autre modèle n'est pas une preuve indépendante**.
3. Les constats N1–N2 sont corrigés au SHA qui serait publié (sinon la livraison n'a pas de provenance
   versionnée).

## 5. Limites

- Revue statique : aucun test, aucune exécution, donc aucune confirmation dynamique des constats.
- Reviewer interne : biais d'auteur possible ; ce document ne remplace pas une revue indépendante.
- Le comptage `unwrap`/`expect` inclut les modules de test ; il n'est pas une mesure produit.
- `Rwanbt/Ravum` non examiné.

## Annexe — liens

[rapport de porte](FLOW-PRODUCTION-GATE-REPORT.md) ·
[seconde revue existante](FLOW-PRODUCTION-SECOND-REVIEW.md) · [vague 8](FLOW-QUALITY-WAVE8.md) ·
[CAPABILITIES](../CAPABILITIES.md) · [registration](../DECISION_REGISTER.md) ·
[release checklist](FLOW-PRODUCTION-RELEASE-CHECKLIST.md) · PR #54
