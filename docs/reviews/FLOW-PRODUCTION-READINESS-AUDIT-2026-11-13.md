# Ravum Flow — audit d'état et de préparation à la production

**Date :** 2026-11-13 (Europe/Paris) · **Branche auditée :** `feat/flow-production-candidate`
**Tête examinée :** `ad1191b` (2026-11-12T01:59+01:00), PR #54 (brouillon, base `feat/flow-mvp-0.1`)
**Autorisation :** `GO_PLANNING` (lecture, recherche, documentation). **Aucune écriture de code, aucun build,
aucun test, aucun benchmark, aucune action Git distante, aucune modification de `Rwanbt/Ravum` n'a été
effectuée par cet audit.**
**Nature :** audit de consolidation et de préparation. Il ne ratifie rien, ne renumérote aucun identifiant
historique (`RF-*`, `US-*`, `H*`, `D*`, `RAVUM-ADR-*`, `FLOW-ADR-*`, `RF-RFC-*`) et n'attribue aucun statut.

---

## 1. Verdict

| Question | Réponse |
|---|---|
| Le produit existe-t-il et fonctionne-t-il ? | **Oui.** CLI `flow` 0.2.0-rc.1, zéro dépendance, ≈6 800 lignes produit, 10 modules, CI verte 5/5 sur la tête. |
| Est-il qualifié au sens de son propre dossier ? | **Partiellement.** Verdict en vigueur `PRODUCTION_PARTIAL`; **1 des 4 motifs de ce verdict est désormais clos**, les trois autres tiennent toujours. |
| Est-il en production ? | **Non.** Rien du travail n'est sur `main` ni sur `dev` : 140 commits d'avance sur `dev`, 152 sur `main`, 5 PR ouvertes toutes en brouillon (4 empilées + la promotion #33), aucun tag, aucune protection de branche. |
| Les décisions qui bloquent sont-elles techniques ? | **Non, pour l'essentiel.** 8 décisions humaines restent ouvertes, dont deux (H1, H7) déterminent ce que « production ready » veut dire. |
| Travail d'ingénierie restant | **Borné et chiffrable** : **44 constats ouverts** classés en cinq familles (§5), dont **17 fermables par un agent sans décision humaine** (`PA-B1..B7`, `PA-B12`, `PA-C2`, `PA-D1..D7`, `PA-E3`). |

**Ce que l'audit contredit.** Le dossier se présente comme cohérent ; il l'est dans ses rapports de vague,
**pas dans ses points d'entrée**. Trois documents de référence (README, rapport de porte, matrice de test)
publient encore des chiffres d'une tête vieille de trois jours et de neuf vagues de qualité (163/162 tests
au lieu de 480/477 ; couverture 83,19 %/68,64 % au lieu de 95,91 %/93,68 %). Le rapport de porte — le seul
document qui porte le verdict — est donc **périmé dans ses motifs**, pas seulement dans ses chiffres. La
[checklist de release](FLOW-PRODUCTION-RELEASE-CHECKLIST.md) décrit, elle, un artefact qui n'est plus celui
de `dist/` : elle publie le binaire de `dist/superseded/` comme binaire courant.

**Ce que l'audit confirme et qui n'était pas prouvé avant.** Les 144 portes nommées de la CI ont été
vérifiées une à une : **les 144 correspondent à une fonction de test réellement présente** dans
`crates/flow`. Le mécanisme « renommer ou supprimer un test casse le build » fonctionne. Aucun `unsafe`,
aucun `TODO`/`FIXME`/`unimplemented!`, aucun `#[ignore]` dans le code produit.

---

## 2. Méthode et classification des affirmations

Chaque affirmation porte un niveau, conformément à `AGENTS.md`.

| Niveau | Emploi dans ce document |
|---|---|
| `CODE_READ` | lue dans `crates/flow/**` ou `.github/workflows/flow-ci.yml` par cet audit |
| `CI_READ` | lue dans une exécution GitHub Actions (identifiant de run cité) |
| `DOC_READ` | lue dans `docs/**`, `README.md`, `CHANGELOG.md` |
| `MEASURED_ELSEWHERE` | chiffre produit par une campagne du dépôt, **non re-mesuré ici** (pas de build autorisé) |
| `PROPOSED` | recommandation de cet audit, non ratifiée |

**Limite centrale.** Tous les nombres de tests, de couverture, de mutation et de performance sont
`MEASURED_ELSEWHERE` : ils sont crus sur la foi des rapports et des runs CI, pas re-produits par cet audit.
Les commandes qui les re-produiraient sont en §9 et exigent un `GO` borné.

---

## 3. État factuel

### 3.1 Produit

| Élément | Constat | Niveau |
|---|---|---|
| Version, dépendances | `0.2.0-rc.1`, `[dependencies]` vide, `Cargo.lock` = 1 paquet | `CODE_READ` |
| Outillage | `rust-toolchain.toml` = 1.95.0 + rustfmt/clippy ; `rust-version = "1.95"` | `CODE_READ` |
| Taille | 10 modules, ≈6 800 lignes produit hors modules de test inclus dans les sources ; `commands.rs` = 2 478 lignes produit (≈36 %) | `CODE_READ` |
| Hygiène du code | 0 `unsafe`, 0 `TODO`/`FIXME`/`unimplemented!`, 0 `#[ignore]`; 339 fonctions `#[test]` | `CODE_READ` |
| Limites dures | 200 000 entrées (listing), 20 000 entrées (comparaison d'octets), 64 MiB contenu, 64 MiB/flux Git, 1 MiB/flux obligation, 32 MiB patch, 1 024 caractères de ref | `CODE_READ` |
| Surface CLI | `init`, `doctor`, `approve`, `run`, `publish`, `reconcile`, `status`, `explain`, `propose`, `proposals`, `assemble`, `disable`/`enable`, `version` | `DOC_READ` |

### 3.2 Tests et CI

| Élément | Constat | Niveau |
|---|---|---|
| Tests exécutés sur la tête | **480 (Windows) / 477 (Linux)**, 0 ignoré — planchers CI 480/477 | `CI_READ` + `CODE_READ` |
| Portes nommées | **144**, toutes résolues vers une fonction présente ; 0 nom orphelin, 0 doublon | `CODE_READ` |
| Jobs CI | 5 : `test ubuntu-latest`, `test windows-latest`, `polyglot×2`, `coverage (product-only gate)` | `CI_READ` |
| Étapes du job `test` | `fmt`, `clippy -D warnings`, tests gardés, build release, reproductibilité (Windows, deux archives), install smoke, binaire release insensible à `FLOW_TEST_CRASH`, self-tests R0-a, packaging par plateforme + provenance | `CI_READ` |
| Dernier run vert | push `43131263091`, PR `43131268888` — **5/5 succès** sur `ad1191b` | `CI_READ` |
| Couverture (Linux CI, tête `00f87b7`) | **95,91 % lignes / 93,68 % branches**, porte 95/90 **franchise, sans dérogation** | `MEASURED_ELSEWHERE` |
| Mutation | sous-ensemble de modules : 517/542 = **95,4 %** (`state` 140/140, `commands` 96,7 %, `main` 94,2 %, `gitcli` 88,6 %) ; **pas en CI** ; crate entière `NOT_RUN` ; 5 modules non mesurés sous Linux (≈401 mutants) | `MEASURED_ELSEWHERE` |
| Stabilité | 10 passes consécutives vertes sur hôte au repos (vague 9) | `MEASURED_ELSEWHERE` |

### 3.3 Git, PR et livraison

| Élément | Constat | Niveau |
|---|---|---|
| Avance de branche | 140 commits devant `origin/dev` (`2c8cb62`), 152 devant `origin/main` (`9d5170a`) | `CODE_READ` |
| `dev` est-il contenu ? | **Oui** — promotion vers `dev` possible sans conflit | `CODE_READ` |
| `main` est-il contenu ? | **Non** — `9d5170a` (licence #49) absent de la branche : la promotion vers `main` exigera une vraie fusion | `CODE_READ` |
| Pile de PR | #51 → #52 → #53 → #54, **toutes en brouillon** ; #54 `MERGEABLE`/`CLEAN`, examen `reviewDecision` vide | `CI_READ` |
| #33 (`dev`→`main`) | brouillon ouvert depuis le 2026-11-09 | `CI_READ` |
| Tags | **aucun** | `CODE_READ` |
| Artefacts `dist/` | Windows + Linux, tous deux épinglés **`f5bb271`** (2026-11-10T11:02Z), compteurs de tests 232/233 | `CODE_READ` |
| `dist/` versionné ? | **Non** — `.gitignore` `dist/` ; `git ls-files dist` = 0 ; `SHA256SUMS.txt` couvre 2 fichiers ; `"unsigned": true` | `CODE_READ` |
| Reproductibilité | par hôte et par chaîne d'outils (Windows) ; inter-hôtes et Linux **non revendiqués** | `DOC_READ` |

### 3.4 Dépôt et gouvernance

| Élément | Constat | Niveau |
|---|---|---|
| Fichiers suivis | **2 703**, dont **2 532 dans `experiments/` (93,6 %)** contre 21 dans `crates/` | `CODE_READ` |
| Volume local | 7,3 Go dans l'arbre de travail (dont 3,5 Go d'artefacts d'expérience, ignorés) ; `.git` = 10,3 Mo | `CODE_READ` |
| Journaux versionnés | 5 fichiers `.log` suivis, alors que `.gitignore` ignore `*.log` — contradiction interne | `CODE_READ` |
| Protection de branche | **aucune** : `/branches/main/protection` = 404, `rulesets` = `[]` | `CI_READ` |
| Sécurité GitHub | `secret_scanning` **activé**, `push_protection` **activée**, `dependabot_security_updates` **désactivées** | `CI_READ` |
| Fichiers communautaires | pas de `CODEOWNERS`, pas de `dependabot.yml`, **aucun workflow de release** | `CODE_READ` |
| Issues | **47 ouvertes, 0 fermée** ; rien après #48 ; `ISSUE_MAP.md` figé à l'import du 2026-11-09 | `CI_READ` |
| Tableau de tâches | projet `f8fd10b0` : **47 cartes, toutes `backlog`** ; les dossiers de décision citent le projet `f49a3c31` et des cartes `t-mw9*` qui **n'existent plus** (`not_found`) | `CI_READ` |

---

## 4. Matrice de préparation à la production

Le dépôt ne définit pas « production ready » en un seul endroit. Cet audit applique sept critères, adaptés
à un outil CLI local-first, et les confronte aux preuves.

| # | Critère | Exigence minimale | État | Verdict |
|---|---|---|---|---|
| 1 | **Enveloppe de plateformes** | supportée, testée, publiée | Windows + Linux démontrés (build, tests, smoke) ; macOS `NOT_AVAILABLE` ; ARM absent ; enveloppe de dépôt bornée (≤20 000 fichiers) non publiée comme limite de support | **PARTIEL** |
| 2 | **Artefact livrable vérifiable** | binaire + provenance + intégrité, au SHA livré | reproductible par hôte ; **provenance périmée de 60 commits** ; non versionnée ; non signé | **PARTIEL** |
| 3 | **Qualité non régressable** | toute régression mesurée bloque | fmt, clippy, 480/477 tests, 144 portes, couverture 95/90 : **en CI** ; mutation et performance : **mesurées mais non bloquantes** | **PARTIEL** |
| 4 | **Posture de sécurité** | limites écrites, canal de divulgation, décision sur la signature | limites écrites (pas de sandbox, reçus non signés, hooks non supprimés) ; `SECURITY.md` encore « pre-implementation » ; pas de modèle de menace à jour | **PARTIEL** |
| 5 | **Compatibilité, montée de version, rétention** | politiques explicites | schémas versionnés et schéma étranger refusé ; **aucune migration**, **aucune rétention**, **aucun GC** | **INSUFFISANT** |
| 6 | **Processus de release** | branche protégée, tag, notes, propriétaire du GO | aucun tag, aucun workflow de release, **aucune protection**, `GO_PRODUCTION` non donné | **ABSENT** |
| 7 | **Exploitabilité / récupération** | diagnostiquer, réconcilier, revenir en arrière | `doctor`, `reconcile`, journal durable, rollback documenté | **ACQUIS** |

**Lecture.** Le produit est meilleur que son dossier (§3.2), et son dossier est meilleur que son état de
livraison (§3.3, §3.4). Le risque de production n'est plus dans le code : il est dans **la chaîne de preuve
qui relie le code à un artefact publiable**, et dans **les décisions qui n'ont pas été prises**.

---

## 5. Registre des travaux restants

Identifiants `PA-*` = **nouveaux** identifiants d'audit, sans rapport avec `RF-*`/`US-*`/`H*`.
Sévérité : **S1** bloque une revendication de production · **S2** dégrade la preuve · **S3** hygiène.
« Ferme » = la preuve exacte qui clôt le constat.

### A — Décisions humaines (aucun agent ne peut les clore)

| ID | Sév. | Constat | Ferme |
|---|---|---|---|
| `PA-A1` | S1 | **H1** — option architecturale provisoire O0/O1/O2/O3 : `OPEN` | décision datée au registre |
| `PA-A2` | S1 | **H7** — usage interne d'abord ou produit indépendant : `OPEN`. C'est ce choix qui définit l'enveloppe de production (sandbox, signature, macOS, multi-utilisateur) | décision + §4 réévaluée |
| `PA-A3` | S1 | **H3** — sandbox non-trusté Windows/Linux : `OPEN` ; `UNSUPPORTED` en attendant | politique + tests OS réels |
| `PA-A4` | S1 | **H4** — seuils et budget S9 : `OPEN` ; toute revendication de productivité reste `UNPROVEN` | seuils chiffrés figés avant run |
| `PA-A5` | S3 | **H6** — handoff P0 vers la session Ravum : `OPEN` (≈15 min, aucun risque pour Flow) | brouillon de handoff accepté |
| `PA-A6` | S2 | **H8** — règle d'arrêt des revues : `OPEN` ; 9 vagues de qualité + 3 dossiers de revue produits sans règle d'arrêt | règle écrite |
| `PA-A7` | S1 | **G10** — indépendance : `NOT_ACHIEVED`, non fermable en interne | signature d'un relecteur hors équipe |
| `PA-A8` | S1 | **D4 / `GO_PRODUCTION`** : non donné ; `D4` = `PARTIAL` au registre vs `NOT GIVEN` en V6 §7.2 | `GO(scope, repo, branch, paths, budgets, evidence, stop_conditions)` daté |
| `PA-A9` | S2 | **D0/D1/D2/D3** — statuts `PARTIAL`/`DOCUMENT-CONDITIONAL`/`PROPOSED`/`DRAFT` jamais réévalués alors que les vagues 1→9 ont exécuté les tests que D2 déclarait non exécutés | mise à jour **proposée** puis ratifiée |

### B — Qualification manquante (ingénierie, sans décision)

| ID | Sév. | Constat | Ferme |
|---|---|---|---|
| `PA-B1` | **S1** | **Le rapport de porte est périmé alors qu'il porte le verdict** : tête `06eb7c3`, compteurs 163/162, couverture 83,19 %/68,64 %, mutation 75,0 %. Sur la tête réelle : 480/477 et 95,91 %/93,68 % | rapport de porte réémis sur `ad1191b`, motifs de verdict mis à jour |
| `PA-B2` | S2 | **La mutation n'est pas une porte** : mesurée à la main, absente de la CI. Une régression de score passe inaperçue | job planifié + seuil qui échoue |
| `PA-B3` | S2 | 5 modules jamais mesurés sous Linux (`json`, `paths`, `lock`, `proposal`, `scratch`, ≈401 mutants) — bloqué sur l'espace disque de l'hôte, pas sur le code | campagne reprise, une ligne par module |
| `PA-B4` | S3 | Mutation crate entière (≈960 mutants, ≈24 h) `NOT_RUN` : on ne publie qu'une **borne inférieure** | un run non borné + survivants jugés |
| `PA-B5` | S2 | 10 survivants `gitcli` (grâce de lecture, égalité à la nanoseconde, moisson de zombies) inaccessibles sans harnais | harnais de synchronisation/crash |
| `PA-B6` | S2 | Les fenêtres de crash sont prouvées **en debug** (`FLOW_TEST_CRASH` compilé hors release). On ne sait pas tuer le binaire livré à un point connu | harnais externe sur le binaire release |
| `PA-B7` | S3 | `cargo-fuzz`/libFuzzer `NOT_RUN` (contrat zéro dépendance) ; le fuzzer artisanal a trouvé un vrai défaut (F1) | cible de fuzz hors arbre + run daté |
| `PA-B8` | S2 | **Reçus non signés** (PRF-01) : « integrity ≠ authentication ». Quiconque écrit `.flow/receipts` peut forger un reçu | décision (clé locale/HMAC) **ou** limite ratifiée dans un modèle de menace nommant qui peut forger |
| `PA-B9` | S2 | **Aucune rétention, aucun GC, aucun quota** de `.flow/` (PRF-02/08) : croissance non bornée, absence de logs seulement affichée | politique de rétention + preuve qu'aucun objet référencé n'est supprimé |
| `PA-B10` | **S1** | **L'enveloppe de dépôt est un refus, pas une limite documentée** : >20 000 entrées ou >64 MiB de contenu ⇒ `refus`, pas de mode dégradé. Un monorepo courant dépasse 20 000 fichiers. Rien ne dit à l'utilisateur si son dépôt est dans le périmètre | soit comparaison en flux (l'enveloppe monte), soit « dépôt supporté ≤ 20 000 fichiers » publié et testé au bord |
| `PA-B11` | S2 | **macOS** `NOT_AVAILABLE` : ni build, ni test, ni artefact | support réel **ou** non-support explicite dans l'enveloppe (§4.1) |
| `PA-B12` | S2 | **Aucune porte de performance** : les temps sont descriptifs. La régression qui wedgerait un utilisateur (type R2-09) ne serait pas vue | smoke de budget en CI (bornes larges, politique de bruit écrite) |
| `PA-B13` | S1 | **S9 confirmatoire** jamais lancé : la raison d'être du produit (gain de temps vérifiable) reste `UNPROVEN` | campagne pré-enregistrée, n suffisant, différence appariée |
| `PA-B14` | S3 | Approbation humaine **par candidat** absente : qui peut `run --candidate` + `publish --candidate` publie dès que la politique est approuvée | décision produit (hors défaut du candidat actuel) |

### C — Livraison et chaîne de preuve

| ID | Sév. | Constat | Ferme |
|---|---|---|---|
| `PA-C1` | **S1** | `dist/` épinglé **`f5bb271`**, tête `ad1191b` (60 commits d'écart), compteurs de tests 232/233 au lieu de 480/477 ; `dist/` **gitignoré** donc provenance et SBOM ne sont pas versionnés (G10 N1/N2). La [checklist de release](FLOW-PRODUCTION-RELEASE-CHECKLIST.md) §2 publie en outre `bbde18c5…` (1 033 216 B) comme artefact courant, alors que `dist/` contient `4fa86af5…` (1 055 232 B) et que `bbde18c5…` est précisément celui de `dist/superseded/` | artefacts reconstruits au SHA livré + manifeste versionné (ou Release GitHub liée depuis le dépôt), et checklist §2 réalignée sur `dist/` |
| `PA-C2` | S3 | `SHA256SUMS.txt` ne couvre que 2 fichiers (G10 N3) | liste complète, ou justification écrite |
| `PA-C3` | S2 | Binaire **non signé** (`"unsigned": true`) : SmartScreen Windows, aucune chaîne de confiance de distribution | signature Authenticode/minisign **ou** acceptation écrite du risque |
| `PA-C4` | S3 | Reproductibilité par hôte seulement ; inter-hôtes et Linux non revendiqués | campagne Linux + comparaison inter-hôtes, ou maintien explicite de la portée |
| `PA-C5` | S2 | Aucun tag, version `0.2.0-rc.1` « unreleased », aucun workflow de release, notes de version non publiées | tag + Release avec artefacts et notes, ou statut pré-release assumé |
| `PA-C6` | **S1** | **Rien n'est sur `main` ni sur `dev`** : pile #51→#52→#53→#54 en brouillon, #33 ouverte depuis 4 jours ; `dev` contenu dans la branche, `main` non | fusion ordonnée de la pile puis promotion `dev`→`main` |
| `PA-C7` | **S1** | **Aucune protection de branche**, aucun `CODEOWNERS`, pas de checks requis : n'importe quel push direct sur `main` passe | ruleset + checks requis + revue obligatoire |
| `PA-C8` | S3 | Installation = copier un binaire, désinstallation = le supprimer ; pas de paquet, pas de mise à jour, pas de canal | décision (acceptable en interne) + documentation d'installation |

### D — Dérive documentaire (agent, bon marché, fort rendement)

| ID | Sév. | Constat | Ferme |
|---|---|---|---|
| `PA-D1` | S2 | `README.md:90` : « 163 tests Windows, 162 Linux » — la CI exige **480/477** | chiffres remplacer par la référence à la tête + au plancher |
| `PA-D2` | S2 | `README.md:4` : « couverture et mutation sous les seuils » — la couverture est **au-dessus** (95,91/93,68 vs 95/90) ; la mutation de module l'est aussi, sauf `gitcli` (88,6 %) | bannière réécrite par motif, pas par slogan |
| `PA-D3` | S2 | Mêmes compteurs périmés dans `FLOW-PRODUCTION-TEST-MATRIX.md`, `FLOW-PRODUCTION-COMPATIBILITY.md`, `FLOW-PRODUCTION-BASELINE.md` | une source unique de compteurs, les autres la citent |
| `PA-D4` | S2 | `SECURITY.md` dit encore « pre-implementation », « no released Flow security engine » : ni versions supportées, ni canal de divulgation, ni délai | politique de divulgation réelle ; vérifier la divulgation privée GitHub |
| `PA-D5` | S3 | `D4` = `PARTIAL` (registre) vs `NOT GIVEN` (V6 §7.2) — N4 non résolu | une seule source de statut |
| `PA-D6` | S2 | **Troisième occurrence du même défaut** : R2-06 (4 compteurs contradictoires) « corrigé », puis vague 5, puis vague 8, aujourd'hui à nouveau. La prose n'est pas dérivée de la mesure | générer les compteurs depuis la sortie CI dans **un** fichier machine, et faire échouer un contrôle de dérive |
| `PA-D7` | S3 | `docs/INDEX.md` désigne le rapport de porte comme source du verdict ; il ne l'est plus | index mis à jour après `PA-B1` |

### E — Dépôt et suivi

| ID | Sév. | Constat | Ferme |
|---|---|---|---|
| `PA-E1` | **S1** | **La file d'acceptation est cassée** : 47 cartes, toutes en `backlog`, et les dossiers de décision du 2026-11-12 renvoient à des cartes (`t-mw9*`, projet `f49a3c31`) qui n'existent plus. Le tableau a été ré-importé depuis les 47 issues (6 `H*` + 26 `RF-*` + 15 `US-*`) : **les sujets sont revenus, les états ne le sont pas** — les 12 cartes en revue (A1–A12), les 7 cartes de travail absentes de l'import (G10, métadonnées, P0, W4/W5/W7/W8) et les 25 `waitingForHuman` ont été perdus | reconstruire la file depuis §5, en conservant les identifiants historiques dans les descriptions |
| `PA-E2` | S2 | 47 issues ouvertes, **0 fermée**, rien après #48 : le suivi GitHub ne reflète ni les vagues 4→9 ni les constats ouverts | fermer/migrer, ou déclarer le tracker non utilisé |
| `PA-E3` | S2 | `experiments/` = **93,6 % des fichiers suivis** et 3,5 Go sur disque ; 5 `.log` versionnés contredisent `.gitignore` | règle de rétention des preuves : rapports + résultats machine, journaux bruts dehors |
| `PA-E4` | S3 | `commands.rs` ≈36 % du code produit (G10 N6) | extraction de sous-modules |
| `PA-E5` | S3 | Pas de `CODEOWNERS`, pas de `dependabot.yml`, mises à jour de sécurité Dependabot désactivées ; Actions épinglées par SHA (bien) mais sans cadence de mise à jour | fichiers + cadence |
| `PA-E6` | S3 | `main` et `dev` portent deux commits de licence distincts (#49, #50) : `main` n'est pas un ancêtre de la branche | fusion assumée lors de la promotion |

---

## 6. Séquence recommandée

L'ordre ci-dessous met **la vérité documentaire avant la mesure**, la mesure avant la livraison, et la
livraison avant la qualification produit. Les efforts sont `PROPOSED`.

### Étape 1 — Rendre le dossier vrai (≈0,5–1 jour, agent, aucune décision)

`PA-D1` `PA-D2` `PA-D3` `PA-D4` `PA-D7` · `PA-B1` (rapport de porte réémis sur `ad1191b`) · `PA-D6`
(source unique de compteurs + contrôle de dérive) · `PA-C2` · `PA-E3`.

*Pourquoi d'abord :* tant que le point d'entrée annonce 163 tests et une couverture sous le seuil, toute
décision humaine est prise sur des chiffres faux. C'est le seul lot qui **améliore la qualité de toutes les
décisions suivantes** sans consommer d'autorisation.

### Étape 2 — Rendre la preuve bloquante (≈2–4 jours, agent + `GO` de build)

`PA-C1` (artefacts reconstruits au SHA livré, manifeste versionné) · `PA-B2` (mutation en CI) ·
`PA-B12` (smoke de performance) · `PA-B3` (5 modules Linux, dès qu'un hôte a ≈20 Go libres) · `PA-B6`
(harnais de crash sur le binaire release).

*Pourquoi ensuite :* aujourd'hui trois qualités revendiquées (mutation, performance, fenêtres de crash) ne
sont pas défendues par une porte. Une revendication non gardée est une revendication qui régressera.

### Étape 3 — Livrer (décisions + administration)

`PA-C6` (fusion de la pile, puis promotion) · `PA-C7` (ruleset, checks requis, `CODEOWNERS`) · `PA-C5`
(tag + Release) · `PA-C4` (portée de reproductibilité) · `PA-C8` · `PA-E5`.

*Sans cette étape, le produit n'est pas « en production », quel que soit son score de tests.*

### Étape 4 — Qualifier le produit (décisions humaines)

`PA-A1` `PA-A2` `PA-A3` `PA-A4` `PA-A7` `PA-A8`, puis `PA-B8` `PA-B9` `PA-B10` `PA-B11` `PA-B13` `PA-B14`.

---

## 7. Les trois goulots qui ne sont pas techniques

1. **H7 puis H1 : « production ready » n'a pas de définition unique.** Voici ce que le même dossier exige
   selon la réponse, à enveloppe de code identique :

   | Critère | Usage interne | Produit indépendant |
   |---|---|---|
   | Fusion, protections, tag, Release | **exigé** | exigé |
   | Artefact au SHA livré + versionné | **exigé** | exigé |
   | Rétention/GC de `.flow/` | **exigé** | exigé |
   | Enveloppe de dépôt (≤20 000 fichiers) | acceptable si documentée | **bloquant** : à lever (`PA-B10`) |
   | Sandbox non-trusté (H3) | non requis | **bloquant** |
   | Signature de code et de reçus | non requis | **exigé** |
   | macOS | non requis | exigé si le marché le demande |
   | S9 confirmatoire | utile | **exigé** (c'est l'argument de vente) |

   Autrement dit : **en usage interne, il reste 1 à 2 jours de travail et 3 décisions ; en produit
   indépendant, il reste des semaines et deux parties externes.** Trancher H7 vaut mieux que dix vagues de
   qualité supplémentaires.

2. **`GO_PRODUCTION` et la fusion ne sont pas des formalités ici.** Le travail est sur une branche ; `dev`
   a 140 commits de retard, `main` 152, et aucune protection n'empêcherait un push direct sur `main`. Le
   risque de production n'est pas que le code casse : c'est qu'il ne soit **jamais publié**, et que la
   preuve accumulée se périme (elle a déjà commencé : `dist/` date de 60 commits).

3. **La file d'acceptation est cassée.** Les dossiers de décision du 2026-11-12 sont excellents et
   répondables en une passe ; les cartes qu'ils référencent n'existent plus, et les 47 cartes restantes sont
   toutes en `backlog` (donc non autorisées). Un humain qui répond aujourd'hui n'a **aucun objet traçable**
   à mettre à jour. `PA-E1` est bon marché et conditionne l'usage de tout le reste.

---

## 8. Ce que cet audit ne peut pas conclure

- **Aucune exécution.** Pas de build, pas de test, pas de benchmark (politique `NO-GO IMPLEMENTATION`).
  Tous les chiffres de §3.2 sont `MEASURED_ELSEWHERE` ; une re-mesure sur la tête `ad1191b` reste à faire.
- **Aucune revue de code exhaustive.** Les 10 modules n'ont pas été relus ligne à ligne ; les constats
  `CODE_READ` portent sur la structure, les marqueurs (`unsafe`/`TODO`/`#[ignore]`), les limites et la CI.
  La qualité sémantique du code reste couverte par les revues existantes, pas par cet audit.
- **Aucun jugement sur `Rwanbt/Ravum`**, non lu et non modifié.
- **Aucune ratification** d'ADR, de RFC, de licence, de seuil S9, ni de `GO_PRODUCTION`.
- **Aucune estimation d'effort engageante** : §6 est `PROPOSED` et ne tient compte ni de la disponibilité de
  l'hôte (la campagne Linux a déjà été bloquée par le disque) ni du temps humain de décision.

---

## 9. Index des preuves et reproduction

**Preuves lues par cet audit**

- Code : `crates/flow/src/{main,commands,proposal,gitcli,state,scratch,lock,json,paths,sha256}.rs` ·
  `crates/flow/Cargo.toml` · `Cargo.toml` · `rust-toolchain.toml`
- CI : `.github/workflows/flow-ci.yml` (5 jobs, 144 portes nommées, planchers 480/477, porte de couverture 95/90)
- Runs : `43131263091` (push), `43131268888` (pull_request) sur `ad1191b` — 5/5 succès
- Dossier : [rapport de porte](FLOW-PRODUCTION-GATE-REPORT.md) ·
  [capacités](../CAPABILITIES.md) · [vague 9](FLOW-QUALITY-WAVE9.md) ·
  [checklist de release](FLOW-PRODUCTION-RELEASE-CHECKLIST.md) ·
  [revue G10](FLOW-PRODUCTION-G10-MAINTAINABILITY-REVIEW.md) ·
  [seconde revue](FLOW-PRODUCTION-SECOND-REVIEW.md) ·
  [dossiers de décision](../planning/2026-11-12-flow-decision-dossiers.md) ·
  [registre des décisions](../DECISION_REGISTER.md) · [V6](../reviews/RAVUM-FLOW-V6-POST-CLAUDE-REFERENCE.md)
- Livraison : `dist/**/PROVENANCE.json` (deux plateformes, `f5bb271`)

**Reproduction (exige un `GO` borné distinct — rien de ceci n'a été exécuté)**

```bash
cargo fmt --check && cargo clippy --locked --all-targets -- -D warnings
cargo test --locked --all-targets            # attendu 480 (Windows) / 477 (Linux), 0 ignoré
pwsh -File experiments/quality/reproduce.ps1 # porte de reproduction unique (fmt, clippy, tests)
# couverture produit (chaîne de la CI) : cargo llvm-cov --all-targets --branch \
#   --ignore-filename-regex '[\/]tests[\/]' puis analyze_lcov.py --fail-under-lines 95 --fail-under-branches 90
# mutation : cargo mutants --in-diff <probe.diff> --jobs N --bins -- --bins   (par module, voir wave 9 §2)
```

**Identifiants ajoutés par cet audit :** `PA-A1..A9`, `PA-B1..B14`, `PA-C1..C8`, `PA-D1..D7`, `PA-E1..E6`.
Aucun identifiant historique n'a été renuméroté.
