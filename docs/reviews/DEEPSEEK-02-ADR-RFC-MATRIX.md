# DEEPSEEK-02 — MATRICE ADR/RFC, CONTRADICTIONS ET ORDRE DE RATIFICATION (D2, partiel)

**Statut :** `PROPOSED / NOT_RATIFIED`. Aucun ADR ni RFC n'a changé de statut. Ce document **consolide** le rapport brut [D](raw/DEEPSEEK-RAW-D-RAVUM-ADR.md) et les écarts relevés en baseline ([`DEEPSEEK-00`](DEEPSEEK-00-BASELINE.md) §6), sans les réécrire.
**Portée :** 42 `RAVUM-ADR`, 12 `RF-RFC` historiques, 12 `FLOW-ADR` proposés, 12 idées `RF-RFC-013..024`.
**Réponse à trancher :** quelles contradictions bloquent une ratification, et dans quel ordre minimal ratifier.

---

## 1. Inventaire et vocabulaire (le piège n°1)

| Série | Nombre | Statut observé | Emplacement |
|---|---|---|---|
| `RAVUM-ADR-000..041` | **42** | **tous `PROPOSED`** (en-tête upstream, ligne 3) | `Rwanbt/Ravum` `docs/adr/` @ `5408d9b3…` + reproduction V6 `:3385-5533` |
| `RF-RFC-001..012` | **12** | **`PENDING / PROPOSED`** | `docs/rfc/` + archive V6 |
| `FLOW-ADR-000..011` | **12** | **`PROPOSED`**, `human sign-off: PENDING` | `docs/adr/` |
| `RF-RFC-013..024` | **12 idées** | non déposées, non acceptées | archive V4/V3 |

> **Règle R1 (agent D, adoptée ici) :** dans tout document Flow, écrire `RAVUM-ADR-0xx` ou `FLOW-ADR-0xx`, **jamais** `ADR-0xx` seul. Les deux séries se recouvrent sur 000–011 et n'ont pas le même objet. `TRACEABILITY.md:16` interdit l'alias mais conserve les mêmes numéros : l'ambiguïté reste entière.
> **Constat de processus assumé :** le brief de mission lui-même employait « ADR-001 autorités », « ADR-006 CAS », « ADR-009 droits » — ce sont des **FLOW-ADR**. L'agent D a refusé la prémisse à raison. C'est exactement le risque systémique que R1 neutralise.

---

## 2. Matrice `RAVUM-ADR` — pertinence réelle pour Flow (42/42)

Codes : **D** = contrainte ou frontière à citer · **I** = doctrine/méthode transposable, jamais obligation · **N** = hors périmètre, ne doit pas être cité comme justification.
Source : rapport D §2 (lecture intégrale via V6, en-têtes upstream échantillonnés, dépendances du registre upstream).

**Bilan : 13 `D`, 11 `I`, 18 `N`.** *(Le §5.1 du rapport D écrit « 19 » et « 13 … soit 13 » : erreur d'arithmétique dans le rapport brut ; la ventilation de son propre tableau donne bien 13/11/18. Correction consignée, non dissimulée.)*

| Pertinence | ADR | Usage autorisé |
|---|---|---|
| **D (13)** | 006, 013, 014, 019, 021, 022, 024, 027, 028, 029, 030, 031, 041 | Frontières RT/activation/preuves, identité de contexte, API machine, sécurité par défaut, provenance, gates CI, gouvernance |
| **I (11)** | 000, 004, 016, 023, 032, 033, 034, 035, 037, 038, 040 | Doctrine de méthode uniquement (périmètre conditionnel, seeds/replay, budgets, local-first, TCB…) |
| **N (18)** | 001, 002, 003, 005, 007, 008, 009, 010, 011, 012, 015, 017, 018, 020, 025, 026, 036, 039 | Langage, compilateur, preuves, backends, full-stack, GPU : **interdits comme justification Flow** |

**Pièges d'usage (les plus coûteux) :**

| ADR | Piège | Formulation correcte |
|---|---|---|
| `RAVUM-ADR-019` | Cité comme « précédent daemon » | **Anti-modèle** : Flow = pas de daemon obligatoire. Citer pour « budgets bornés » seulement. |
| `RAVUM-ADR-021/022/024` | Cité comme capacités acquises (hot swap, rollback, RT) | **Frontières d'ownership**. L'implémentation déclare la migration d'état non implémentée (`ravum-patch-host/src/lib.rs:10`) et « no claim of hard real-time » (`:13`). |
| `RAVUM-ADR-031` | Cité comme « gate satisfait » | Reprendre la **règle** (NO-GO si requis rouge/inconnu/SHA divergent), pas l'échelle `T0-T4` ravumienne. Et le writer observé viole l'ordre des hooks : 031 n'est **pas** satisfait. |
| `RAVUM-ADR-006` | Cité comme garantie RT par Flow | Flow ne produit **aucune** garantie `rt_safe`/`rt_bounded`/`hard_rt`. |
| `RAVUM-ADR-041` | Cité comme gouvernance en place | Sa **précondition** (licences évaluées avant ajout) est absente sur son propre dépôt → voir `X-01`. |

---

## 3. Matrice `RF-RFC` — « décidé de fait » vs « proposé »

Aucun RFC n'est ratifié. « Décidé de fait » = un acte observable (dépôt créé, fichier nommé) a tranché la question **sans vote humain**.

| RFC | Question | Décidé de fait ? | Tension avec les `FLOW-ADR` |
|---|---|---|---|
| 001 | Nom `Ravum Flow` / `ravum-flow` | Partiellement : dépôt `Rwanbt/Ravum-flow` créé | Aucune ; mais collision probable du binaire `flow` (produit G) |
| 002 | Licence | **Non** — aucune `LICENSE`, ni Ravum ni Flow | `X-01` |
| 003 | Source de vérité | Non | Plus **étroit** que `FLOW-ADR-001` (n'inclut ni arbre candidat ni autorité runtime) |
| 004 | Outils non intégrés | Non | `FLOW-ADR-002` couvre 4 modes, le RFC 2 |
| 005 | MVCC fichier/hunk | Non | Deux couches à ne pas confondre : admission de patch ≠ CAS de ref |
| 006 | Protocole externe | Non | `--json` du CLI couvre 90 % du besoin agent (rapport G) |
| 007 | Stockage : **CAS local + journal** | Non | **Contredit `FLOW-ADR-006`** (CAS maison = alternative à écarter) → `X-02` |
| 008 | Build/CI natif | Non | Cohérent avec `FLOW-ADR-007` |
| 009 | Packaging, daemon optionnel | Non | Conciliable avec `FLOW-ADR-011`, à écrire explicitement |
| 010 | Niveaux de preuve, `PROVED` borné | Non | **Flow n'a aucun producteur de preuve** → `X-03` |
| 011 | Sync distante hors MVP | Non | Cohérent (R7) |
| 012 | IDE après CLI/JSON | Non | Cohérent |

---

## 4. Contradictions nommées (registre unique)

Fusion des `C1..C11` (rapport D) et `DOC-01..08` (baseline). **Aucune n'est une impossibilité logique** ; toutes sont réparables par texte, sauf `X-01` qui dépend d'un acte du propriétaire.

| ID | Contradiction | Preuve | Bloque |
|---|---|---|---|
| **X-01** | **Droits/licence** : `RAVUM-ADR-041` exige des licences évaluées ; Ravum n'en a aucune (`README.md:216-218` le dit lui-même), Flow non plus. | Rapport D §4.1 ; API `license:null` ; 404 sur `LICENSE`/`LICENSE.md`/`COPYING` | H2, `FLOW-ADR-009`, toute extraction |
| **X-02** | **Stockage** : `RF-RFC-007` présuppose un CAS de contenu ; `FLOW-ADR-006` le met en alternative à écarter (objets Git + journal). | `docs/rfc/RF-RFC-007.md` vs `docs/adr/FLOW-ADR-006.md` | `FLOW-ADR-006` |
| **X-03** | **Statuts de preuve** : `RF-RFC-010` importe `PROVED` sans producteur côté Flow. | RFC-010 ; `RAVUM-ADR-014` | `FLOW-ADR-007`, reçus |
| **X-04** | **Nomenclatures de gates** : D0–D4, G0–G5 (deux jeux différents dans la V6 : `:2524-2529` et `:5685-5693`), R0–R7. | V6, `DECISION_REGISTER.md`, `FLOW-ADR-000` | `FLOW-ADR-000` et tout ce qui le cite |
| **X-05** | **Machine d'états périmée** dans `ARCHITECTURE.md` : `APPROVED_HOOKS_APPLIED`, `CHECKS_PENDING`, `PUBLICATION_INTENT_DURABLE`, `REF_PUBLISHED` — alias que la V6 déclare obsolètes. | `ARCHITECTURE.md:40` vs V6 `:184-208` | Implémentation future, reçus |
| **X-06** | **Lettres S9 contradictoires** : V6 `D=Git classique, E=Flow, F=outil, G=séquentiel` vs `BENCHMARK_S9.md` `D=séquentiel, E=jj/GitButler, F=O0, G=Flow`. | V6 `:290` vs `BENCHMARK_S9.md:8` | H4, tout run S9 |
| **X-07** | **Double numérotation des tests** : V6 `CONTEXT-03/SANDBOX-05/PATH-08/REF-10/CRASH-11/E2E-12/CACHE-13` vs `SECURITY_TESTS.md` `TREE-03/TRUST-15/PATH-16/GIT-10/CRASH-12/E2E-13/CACHE-14`. | V6 `:407-427` vs `SECURITY_TESTS.md:5-22` | Traçabilité des preuves |
| **X-08** | **Biais de recommandation** : `ARCHITECTURE.md:56` recommande O3 ; `DECISION_REGISTER.md:6` dit « comparer O0/O1 d'abord » ; la V6 dit « O3 hypothèse, pas vainqueur ». | trois fichiers | H1 |
| **X-09** | **Mapping de phases flou** : plusieurs RF portent « R3/R5 » ou « R3/R4 » sans critère de sortie unique ; 9 RF ont deux phases. | `BACKLOG.md`, rapport G §2 | Roadmap R1+ |
| **X-10** | **Collision d'identifiants `RF-`** : `RF-0..9` (jalons) et `RF-001..026` (tickets) coexistent dans les mêmes textes. | V6 §7.1 | Support, triage |
| **X-11** | **Collision `ADR-`** 000–011 entre séries Ravum et Flow (cf. §1). | `TRACEABILITY.md:16` | Tous les documents |
| **X-12** | **Deux coordinateurs upstream** alors que la V6 exige « un seul coordinateur » : `ravum-coordinator` (M1) et `ravum-workspace-coordinator` (COLLAB) coexistent ; `RAVUM_BOUNDARY.md:15` omet le premier. | rapport D §4.2 et D5 | H6, périmètre Flow |

---

## 5. Ce que la V6 a déjà corrigé (à ne pas re-signaler comme neuf)

`COR-01..COR-21` couvrent : identités d'arbres, ordre des hooks, `env_clear` insuffisant, `AUTO_SAFE` textuel, fencing, CAS/WAL, binaires, benchmark, gates, fausse indépendance des revues, `ravum-store`, alternatives, double orchestration, cache/E2E, revue vs implémentation, et les cinq corrections Claude. **Elles sont intégrées dans le texte V6 ; elles ne sont pas des décisions.** Les re-signaler sans vérifier s'ils sont déjà traités serait du bruit.

---

## 6. Droits et provenance — ce que l'absence de licence signifie

| Fait | Étiquette | Conséquence |
|---|---|---|
| Aucun `LICENSE` côté Ravum, déclaré par le dépôt lui-même (`README.md:216-218`, `CONTRIBUTING.md:3`) | `SOURCE_READ` | Droit d'auteur par défaut : **aucun grant** de copie/modification/distribution à un tiers. |
| Aucun `LICENSE` côté Flow, `license:null` | `SOURCE_READ` | Flow n'est pas open source aujourd'hui ; H2 couvre **les deux** dépôts. |
| Authorship (100 derniers commits `dev`) : 57 `Rwanbt`, 39 `Ravum Agent`, 4 `Erwan Barat` ; aucun `Co-Authored-By`, aucun CLA/DCO visible | `MEASURED` (échantillon) | Fait, **pas** conclusion juridique. |
| Règle GitHub citée : sans licence, « no one may reproduce, distribute, or create derivative works » | `SOURCE_READ` | À formuler comme fait documentaire, pas comme avis juridique. |

**Options, par coût de droits croissant :** `O0` ne rien extraire · `O-adaptateur` (frontière de processus, aucune copie) · `accord écrit` nommant chemins et droits · `clean-room` — **affaibli** ici : les agents Flow ont déjà lu `git_writer.rs`, `ravum-store`, `patch-host` ; toute revendication clean-room sur ces modules doit être déclarée comme compromise.

**Modules à ne PAS extraire ni réimplémenter :** `ravum-patch-host` (autorité d'activation, ABI), `ActiveVersionId`/rollback, audio RT (`ravum-dsp-harness`), preuves sémantiques (`ravum-proof`), coordinateurs, `ravum-store` (confirmé : base SQLite de l'app M4 Todo, `lib.rs:1-16` — **pas un CAS**).

---

## 7. Ordre de ratification minimal (proposition, non ratifiée)

```
X-04 corrigé  →  FLOW-ADR-000
              →  FLOW-ADR-001
              →  {FLOW-ADR-003, FLOW-ADR-010}
              →  FLOW-ADR-002        (après H3)
              →  {FLOW-ADR-005, FLOW-ADR-006}   (X-02 tranché avant 006)
              →  FLOW-ADR-007
              →  {FLOW-ADR-004, FLOW-ADR-011}   (H3 avant 004)
              →  FLOW-ADR-008        (H4 : seuil + budget figés)
              →  FLOW-ADR-009        (H2 + grant écrit ; premier verrou pour toute extraction)
```

**Ce qui interdit une ratification *en bloc* :** `X-01` (droits), `X-02` (stockage), `X-03` (statuts), `X-04` (gates), plus H2/H3/H4 non tranchés.
**Ce qui ne l'interdit pas :** les autres tensions sont des risques de citation, réparables par une note ou un préfixe.

---

## 8. Désaccords à conserver ouverts

1. **D vs mandat** — D refuse de présenter les contradictions comme un « blocage doctrinal » : selon lui, deux actes humains manquants (H2, H4) et une divergence de présupposé (X-02) suffisent, le reste est séquence. Le mandat demandait « ce qui empêche une ratification en bloc ». Les deux lectures sont conservées : le *blocage* est réel, mais il est **administratif et décisionnel**, pas logique.
2. **D vs baseline** — compteurs datés : 90 issues et 1 PR (#101) côté Ravum, pas « 80 / 0 ». La baseline a été corrigée (§2 de `DEEPSEEK-00`). La protection de branche Ravum `dev` est **active** (check `mvp`), contrairement à une lecture non authentifiée qui renvoie 401.
3. **D vs « 42/42 lus »** — pour 39 des 42 ADR, la lecture est `SOURCE_READ` via la reproduction V6 + registre upstream, **pas** une comparaison octet à octet avec l'upstream. Un écart de transcription resterait invisible. Formulation honnête retenue.
4. **D vs G sur le nombre d'ADR « hors périmètre »** — D compte 18 `N` ; G suggère de sortir les stories qui citent des ADR ravumiens. Ce n'est pas la même question (citation vs périmètre produit) ; les deux listes ne se contredisent pas.

---

## 9. Conclusion D2

- Les **42+12+12** objets sont inventoriés, tracés et **tous non ratifiés** ; 13/11/18 pour la pertinence Ravum→Flow.
- **Quatre** contradictions bloquent une ratification groupée (`X-01..X-04`), **huit** autres sont des défauts de forme à corriger avant implémentation (`X-05..X-12`).
- **Aucune** ratification n'est proposée ici. L'ordre §7 est une **séquence de travail documentaire**, pas une approbation.
- Le geste le plus rentable n'est pas un nouveau document : c'est de **corriger `X-05`, `X-06`, `X-07`, `X-11`** (quatre fichiers, une heure) puis de demander **H2** et **H4**.

**Verdict D2 : `PROPOSED` — non franchi.** Les contrats sont écrits, pas testés ; les droits et les seuils manquent.

---

*Sources : [rapport D](raw/DEEPSEEK-RAW-D-RAVUM-ADR.md) (42 ADR, RFC, droits), [rapport G](raw/DEEPSEEK-RAW-G-PRODUCT-UX.md) (mapping US/RF), V6 lignes citées, `docs/adr/*`, `docs/rfc/*`, et vérifications API du 2026-11-09. Aucun test, aucune exécution, aucune écriture Ravum.*
