# RAPPORT BRUT — Agent D « Ravum & ADR »

**Statut du document :** brut, non consolidé, **aucun ADR/RFC ratifié**. Toutes les lignes de ce fichier sont des propositions de lecture soumises au Lead et à l'humain.
**Observation :** 2026-11-09, ~16:05–16:25 Europe/Paris. **Aucune écriture hors de ce fichier**, aucun clone, aucun build, aucun test, aucune modification Ravum.
**Périmètre :** 42 ADR Ravum, 12 RF-RFC Flow, 12 FLOW-ADR proposés, droits/provenance, frontières d'extraction.

> **AVERTISSEMENT DE NOMENCLATURE — à lire avant tout usage de ce rapport.**
> Les identifiants `ADR-000…ADR-011` désignent **deux séries différentes** : les ADR Ravum (langage/compilateur, 000-041) et les FLOW-ADR (décisions d'orchestration, 000-011). Le brief de mission contient déjà cette collision (cf. §6, D4). Dans tout document Flow, écrire `RAVUM-ADR-0xx` ou `FLOW-ADR-0xx`, jamais `ADR-0xx` seul (`docs/TRACEABILITY.md:16` interdit l'alias mais conserve la numérotation identique : l'ambiguïté reste entière).

---

## 1. Mission et méthode

1. **Question posée :** documenter factuellement (a) l'état des 42 ADR Ravum et leur pertinence réelle pour Flow, (b) l'état des 12 RF-RFC et 12 FLOW-ADR, (c) les droits/provenance et les frontières d'extraction, (d) ce qui bloque une ratification en bloc. Je ne ratifie rien, je ne réécris aucun ADR.
2. **Sources primaires utilisées :**
   - Ravum via **API GitHub / raw.githubusercontent.com uniquement** au SHA épinglé `5408d9b3a48dab2884e2b3eb1aa89c3c16ce3d96` (= `dev`, vérifié par `GET /repos/Rwanbt/Ravum/branches`) : `contents` de `docs/adr`, `crates`, racine ; en-têtes des ADR-000/024/041 ; `crates/ravum-store/src/lib.rs` ; `crates/ravum-patch-host/src/lib.rs` ; `crates/ravum-{workspace-,}coordinator/src/lib.rs` ; `crates/ravum-dsp-harness/src/lib.rs` ; `crates/ravum-workspace-coordinator/src/git_writer.rs` (grep de lignes) ; `README.md` ; `CONTRIBUTING.md` ; `SECURITY.md` ; commits (100 derniers) ; PR/branches/protections ; `docs/adr/README.md`.
   - Flow (lecture locale, branche `docs/deepseek-d0-d4-review`, HEAD `1d5fddf`) : `AGENTS.md`, `docs/DEEPSEEK_GOAL.md`, `docs/reviews/RAVUM-FLOW-V6-POST-CLAUDE-REFERENCE.md` (intégralement pour les zones ADR/RFC), `docs/adr/*.md` (12), `docs/rfc/*.md` (12), `docs/RAVUM_BOUNDARY.md`, `docs/TRACEABILITY.md`, `docs/DECISION_REGISTER.md`, `docs/BACKLOG.md`, `docs/USER_STORIES.md`.
   - Tiers : page officielle GitHub « Licensing a repository » (texte source `github/docs`, ligne 20).
3. **Lecture des 42 ADR :** texte intégral lu dans la reproduction V6 (annexe Z : `V6:3393-5529`, 42 sections). **Fidélité au dépôt primaire :** vérifiée par échantillon (en-têtes ADR-000/024/041 identiques à V6, `Statut`, `Dépendances`) + table du registre upstream (`docs/adr/README.md`, ligne `7+n` pour `ADR-n`) pour les 42 dépendances + existence des 42 fichiers par `contents` API. Les 39 autres textes ne sont **pas** comparés octet à octet → `SOURCE_READ` via V6, pas `CODE_READ` upstream (cf. §8).
4. **Classifications utilisées :** `CODE_READ` (lu dans le fichier upstream au SHA), `SOURCE_READ` (lu dans une reproduction/registre), `MEASURED` (compteur d'API daté), `PROPOSED` (texte d'ADR/RFC), `UNKNOWN`, `NOT_REVIEWED`. **Aucun `MEASURED` n'est un test** ; **aucun ADR n'est `ACCEPTED`.**
5. **Ce que je n'ai pas fait :** pas de copie/adaptation de code Ravum, pas de création de LICENSE, pas de changement de statut, pas de conseil juridique. Le §4 documente des faits et des risques, pas une conclusion de droit.

### 1.1 État factuel re-mesuré (contredit partiellement la baseline du Lead — cf. §6)

| Fait | Valeur observée (2026-11-09 ~16:10) | Preuve |
|---|---|---|
| Ravum public | oui | `GET /repos/Rwanbt/Ravum` → `visibility=public` |
| Ravum licence déclarée | **aucune** (`license` vide) | idem |
| `LICENSE` / `LICENSE.md` / `COPYING` au SHA épinglé | **404 pour les trois** | `GET /contents/...?ref=5408d9b…` |
| Fichiers `docs/adr` au SHA | **43 entrées = 42 `ADR-*.md` + `README.md`** | `GET /contents/docs/adr?ref=…` |
| Statut des 42 ADR | **tous `PROPOSED`** | `docs/adr/README.md` (colonne Statut) ; en-têtes éch. |
| Ravum issues ouvertes | **90** (liste: 91 items dont 1 PR) | `GET /issues?state=open` ; `search total_count=90` |
| Ravum PR ouvertes | **1** — `#101` brouillon (`feat/lang-core-1.0-rc`→`dev`), créée le 2026-11-09 (horodatage API, fuseau non normalisé dans ma capture) | `GET /pulls?state=open` |
| Ravum `dev` | `5408d9b3…` — commit « merge(mvp0d) … (#87) », 12:46:36 | `GET /commits/5408d9b…` |
| Ravum `main` | `54491488…` | `GET /branches` |
| Flow issues / PR | **47** issues / **1** PR (`#33` dev→main) | `search` + `pulls` |
| Flow licence | **aucune** (`license` vide) et **aucun fichier LICENSE** dans le checkout | `GET /repos/Rwanbt/Ravum-flow` ; `glob` local |
| Protection de branche (Ravum et Flow) | **UNKNOWN** — `GET /branches/{main,dev}/protection` → **HTTP 401** | cf. §8 |

---

## 2. Matrice 42/42 ADR Ravum

**Codes pertinence :** `D` = directe (doit être citée comme contrainte ou frontière par Flow) · `I` = indirecte (doctrine/méthode transposable, jamais obligation) · `N` = nulle (hors périmètre Flow, ne doit pas être citée comme justification).
**Dépendances :** colonne du registre upstream `docs/adr/README.md` (source primaire), concordante avec `V6:2388-2429`.
**Preuve :** `V6:Lxxxx` = début du texte intégral dans l'annexe Z ; `R:n` = ligne `7+n` du registre upstream ; `hdr` = en-tête upstream contrôlé directement (échantillon). Tous les textes portent `Statut : PROPOSED` en ligne 3 de chaque fichier upstream.
**Contradictions :** `C1…C8` renvoient au §3.3 (aucune n'est « résolue » par ce rapport).

| ID | Titre (thème) | Dép. | Pert. | Note / contradiction | Preuve |
|---|---|---|---|---|---|
| RAVUM-ADR-000 | Périmètre universel, garanties conditionnelles | — | I | Doctrine « toute garantie est liée à un profil/périmètre » : bonne règle d'écriture Flow, ne crée aucune obligation. C6 si citée comme promesse d'universalité. | V6:3393 · R:7 · hdr |
| 001 | Compilateur initial en Rust | 000 | N | Choix d'implémentation du compilateur. **Ce n'est pas « les autorités »** (§6 D4). | V6:3444 · R:8 |
| 002 | Modèle mémoire ownership/emprunts/régions | 001 | N | Langage uniquement. | V6:3495 · R:9 |
| 003 | Erreurs, panic, défaillance | 002 | N | Le vocabulaire d'échec n'est pas celui des statuts de vérification Flow. | V6:3546 · R:10 |
| 004 | Contrats, raffinements, invariants | 003 | I | « Preuves séparées du code exécutable » éclaire `spec ≠ test`, mais reste adossé au solveur Ravum. | V6:3597 · R:11 |
| 005 | Système d'effets et capacités | 002,004 | N | Base conceptuelle de 029 ; **pas « fencing »** (§6 D4). | V6:3648 · R:12 |
| 006 | Classes temps réel et certification de délai | 005 | D | Frontière : définit `rt_safe`/`rt_bounded`/`hard_rt`. Flow ne doit produire **aucune** de ces garanties. C4. | V6:3699 · R:13 |
| 007 | Arithmétique, flottants, fast-math | 003,004 | N | Langage. | V6:3750 · R:14 |
| 008 | Concurrence, synchronisation, annulation | 002,005 | N | Concurrence d'exécution du langage ≠ concurrence d'écriture Git de Flow. | V6:3801 · R:15 |
| 009 | Frontières `unsafe`, FFI, code de confiance | 001,004 | N | **Ce ne sont pas « les droits »** (§6 D4). | V6:3852 · R:16 |
| 010 | Frontend et IR sémantique multi-niveaux | 002,003,004 | N | **Ce ne sont pas « les binaires »** (§6 D4). | V6:3903 · R:17 |
| 011 | Syntaxe progressive et parsing d'édition | 010 | N | « Overlay » = buffer d'éditeur ; collision de vocabulaire avec l'overlay FS de Flow. C5. | V6:3954 · R:18 |
| 012 | Compilation incrémentale à requêtes | 010,011 | N | Langage. | V6:4005 · R:19 |
| 013 | Cache de preuves et identité de contexte | 012 | D | Analogue direct de `BuildContextId` ; règle « cache invalidé par défaut en cas d'incertitude ». C3. | V6:4056 · R:20 |
| 014 | Protocole de preuve, statuts, confiance | 004,013 | D | Fournit le vocabulaire `PROVED/DISPROVED/UNKNOWN/TIMEOUT/UNSUPPORTED/ASSUMED/TESTED` et « UNKNOWN ≠ succès ». | V6:4107 · R:21 |
| 015 | Solveurs et moteurs de vérification | 010,014 | N | Ravum uniquement. | V6:4158 · R:22 |
| 016 | Model checking, fuzzing, simulation déterministe | 008,015 | I | Seeds/replay versionnés = méthode transposable à S9, pas une exigence produit. | V6:4209 · R:23 |
| 017 | Codegen initial : Rust sûr | 010,012 | N | Langage. | V6:4260 · R:24 |
| 018 | Backends natifs Cranelift/LLVM | 017 | N | Langage. | V6:4311 · R:25 |
| 019 | Daemon permanent, isolation, budgets | 012 | D | **Anti-modèle** : Flow = pas de daemon obligatoire (FLOW-ADR-011). Citer 019 pour « budgets bornés », jamais pour « daemon requis ». C2. | V6:4362 · R:26 |
| 020 | Compilations à la frappe, source incomplète | 011,019 | N | « Overlays versionnés » = buffers éditeur. C5. | V6:4413 · R:27 |
| 021 | Hot swap atomique, versionnement du code | 019,020 | D | Frontière d'ownership : Flow ne détient **jamais** l'activation. C4. | V6:4464 · R:28 |
| 022 | Migration d'état, rollback, effets irréversibles | 021 | D | Doctrine « pas de rollback fictif » + réconciliation non destructive, transposable à FLOW-ADR-005. C4. | V6:4515 · R:29 |
| 023 | Runtime minimal, profils sans runtime | 000,002 | I | Soutient « aucun runtime obligatoire » ; ne concerne pas Flow directement. | V6:4566 · R:30 |
| 024 | Thread audio, bascule DSP aux frontières de bloc | 006,021,022 | D | Interdiction : aucun code Flow dans le callback, aucune promesse de latence. C4. | V6:4617 · R:31 · hdr |
| 025 | Frontend full-stack et WASM | 023,005 | N | Langage/produit Ravum. | V6:4668 · R:32 |
| 026 | Schémas et migrations de bases de données | 025 | N | Une base SQLite de Flow ne relève pas de cet ADR (piège de réutilisation). | V6:4719 · R:33 |
| 027 | API machine pour agents et IDE | 014,019 | D | JSON/SARIF versionné, IDs stables, CLI/LSP découplés : modèle direct pour le protocole Flow. | V6:4770 · R:34 |
| 028 | Autorité des contrats, changement IA vérifié | 027 | D | « Un agent propose, n'affaiblit pas » : aligné sur la gouvernance Flow, mais Flow n'a pas encore de mécanisme équivalent (C7). | V6:4821 · R:35 |
| 029 | Security-by-default et sandbox du build | 005,009 | D | Sandbox FS/réseau/process + manifeste d'effets = socle de P0-ENV / FLOW-ADR-004. | V6:4872 · R:36 |
| 030 | Reproductibilité, provenance, artefacts | 029 | D | Lockfiles, SHA exacts, artefacts identifiés, manifestes de preuve. | V6:4923 · R:37 |
| 031 | CI en gates et qualification exacte au SHA | 014,029 | D | « NO-GO si requis rouge, inconnu non justifié ou SHA divergent » = règle Flow. Son échelle T0-T4 est ravumienne : **ne pas la reprendre telle quelle**. C8. | V6:4974 · R:38 |
| 032 | Performance et budgets de construction | 012,018 | I | Méthode p50/p95/p99 + RAM/CPU transposable ; cibles 50/150/500 ms = compilateur, **non transposables**. C3. | V6:5025 · R:39 |
| 033 | Télémétrie, confidentialité, données locales | 027,029 | I | Doctrine local-first/offline ; Flow l'a déjà par FLOW-ADR-011. | V6:5076 · R:40 |
| 034 | Ergonomie, LSP, debug, navigation des preuves | 011,027 | I | « CLI/JSON avant extension IDE » = ordre déjà proposé par RF-RFC-012. | V6:5127 · R:41 |
| 035 | Certificats de preuve et TCB | 014,018,030 | I | Concept de TCB explicite ; certificats non disponibles pour toutes les obligations. | V6:5178 · R:42 |
| 036 | Interop GPU et accélérateurs | 007,024 | N | Hors périmètre Flow. | V6:5229 · R:43 |
| 037 | Paquets, plugins, macros, écosystème | 028,029 | I | Supply chain/plugins sandboxés : transposable en politique de dépendances Flow. | V6:5280 · R:44 |
| 038 | Ordre des plateformes et coût de portabilité | 000,023 | I | Linux+Windows+WASM d'abord : recoupe RF-RFC-009 (Windows/Linux avant macOS). | V6:5331 · R:45 |
| 039 | Validation de l'optimisation, compatibilité | 007,018,035 | N | Compilateur. | V6:5382 · R:46 |
| 040 | Diagnostics structurés, audit, retour IA | 027,032 | I | JSON Schema/SARIF + minimisation des secrets ; orienté compilateur. | V6:5433 · R:47 |
| 041 | Gouvernance open source et changements | 030 | D | ADR/RFC/CODEOWNERS/branche protégée/**licences évaluées avant ajout**. C1 : sa précondition est absente sur son propre dépôt. | V6:5484 · R:48 · hdr |

**Synthèse 42/42 :** 13 `D`, 11 `I`, 18 `N` (comptage de ce rapport, pas une décision). **42/42 `PROPOSED`**, aucun `ACCEPTED`, aucun `REJECTED`, aucun `SUPERSEDED`.

---

## 3. Matrice 12 RF-RFC + 12 FLOW-ADR et contradictions

### 3.1 RF-RFC-001..012 — « décidé » vs « proposé »

**Aucun RFC n'est décidé :** les 12 fichiers portent `PENDING / PROPOSED` (`docs/rfc/*.md:3`). « Décidé » ci-dessous signifie **décidé de fait par un acte observable** (dépôt créé, fichier nommé), jamais ratifié.

| RFC | Hypothèse historique | Décidé de fait ? | FLOW-ADR lié | Tension / conflit |
|---|---|---|---|---|
| RF-RFC-001 nom/binaires | `Ravum Flow` / `ravum-flow` | Partiellement : dépôt réel `Rwanbt/Ravum-flow` créé 2026-11-09 13:32 ; casse = `Ravum-flow` | — | La question du RFC (« valider la casse réelle ») est tranchée par l'API, pas par un humain → acte non ratifié. |
| RF-RFC-002 licence/droits | « étudier MIT OR Apache-2.0 » | **Non** — aucune LICENSE dans Flow (glob local), `license` vide côté API | FLOW-ADR-009 | Contredit `DECISION_REGISTER.md:27` (« no implicit MIT/Apache ») et FLOW-ADR-009 (« pas de licence choisie par IA »). C1. |
| RF-RFC-003 source de vérité | Git publication + journal local | Non | FLOW-ADR-001 | RFC-003 omet l'arbre candidat et l'autorité runtime que FLOW-ADR-001 ajoute → RFC plus étroit que l'ADR ; ratifier l'un sans amender l'autre crée deux périmètres. |
| RF-RFC-004 outils non intégrés | observé, jamais sûr par présomption | Non | FLOW-ADR-002 | FLOW-ADR-002 ajoute `MANAGED_PATCH` + `VIRTUALIZED` différé : le RFC ne couvre que 2 des 4 modes. |
| RF-RFC-005 MVCC fichier/hunk | optimiste fichier/hunk | Non | (RF-008 ; FLOW-ADR-005 pour la publication) | Deux couches distinctes (admission de patch vs publication de ref) : les confondre ferait passer un CAS de ref pour un contrôle de conflit sémantique. |
| RF-RFC-006 protocole externe | Rust + JSONL stdio, MCP éventuel | Non | (RAVUM-ADR-027 en analogue) | Citer RAVUM-ADR-027 comme justification du protocole Flow serait un glissement d'échelle (API de compilateur ≠ protocole d'orchestrateur). |
| RF-RFC-007 stockage durable | **CAS local + journal append-only** | Non | FLOW-ADR-006 | **Contradiction de présupposé** : FLOW-ADR-006 met le CAS maison en *alternative à écarter* et propose objets Git + SQLite WAL. C9 (bloquant pour une ratification groupée). |
| RF-RFC-008 build/CI initial | Cargo natif puis adaptateurs | Non | FLOW-ADR-007 | Cohérent (outils natifs, graphe conservateur, E2E visibles). |
| RF-RFC-009 packaging | CLI Rust Win/Linux/macOS, daemon optionnel | Non | FLOW-ADR-011 | « daemon optionnel » vs FLOW-ADR-011 « CLI local-first sans daemon » : conciliable, mais doit être écrit. |
| RF-RFC-010 niveaux de preuve | receipts versionnés, `PROVED` borné | Non | (FLOW-ADR-003 pour les reçus) | **Flow n'a aucun solveur** : importer `PROVED` dans le vocabulaire Flow sans producteur identifié = faux statut. C10. |
| RF-RFC-011 sync distante | hors MVP, TLS/RBAC avant réseau | Non | FLOW-ADR-011 (R7) | Cohérent. |
| RF-RFC-012 IDE/UX | CLI/JSON avant extensions | Non | (RAVUM-ADR-034 en analogue) | Cohérent sur l'ordre ; ne pas citer RAVUM-ADR-034 comme preuve d'ergonomie Flow. |

### 3.2 FLOW-ADR-000..011 — propositions et dépendances de ratification

Tous `PROPOSED` (`docs/adr/*.md:3`), `Human sign-off: PENDING`. « Dépend de » = dépendance logique que **j'infère** des textes (aucune n'est écrite comme telle par les auteurs) ; « bloque » = ce qui empêche une ratification sans ambiguïté.

| ID | Décision candidate (résumé fidèle) | Dépend de | Bloqué par |
|---|---|---|---|
| FLOW-ADR-000 | D0–D4 documentaire / R0–R7 exécution après GO borné | — | C11 (trois nomenclatures de gates coexistent : D0–D4, G0–G5 dans `V6:2524-2529`, R0–R7 dans le backlog) |
| FLOW-ADR-001 | Git publié / arbre candidat / journal / runtime Ravum = autorités distinctes | 000 | RF-RFC-003 plus étroit ; identité « runtime Ravum » à ne pas confondre avec un contrôle Flow (C4) |
| FLOW-ADR-002 | 4 modes d'agents, CAS de préconditions, isolation jetable | 001 | C8 (preuves d'isolation OS absentes, H3) ; `VIRTUALIZED` différé vs US-004 |
| FLOW-ADR-003 | hooks dans l'ordre Git, gel post-hooks, reçu avant CAS ref | 001 | Aucune preuve négative encore produite (P0-HOOK-ORDER non reproduit) |
| FLOW-ADR-004 | TRUSTED/UNTRUSTED + sandbox FS/réseau/process réelle | 001, 003 | H3 : Windows non qualifié ; « refus si sécurité manquante » doit être un test, pas une phrase |
| FLOW-ADR-005 | writer exclusif + epoch + CAS ref + outcome sous verrou, journal idempotent | 001, 003 | C4/C10 : un `git` externe contourne ; ne pas promettre plus que « détection + revalidation » |
| FLOW-ADR-006 | objets Git + journal SQLite WAL FULL ou append-only, GC/pinning prouvés | 001 | **C9** : RF-RFC-007 présuppose un CAS ; divergence à trancher avant ratification |
| FLOW-ADR-007 | Cargo/tsc/pytest/CMake natifs, graphe conservateur, cache contextuel, E2E bloquants | 001, 003, 006 | C10 (statuts) ; pas de mesure S9 |
| FLOW-ADR-008 | S9 apparié + seuil/budget humains avant essai, veto perte/secret/CI | 000, 011 | H4 : seuil et budget non fixés ; `NOT_AVAILABLE` ≠ échec à définir |
| FLOW-ADR-009 | inventaire auteurs/SHA/tests/dépendances ; adaptateur, licence licite ou clean-room ; pas de cycle ni audio RT | 000, 041 (RAVUM) | **H2** : aucune licence Ravum, aucun grant écrit ; aucune licence Flow (C1) |
| FLOW-ADR-010 | édition en octets + modes, rename/delete, symlink/casse, LFS/submodule, refus explicite | 001, 003 | Preuve de l'état actuel : P1-FILES `CODE_READ` (cf. §4.3) ; pas de corpus de tests |
| FLOW-ADR-011 | CLI local-first sans LLM/serveur, workers bornés 16 Go, remote/enterprise en R7 | 000 | Aucune (la moins bloquante), mais ne doit pas absorber RAVUM-ADR-019 (C2) |

### 3.3 Contradictions nommées (ce qui empêche une ratification en bloc)

- **C1 — Droits :** RAVUM-ADR-041 exige des licences « évaluées avant ajout d'outils » ; le dépôt Ravum n'a **aucune** licence (API 404 + `README.md:216-218` + `CONTRIBUTING.md:3`) et Flow non plus. Ratifier 041, RF-RFC-002 et FLOW-ADR-009 « en bloc » reviendrait à ratifier une politique de licence inexistante. **Preuve :** §1.1, §4.1.
- **C2 — Daemon :** RAVUM-ADR-019 propose un « daemon permanent par workspace » ; FLOW-ADR-011 et RF-RFC-009 veulent « daemon optionnel / sans daemon ». Non contradictoire si les périmètres (compilateur Ravum vs orchestrateur Flow) sont écrits ; contradictoire si 019 est cité comme précédent de Flow.
- **C3 — Cache/identité :** RAVUM-ADR-013 exige l'invalidation par défaut en cas d'incertitude ; RAVUM-ADR-032 vise p95 < 500 ms sur fonction chaude. Tension non mesurée (coût d'un cache froid). Ne pas citer 013 + 032 ensemble comme promesse de performance.
- **C4 — Activation/RT :** RAVUM-ADR-021/022/024 décrivent hot swap, migration et bascule DSP ; l'implémentation observée déclare la migration d'état **non implémentée** (`crates/ravum-patch-host/src/lib.rs:10`) et « aucune prétention au temps réel dur » (`:13`). Citer 021/022/024 comme capacités acquises serait faux ; les citer comme **frontière d'ownership** est correct.
- **C5 — « Overlay » :** RAVUM-ADR-011/020 = overlays de buffers d'éditeur ; Flow utilise « overlay » pour un recouvrement FS/VFS. Collision lexicale à bannir dans les deux sens.
- **C6 — « Universel » :** RAVUM-ADR-000 subordonne toute garantie à un profil ; RAVUM-ADR-038 restreint les cibles initiales (Linux/Windows/WASM). Citer 000 comme promesse d'universalité contredit 038.
- **C7 — Autorité des contrats :** RAVUM-ADR-028 interdit à un agent d'affaiblir un contrat ; aucun mécanisme équivalent n'existe côté Flow (`FLOW-ADR-000` ne le traite pas). Citer 028 comme « déjà couvert » serait une sur-citation.
- **C8 — Gates :** RAVUM-ADR-031 exige `NO-GO` si un requis est rouge, inconnu ou si le SHA diverge ; or l'implémentation observée fige l'arbre **avant** les hooks (`git_writer.rs:842` puis hooks 1223-1227, `commit-tree` réutilise `tree` en 868) et exécute les hooks dans la racine réelle (`:1344`). Ce n'est pas une preuve d'exploitation : c'est une **divergence contrat/implémentation** (`CODE_READ`). Toute doc Flow qui citerait 031 comme « satisfait » serait fausse.
- **C9 — Stockage :** RF-RFC-007 (« CAS local de contenu ») vs FLOW-ADR-006 (« comparer objets Git ; CAS maison = alternative ») : présupposés incompatibles, réparables par un amendement explicite de l'un des deux.
- **C10 — Statuts :** RF-RFC-010 (« `PROVED` borné ») suppose un producteur de preuves qui n'existe pas côté Flow ; le vocabulaire de RAVUM-ADR-014 ne peut être importé que comme **étiquette de reçu Ravum**, pas comme résultat Flow.
- **C11 — Nomenclatures de gates :** `DECISION_REGISTER.md` (D0–D4), `FLOW-ADR-000` (D0–D4 + R0–R7), `V6:2524-2529` (G0–G5) et `V6:5685-5693` (G0–G5, contenu différent : « G2 = sécurité » vs `D2 = security and transactional contract`) coexistent. Deux « G2 » de sens différents = risque de citation erronée.

### 3.4 Ordre de ratification minimal (proposition, non ratifiée)

1. **FLOW-ADR-000** (nommer les gates ; résoudre C11 d'abord dans le texte, sinon tout le reste hérite de l'ambiguïté).
2. **FLOW-ADR-001** (autorités) — prérequis de 003/005/006/007.
3. **FLOW-ADR-003** et **FLOW-ADR-010** (arbre exact, hooks, fichiers) — sans eux, 005/007 n'ont pas d'objet mesurable.
4. **FLOW-ADR-002** (modes/isolation) — après 001+003 ; dépend de H3 pour être honnête.
5. **FLOW-ADR-005** et **FLOW-ADR-006** — après 003 ; trancher C9 avant 006.
6. **FLOW-ADR-007** — après 001/003/006.
7. **FLOW-ADR-004** et **FLOW-ADR-011** — parallélisables ; 004 attend H3.
8. **FLOW-ADR-009** — en dernier des ADR « techniques », mais **en premier** pour toute réutilisation Ravum ; exige H2 + écrit.
9. **FLOW-ADR-008** (S9) — à figer avant H1/H4 ; ne peut être « ratifié » qu'avec seuil et budget humains.

**Ce qui interdit une ratification en bloc :** C1 (droits), C9 (stockage), C10 (statuts), C11 (gates), plus H2/H3/H4 non tranchés. Aucune de ces contradictions n'est une impossibilité logique : toutes sont réparables par texte, sauf C1 qui dépend d'un acte du propriétaire.

---

## 4. Droits, provenance, frontières

### 4.1 Faits sur les droits (aucune conclusion juridique)

1. **Ravum est public sans licence** : `GET /repos/Rwanbt/Ravum` → `visibility=public`, champ `license` vide ; `LICENSE`, `LICENSE.md`, `COPYING` → **404** au SHA épinglé ; la racine ne contient aucun fichier de licence (`contents` racine).
2. **Le dépôt dit lui-même l'absence de licence** : `README.md:216-218` — « aucune licence n'est actuellement déclarée, donc n'assumez pas que le contenu soit librement réutilisable » ; `CONTRIBUTING.md:3` — « le futur code n'a pas encore de licence arrêtée : avant tout apport externe de code, demander la politique de licence au responsable ». *(SOURCE_READ, upstream au SHA)*
3. **Règle documentaire GitHub** (page officielle, texte source `licensing-a-repository.md:20`) : « without a license, the default copyright laws apply, meaning that you retain all rights to your source code and no one may reproduce, distribute, or create derivative works from your work ». C'est un **fait documentaire cité**, pas un avis juridique.
4. **Conséquence factuelle pour Flow :** il n'existe aucun grant écrit permettant à un tiers — y compris un autre dépôt du même compte — de copier, modifier ou distribuer le code Ravum. Le propriétaire peut réutiliser son propre code, mais cela reste une **décision humaine non prise** (H2), et la licence de distribution de Flow n'est pas choisie (aucune LICENSE dans Flow).
5. **Authorship (fait, sans qualification juridique) :** sur les 100 derniers commits de `dev` (API, cap 100) : 57 `Rwanbt <barat.erwan@gmail.com>`, 39 `Ravum Agent <agent@ravum.local>`, 4 `Erwan Barat <barat.erwan@gmail.com>` ; **aucun `Co-Authored-By`** dans cet échantillon. Aucun CLA/DCO visible dans `CONTRIBUTING.md`. Un identifiant d'agent ne prouve ni l'existence ni l'absence de droits de tiers (`V6:5627` le dit correctement) ; l'échantillon ne couvre pas l'historique complet.
6. **Ce qu'il faudrait obtenir du propriétaire** (demande à formuler par l'humain, pas par un agent) : (a) un grant écrit nommant les chemins envisagés et les droits accordés (copie/modification/distribution, y compris sous une autre licence) ; (b) l'identification des titulaires pour toutes les lignes concernées, y compris les commits d'identité « Ravum Agent » ; (c) la licence de distribution choisie pour Flow et sa compatibilité avec les dépendances tierces (`Cargo.lock` non audité ici) ; (d) une politique de contribution (DCO/CLA) avant tout apport externe ; (e) une trace SPDX/provenance dans les deux dépôts.
7. **Options restantes, classées par coût de droits croissant :**
   - **O-adaptateur** (frontière de processus/CLI, aucune copie de source) : la moins risquée en droits, mais crée une dépendance de version et doit documenter qui possède l'interface.
   - **Clean-room** (réimplémentation à partir des seuls contrats documentés) : exige une séparation réelle ; **les agents Flow ont déjà lu le code Ravum** (dont moi : `git_writer.rs`, `ravum-store`, `patch-host`). Une revendication clean-room sur ces modules précis serait donc affaiblie et doit être déclarée comme telle.
   - **Accord écrit / licence** (MIT OR Apache-2.0 sur des chemins nommés, cf. RF-RFC-002) : seule option qui autorise une extraction ; bloquée par H2 et par C1.
   - **O0 / ne rien extraire** : reste viable et sans coût de droits.
8. **Ce qui interdit une copie de code Ravum dans Flow aujourd'hui :** l'absence de grant (fait 1-2), l'interdiction d'adoption implicite posée par `DECISION_REGISTER.md:27` et FLOW-ADR-009, et le fait que la licence de Flow n'est pas décidée. Le fait que le même propriétaire détienne les deux dépôts ne suffit pas : c'est une décision à acter (H2), pas une conséquence automatique.

### 4.2 Modules Ravum à ne PAS extraire ni réimplémenter

| Module (upstream au SHA) | Ce qu'il est (preuve) | Pourquoi Flow ne doit pas l'extraire ni le refaire |
|---|---|---|
| `ravum-patch-host` | « Patch host: versioned native modules behind a checked C ABI » ; un seul point d'entrée `f64 -> f64` ; le *host* décide ; in-flight gardé par `Arc`, déchargé par un thread dédié ; **migration d'état non implémentée (ADR-022 ouvert)** ; « no claim of hard real-time » (`crates/ravum-patch-host/src/lib.rs:1-13`) | C'est l'**autorité d'activation** et l'ABI de Ravum. L'extraire importerait l'activation (interdite à Flow, `RAVUM_BOUNDARY.md:7`) et un couplage ABI/layout. |
| `ActiveVersionId` / versionnement / rollback | RAVUM-ADR-021/022 (V6:4464-4560) ; état réel : migration non implémentée (`patch-host:10`) | Deux propriétaires de « dernière version saine » = double ownership et cycles ; Flow ne peut pas promettre un rollback qu'il ne possède pas. |
| Audio RT / DSP | `ravum-dsp-harness` : compteur d'allocations, cadence **simulée**, « no audio driver, no real-time thread priority, no XRUN » (`crates/ravum-dsp-harness/src/lib.rs:1-8`) | Sûreté RT ≠ sandbox : Flow ne doit embarquer aucun code RT ni hériter d'une promesse de latence (RAVUM-ADR-006/024). |
| Preuves sémantiques | RAVUM-ADR-014/015/035 ; `ravum-proof` (crate list) | Flow ne produit que des reçus bornés ; dupliquer un solveur ou un statut `PROVED` fabriquerait de faux verts (C10). |
| Coordinator | **Deux implémentations coexistent déjà** : `ravum-coordinator` (« M1 continuous loop » : snapshot courant, ordre analyze→build→verify→publish, smoke contract, `TESTED` jamais `PROVED`) et `ravum-workspace-coordinator` (« COLLAB-1→5 multi-agent proposals » : `AUTO_SAFE/REBASE_REVALIDATE/CONFLICT/REJECT/MANUAL_REQUIRED`, v2 fail-closed, `TEXTUALLY_APPLICABLE` jamais `AUTO_SAFE` seul) | `V6:2461` exige déjà d'éviter deux coordinateurs ; l'état upstream en montre deux. Un troisième côté Flow aggraverait le double ownership. Flow doit **consommer une interface stable** ou rester hors de ce périmètre. |
| `ravum-store` | `//! SQLite boundary and versioned migrations for the M4 reference application` ; importe `ravum_todo_core::{NewTask,Task,TaskPatch}` ; `SCHEMA_VERSION = 2` ; « SQL lives here and nowhere else » (`crates/ravum-store/src/lib.rs:1-16`) | **Confirmé par lecture : c'est la base SQLite de l'application Todo M4, pas un CAS générique.** `P1-STORE` passe de « handoff » à `CODE_READ` ; toute réutilisation « par le nom » est un contresens. `ravum-todo-core` confirme l'application M4. |

**Candidats d'audit restants — `NOT_REVIEWED` :** `ravum-snapshot`, `ravum-protocol`, `ravum-daemon`, `ravum-unit-graph`, `ravum-collab-oracle`, `ravum-batch`, `ravum-rust-adapter` (liste `RAVUM_BOUNDARY.md:15`, `V6:2367`) : aucun n'a été lu ici, et `ravum-coordinator` **manque** à cette liste alors qu'il existe upstream (§6, D5).

### 4.3 Preuve `CODE_READ` utile aux FLOW-ADR-003/010 (P0-HOOK-ORDER et P1-FILES)

`crates/ravum-workspace-coordinator/src/git_writer.rs` au SHA épinglé : `write-tree` en **842**, hooks en **1223/1226/1227** (`prepare-commit-msg` **avant** `pre-commit`), `commit-tree` réutilisant `tree` en **868**, `current_dir(&self.repo)` en **1344** (hooks dans la racine réelle), contenu `String` → `content.as_bytes()` en **799/803/1149**, mode par défaut `"100644"` en **816**, doc interne revendiquant l'ordre `prepare-commit-msg, pre-commit, commit-msg, post-commit` en **37-47**. C'est un `CODE_READ` : **ce n'est pas une reproduction de bug** et ce n'est pas mon mandat de la produire.

---

## 5. Réponse contradictoire (aux trois questions posées)

### 5.1 Quels ADR sont hors périmètre Flow et ne doivent pas être cités comme justification

- **Ne pas citer du tout (18, tous classés `N`) :** RAVUM-ADR-001, 002, 003, 005, 007, 008, 009, 010, 011, 012, 015, 017, 018, 020, 025, 026, 036, 039 — soit : 001-012 (langage/compilateur), 015-018 (preuves/backends), 020 (overlays d'éditeur), 025-026 (full-stack/DB), 036 (GPU), 039 (optimisation).
- **Citer uniquement comme méthode, jamais comme obligation (11, classés `I`) :** 000, 004, 016, 023, 032, 033, 034, 035, 037, 038, 040.
- **Citer comme contrainte ou frontière (13, classés `D`) :** 006, 013, 014, 019, 021, 022, 024, 027, 028, 029, 030, 031, 041. Seule exception de double usage : 019, à lire « budgets bornés », **jamais** « daemon requis ».
- **Cas particuliers piégeux :** 031 (son échelle T0-T4 est ravumienne : reprendre la règle, pas l'échelle) ; 019 (anti-modèle) ; 021/022/024 (frontières, pas capacités) ; 006 (frontière RT, pas garantie Flow) ; 041 (gouvernance citable, mais sa précondition licence est absente → C1).

### 5.2 Quelles contradictions empêchent une ratification en bloc

`C1` (droits/licence : précondition de RAVUM-ADR-041 absente et H2 ouvert), `C9` (CAS de RF-RFC-007 vs objets Git de FLOW-ADR-006), `C10` (`PROVED` de RF-RFC-010 sans producteur Flow), `C11` (D/G/R : trois nomenclatures de gates), et l'absence des décisions `H2`/`H3`/`H4`. `C2`–`C8` sont des risques de citation, pas des blocages de ratification ; `C3`/`C4` sont des tensions à documenter, pas des impossibilités.

### 5.3 Ordre de ratification minimal

`000 → 001 → {003, 010} → 002 → {005, 006} → 007 → {004, 011} → 008 → 009`, avec : C11 corrigé avant 000, C9 tranché avant 006, H3 avant 004, H4 avant 008, H2 + grant écrit avant 009 et avant **toute** ligne de code Ravum dans Flow. `009` est dernier dans l'ordre documentaire mais **premier verrou** pour l'extraction.

---

## 6. Désaccords réels

- **D1 — Compteurs de la baseline.** Le brief indique « aucune PR ouverte côté Ravum, 80 issues ». J'ai mesuré **90 issues ouvertes** et **1 PR ouverte** (`#101`, brouillon, `feat/lang-core-1.0-rc`→`dev`, créée le 2026-11-09). Explication la plus probable : dérive temporelle (le dépôt a été poussé le jour même, après l'observation du Lead), pas une erreur de méthode. **Conséquence :** toute phrase « Ravum n'a aucune PR ouverte » est déjà fausse ; les documents doivent dater et citer la requête, pas un chiffre en prose. Note : `V6:2364` listait déjà des PR #78-#85 ouvertes — la formulation « aucune PR » était donc fausse même historiquement.
- **D2 — « 42 fichiers ».** Exact pour `ADR-*.md` (42), mais `contents` renvoie **43 entrées** (42 + `README.md`). La formulation correcte est « 42 fichiers `ADR-*.md` + 1 registre `README.md` » ; sinon un audit automatique conclura à un écart.
- **D3 — « Aucune protection de branche ».** Je ne peux ni confirmer ni infirmer : `GET /branches/main/protection` et `/branches/dev/protection` renvoient **HTTP 401** sans authentification, sur Ravum comme sur Flow. C'est `UNKNOWN`, pas `CONFIRMÉ`. Recommandation : soit re-mesurer avec un jeton, soit écrire « non vérifié » dans les documents.
- **D4 — Le brief de mission applique la numérotation FLOW aux ADR Ravum.** « ADR-001 autorités », « ADR-006 CAS/journal », « ADR-009 droits », « ADR-005 fencing », « ADR-010 binaires » sont des **FLOW-ADR** ; les RAVUM-ADR-001/005/006/009/010 sont respectivement « compilateur en Rust », « effets/capacités », « classes temps réel », « unsafe/FFI », « frontend/IR ». Je refuse de ratifier la prémisse : le vrai constat est un **risque systémique de collision d'identifiants** (000-011 identiques dans deux séries), que `TRACEABILITY.md:16` ne neutralise pas puisqu'il conserve les mêmes nombres.
- **D5 — La liste des candidats d'audit est incomplète et l'anti-duplication est déjà violée en amont.** `RAVUM_BOUNDARY.md:15` et `V6:2367` listent 8 crates mais **omettent `ravum-coordinator`** (M1), alors que `ravum-workspace-coordinator` (COLLAB) est aussi présent : deux coordinateurs coexistent au SHA épinglé. Affirmer « Flow ne créera pas un second coordinateur » (`V6:2461`) sans traiter ce doublon interne laisse la duplication à trois.
- **D6 — « 42/42 lus » ne vaut que pour la reproduction.** J'ai lu les 42 textes dans V6 et vérifié 3 en-têtes upstream + les 42 dépendances dans le registre upstream. Pour 39 fichiers, je n'ai **pas** comparé octet à octet : la formulation honnête est `SOURCE_READ (V6) + SOURCE_READ (registre upstream)`, pas `CODE_READ upstream`. Un écart de transcription (V6:3389 le prévoit) resterait invisible.
- **D7 — Désaccord de fond sur le cadrage de la mission.** La mission suppose que des « contradictions empêchent la ratification en bloc ». Je constate l'inverse : aucune contradiction logique n'empêche une ratification **séquencée** ; ce qui bloque, ce sont deux **actes humains manquants** (H2 droits, H4 seuil) et une **divergence de présupposé réparable** (C9). Présenter cela comme un blocage doctrinal conduirait à sur-documenter au lieu de demander les deux décisions.

---

## 7. Recommandations bornées + falsification

1. **R1 — Préfixes obligatoires.** Dans tous les documents Flow (et titres d'issues), écrire `RAVUM-ADR-0xx` / `FLOW-ADR-0xx` / `RF-RFC-0xx` ; interdire `ADR-0xx` nu. *Coût :* une passe de texte. *Falsification :* si un lecteur externe identifie sans ambiguïté « ADR-006 » dans un document Flow, R1 est inutile.
2. **R2 — Allowlist de citation.** Reprendre la colonne « Pert. » du §2 comme règle : `N` = interdiction de citation comme justification ; `I` = section « doctrine » uniquement ; `D` = contrainte/frontière. *Falsification :* produire un cas où un ADR classé `N` fonde une décision Flow sans détour par un ADR Flow.
3. **R3 — Verrou H2 avant tout contact de code.** Aucune ligne Ravum dans Flow (ni copie, ni vendoring, ni « adaptation ») avant un grant écrit nommant les chemins + la licence de Flow ; en attendant, le contact reste de la lecture. *Falsification :* le propriétaire produit un grant écrit couvrant les chemins visés → R3 devient une simple formalité de traçabilité.
4. **R4 — Réparer C9 sans réécrire l'histoire.** Conserver RF-RFC-007 tel quel et ajouter une note « hypothèse superseded » renvoyant à FLOW-ADR-006 ; ne jamais réécrire le RFC (règle `V6:2382`). *Falsification :* si S9 mesure un besoin réel de CAS de contenu (coût de lecture Git > seuil), la note est inversée en faveur du CAS.
5. **R5 — Statuts : ne pas importer `PROVED`.** Flow utilise son propre jeu (`TEXTUALLY_APPLICABLE`, `TESTED`, `MANUAL_REQUIRED`, `STALE`, `UNKNOWN`…, déjà présent dans le coordinateur v2) ; `PROVED` n'apparaît que comme champ de reçu Ravum, jamais comme résultat Flow. *Falsification :* un producteur Flow de preuves bornées, avec checker, rendrait `PROVED` légitime.
6. **R6 — Séquence de ratification du §3.4, avec préconditions par ADR.** *Falsification :* ratifier 006 sans trancher C9 ou 009 sans H2 doit être déclaré recevable par l'humain ; sinon la séquence tient.
7. **R7 — Anti-citation explicite.** Ajouter aux documents Flow une liste noire : RAVUM-ADR-019 (daemon requis), 021/022/024 (capacités hot swap/RT acquises), 031 (échelle T0-T4), 006 (garantie RT Flow). *Falsification :* montrer un ADR Flow dont le sens dépend de l'un de ces quatre emplois.
8. **R8 — Mesures datées.** Remplacer les compteurs en prose par `requête + date + total_count` (ex. `search/issues?q=repo:Rwanbt/Ravum+type:issue+state:open` → 90 à 2026-11-09T16:10+01:00). *Falsification :* un compteur en prose qui reste exact 24 h sans re-mesure.
9. **R9 — Ne pas promettre l'isolation Windows.** `H3` non tranché : tout texte Flow doit dire `UNSUPPORTED`/`MANUAL_REQUIRED` tant qu'aucun test OS Windows n'existe (position déjà défendue par `V6:5805`). *Falsification :* une mesure Windows de confinement FS/réseau/process reproductible.

**Aucune de ces recommandations ne ratifie quoi que ce soit et aucune ne crée de licence.** Toutes restent réversibles par un texte.

---

## 8. Limites et `NOT_REVIEWED`

- **`NOT_REVIEWED` (upstream) :** `ravum-snapshot`, `ravum-protocol`, `ravum-daemon`, `ravum-unit-graph`, `ravum-collab-oracle`, `ravum-batch`, `ravum-rust-adapter`, `ravum-cli`, `ravum-todo-core`, `ravum-proof`, `ravum-coordinator` (au-delà de l'en-tête), `ravum-dsp-harness` (au-delà de l'en-tête), `ravum-patch-host` (au-delà de l'en-tête) ; `Cargo.lock` et licences des dépendances tierces ; `SECURITY.md` (aucune occurrence licence/CLA) au-delà du grep ; historique de commits au-delà de 100.
- **`NOT_REVIEWED` (Flow) :** V4/V5 antérieurs, `docs/PROJECT_CHARTER.md`, `ARCHITECTURE.md`, `ROADMAP.md`, `BENCHMARK_S9.md`, `SECURITY_TESTS.md`, le détail des 26 RF (seul le mapping `BACKLOG.md` a été lu), les 12 INV (cités via V6 uniquement), RF-RFC-013..024 (hors périmètre de ce rapport).
- **`UNKNOWN` :** protection de branche des deux dépôts (HTTP 401) ; exhaustivité des contributeurs et titularité ; compatibilité des licences tierces ; horodatages exacts en UTC (le quota API non authentifié s'est épuisé en fin de session, ~16:15 Europe/Paris, après les mesures du §1.1 ; toute re-vérification nécessite un jeton ou une nouvelle fenêtre de quota).
- **Pas de reproduction :** P0-HOOK-ORDER, P0-ENV, P1-FILES, P0-TREE restent des `CODE_READ`/`SOURCE_READ` ; aucun test, aucun build, aucune fixture. « Document n'est pas test » (`AGENTS.md:14`).
- **Pas juriste :** le §4 documente des faits et des risques ; aucune conclusion sur la validité d'un transfert, la titularité ou la compatibilité de licences n'est tirée ici.
- **Chevauchements d'équipe :** le volet droits recoupe probablement l'agent B (sécurité/`P0-ENV`) et le volet provenance l'agent A (Git/`P0-TREE`) ; je n'ai pas coordonné avec eux — ce rapport doit être lu comme un avis disjoint, pas comme un consensus.
- **Fidélité V6/upstream :** voir D6 ; un écart de transcription dans les 39 ADR non échantillonnés n'est pas exclu.

---

## 9. Références

**Ravum, SHA épinglé `5408d9b3a48dab2884e2b3eb1aa89c3c16ce3d96` (= `dev`, 2026-11-09 12:46:36).** Lecture par API/raw uniquement ; aucun clone, aucune exécution.
1. Métadonnées : `https://api.github.com/repos/Rwanbt/Ravum` (public, `license` vide) — consulté 2026-11-09.
2. `https://api.github.com/repos/Rwanbt/Ravum/contents/docs/adr?ref=5408d9b3…` → 43 entrées (42 `ADR-*` + `README.md`).
3. `…/contents/LICENSE?ref=…`, `…/LICENSE.md?ref=…`, `…/COPYING?ref=…` → **404**.
4. `…/contents/?ref=…` (racine : aucun fichier de licence).
5. `https://raw.githubusercontent.com/Rwanbt/Ravum/5408d9b3…/docs/adr/README.md` (registre 42/42 + politique ADR).
6. `…/docs/adr/ADR-000-…md`, `…/ADR-024-…md`, `…/ADR-041-…md` (en-têtes vérifiés).
7. `…/crates/ravum-store/src/lib.rs:1-16` (SQLite/M4 Todo, pas CAS) ; `…/crates/ravum-patch-host/src/lib.rs:1-13` ; `…/crates/ravum-workspace-coordinator/src/lib.rs:1-16` ; `…/crates/ravum-coordinator/src/lib.rs:1-8` ; `…/crates/ravum-dsp-harness/src/lib.rs:1-8`.
8. `…/crates/ravum-workspace-coordinator/src/git_writer.rs` : lignes 37-47, 799, 803, 816, 842, 868, 1149, 1223, 1226, 1227, 1344.
9. `…/README.md:216-218` (aucune licence déclarée) ; `…/CONTRIBUTING.md:3` ; `…/SECURITY.md` (sans objet licence).
10. `https://api.github.com/repos/Rwanbt/Ravum/commits?sha=5408d9b3…&per_page=100` (authorship) ; `…/branches` ; `…/pulls?state=open` (#101) ; `…/issues?state=open` (90).
11. `https://api.github.com/repos/Rwanbt/Ravum-flow` (public, `license` vide, `dev=1d5fddf`, `main=5672087`) ; `…/pulls?state=open` (#33) ; `search/issues` (47).

**Flow (local, HEAD `1d5fddf`, branche `docs/deepseek-d0-d4-review`) :** `AGENTS.md:11-24` ; `docs/DEEPSEEK_GOAL.md:36-74` ; `docs/TRACEABILITY.md:6-16` ; `docs/DECISION_REGISTER.md:5-27` ; `docs/RAVUM_BOUNDARY.md:3-24` ; `docs/BACKLOG.md:7-32` ; `docs/USER_STORIES.md:9-93` ; `docs/adr/FLOW-ADR-000..011` ; `docs/rfc/RF-RFC-001..012` ; `docs/reviews/RAVUM-FLOW-V6-POST-CLAUDE-REFERENCE.md:2361-2467, 2524-2529, 3385-5529, 5534-5697, 5703-5838`.

**Tiers :** `https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/licensing-a-repository` (texte source `github/docs`, `licensing-a-repository.md:20`), consulté 2026-11-09.

*Fin du rapport brut D. Aucun statut modifié ; toute ratification reste un acte humain.*
