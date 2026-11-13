# DEEPSEEK-RAW-C — Build/CI : analyse d'impact, cache, E2E honnête

**Agent C — Build/CI.** Rapport **brut**, non consolidé, produit pour le Lead.
**Date d'observation :** 2026-11-09 (Europe/Paris). **Dépôt :** `D:\App\Ravum-flow`, branche
`docs/deepseek-d0-d4-review`, HEAD `1d5fddf6efc5d0535a474bebfb8d33faf341d746`.
**Statut :** `RAW / PROPOSED` — aucune décision ratifiée, aucun GO, aucun code, aucun build.

---

## 1. Mission / méthode

### 1.1 Question posée

Comment accélérer la validation d'un arbre candidat **sans produire de faux vert**, par écosystème
(Cargo, TypeScript, Python, C/C++, Go) ; quelle clé de cache est réellement exacte ; que valent les
approches existantes (sccache, Bazel/Buck2 remote cache) pour un solo offline sur 16 Go ; et **Flow
doit-il posséder l'ordonnanceur de builds ou déléguer aux outils natifs/CI en ne gardant que le
graphe d'obligations + receipts ?**

### 1.2 Méthode et discipline de preuve

1. Lecture locale des documents obligatoires : `AGENTS.md`, `docs/DEEPSEEK_GOAL.md`, V6 §0–§10 (l. 1–460),
   `docs/BENCHMARK_S9.md`, `docs/SECURITY_TESTS.md`, `docs/ROADMAP.md`, `docs/adr/FLOW-ADR-007.md`,
   `docs/rfc/RF-RFC-008.md`, `docs/BACKLOG.md` (RF-012..015, RF-018..020), et pour le contexte
   `docs/ARCHITECTURE.md`, `docs/DECISION_REGISTER.md`, `docs/TRACEABILITY.md`, `docs/ISSUE_MAP.md`,
   `docs/reviews/DEEPSEEK-00-BASELINE.md`.
2. Vérification d'un petit nombre de faits de dépôt par API GitHub publique (lecture seule), datée [R34].
3. Lecture de **sources primaires** des outils (Cargo, TypeScript, pytest, Python, Ninja/GCC/MSVC/CMake, Go,
   sccache, Bazel, GitHub merge queue), URL et date en §7 ; citations courtes uniquement.
4. Sondage **lecture seule** des toolchains installées sur la machine d'audit (versions), sans build ni test [R35].
5. **Aucun** build, test, benchmark, CI, migration, daemon, changement de branche, écriture Git. Ravum non
   cloné, non ouvert, non exécuté ; seul `Rwanbt/Ravum` en API/raw au SHA `5408d9b3…` a été consulté.
6. Chaque affirmation porte une étiquette : `SOURCE_READ`, `MEASURED`, `API_READ`,
   `MEASURED_HISTORICAL`, `PROPOSED`, `UNKNOWN`, `NOT_REVIEWED`. Une doc lue n'est pas une mesure ; un
   raisonnement n'est pas une preuve.

### 1.3 Environnement mesuré (machine d'audit, 2026-11-09)

| Élément | Valeur | Étiquette |
|---|---|---|
| RAM totale | **15,7 GiB** (cohérent avec la cible 16 Go) | `MEASURED` [R35] |
| CPU | i7-13620H, **10 cœurs / 16 threads** | `MEASURED` [R35] |
| Disque `D:` | 188,4 Go libres (2 414,5 Go utilisés) | `MEASURED` [R35] |
| Rust | cargo/rustc **1.95.0** | `MEASURED` [R35] |
| Node / npm | **26.13.0** / 11.20.0 | `MEASURED` [R35] |
| Python | **3.13.7**, pip 25.3, pytest **9.1.1** | `MEASURED` [R35] |
| CMake / Ninja | **4.1.1** / **1.13.1** | `MEASURED` [R35] |
| sccache | **0.18.0** | `MEASURED` [R35] |
| Git | 2.51.1.windows.1 | `MEASURED` [R35] |
| MSVC | VS Community 2026, toolset C++ présent ; `cl` hors `PATH` | `MEASURED` [R35] |
| Go | **absent** de la machine | `MEASURED` [R35] |
| WSL | non vérifié (commande bloquée, job tué) | `NOT_REVIEWED` |

**Conséquence :** l'audit peut raisonner sur les versions réellement présentes, mais **ne peut pas**
mesurer les temps incrémentaux, la RSS des compilateurs, ni l'adaptateur Go. Ces points restent `UNKNOWN`
et sont renvoyés à S9 (§6).

### 1.4 Ce que ce rapport ne fait pas

Il ne recommande aucun outil comme « solution » ; il ne ratifie ni FLOW-ADR-007 ni RF-RFC-008 ; il ne
tranche pas H1–H6. Il propose des **critères falsifiables** et signale les points où la documentation V6
doit être corrigée avant D2/D3.

---

## 2. Constats

### 2.0 Matrice de fiabilité du graphe (réponse courte à la question 1)

| Écosystème | Graphe réellement fiable | Incomplet par nature | Bascule en validation complète |
|---|---|---|---|
| **Rust/Cargo** | workspace, membres, deps `normal/dev/build`, features résolues, cibles | build scripts (fichiers + env + `OUT_DIR`), proc-macros, `include!`, `cfg` produits, dep-info non exposé | build script/proc-macro touché, config/lock/toolchain changés, cible différente |
| **TS** | arêtes `references` explicites + typecheck du projet | alias `paths` hors arbre, `import()` calculé, bundler/test runner, `.d.ts` et conditions `exports` | projet non référencé, `tsconfig`/alias changé, bundler ou runner concerné |
| **Python** | quasi rien de statique | imports dynamiques, `conftest.py`, plugins/entry points, environnement | **toujours** full suite en MVP (ou `UNKNOWN`) |
| **C/C++** | depfiles **après** compilation, cibles CMake/Ninja | nouveau header, includes calculés, générés, ABI, matrice d'options | header ajouté/supprimé, configure changé, ABI |
| **Go** | imports + deps de test par contexte | `go generate`, `embed`, cgo, build tags | tags/env/cgo/generate concernés |

### 2.1 Cargo

| ID | Constat | Preuve + étiquette | Impact Flow |
|---|---|---|---|
| C-01 | `cargo metadata` expose membres, cibles et graphe résolu, mais le `resolve` inclut **toutes** les plateformes par défaut ; `--filter-platform` le restreint et le tableau `packages` reste complet. | [R1] `SOURCE_READ` | Un graphe non filtré sur-déclare (sûr mais coûteux) ; une résolution par features/cible mal épinglée peut sous-déclarer. Toujours figer triple + features + profil. |
| C-02 | Sans `--no-deps`, la commande résout (et peut fetcher) ; `--locked` échoue si le lock manque ou changerait ; `--offline` peut produire une résolution **différente** de l'online. | [R1] `SOURCE_READ` | Deux exécutions du même tree peuvent produire deux graphes. Le contexte doit inclure hash de `Cargo.lock`, `--locked/--offline`, version cargo. |
| C-03 | Les build scripts apparaissent comme cible `custom-build` et deps `kind: "build"`, mais ni leurs entrées relues (`rerun-if-changed`), ni leurs variables, ni les fichiers `OUT_DIR` ne sont exposés par metadata. | [R1][R2] `SOURCE_READ` | L'arête paquet est connue ; l'arête fichier/env ne l'est pas. Un build script qui lit un fichier hors package rend le graphe intrinsèquement incomplet. |
| C-04 | Par défaut, Cargo relance le build script **si un fichier du package change** ; `OUT_DIR` n'est ni nettoyé ni réinitialisé entre builds. | [R2] `SOURCE_READ` | La granularité conservatrice correcte est le package ; toute revendication « ce fichier n'affecte pas le build script » est non prouvée. |
| C-05 | Les proc-macros sont des cibles `proc-macro` du graphe ; l'expansion elle-même n'est pas représentée comme une entrée fichier. | [R1][R2] `SOURCE_READ` | Modification d'une crate proc-macro ⇒ rebuild transitif des dépendants (Cargo le fait) ; Flow ne doit pas prétendre raffiner. |
| C-06 | Le dep-info rustc (`*.d`) et le fingerprint Cargo sont la vérité fichier, mais ne sont pas exposés par `cargo metadata` ; l'incrémental rustc écrit son état dans le target dir. | [R5][R6] `SOURCE_READ` | Flow ne peut pas reconstruire seul les entrées fichier ; réutiliser l'état natif, et traiter le rebuild Cargo comme autorité. |
| C-07 | Les features d'une dépendance partagée sont l'**union** des features activées par ses dépendants (unification) ; `resolver = "2"` change les règles. | [R3] `SOURCE_READ` | Le contexte de build doit inclure l'ensemble exact des features et le resolver, sinon deux builds distincts partagent la même clé. |
| C-08 | Le partage d'un `target-dir` entre arbres/candidats n'est documenté ni comme sûr ni comme équivalent ; contention, invalidation croisée et verrous sont `UNKNOWN`. | [R4] + `UNKNOWN` | Par défaut : un target dir par contexte ; toute mutualisation doit être mesurée en S9, pas supposée. |

### 2.2 TypeScript

| ID | Constat | Preuve + étiquette | Impact Flow |
|---|---|---|---|
| C-09 | `tsc -b` construit un graphe à partir des **`references` explicites** ; les projets référencés doivent avoir `composite` ; il détecte l'à-jour et ordonne les builds. | [R7] `SOURCE_READ` | Ne jamais inférer un graphe de projets depuis les imports : sans `references`, il n'y a pas de graphe `tsc -b`. |
| C-10 | `paths` remappe les imports vers des emplacements de résolution (relatifs à `baseUrl` ou au `tsconfig`) ; il ne crée aucune arête de projet et peut pointer hors de l'arbre. | [R8] `SOURCE_READ` | Résoudre les alias par tsconfig ; une cible hors tree est externe → `UNKNOWN`/élargissement, sinon le consommateur est manqué. |
| C-11 | Les arêtes purement de types (`import type`, `.d.ts`, `declare module`, `types`/`typeRoots`, conditions `exports`) ne sont pas des imports runtime et restent invisibles à un graphe de fichiers naïf. | [R7][R10] + `PROPOSED` | Un changement de `.d.ts` peut casser un consommateur sans import runtime : typecheck des projets référencés ou full. |
| C-12 | `tsc` n'est ni un bundler ni un test runner : les outils runtime (Vite/webpack/jest/vitest) ont leurs propres graphes et configs ; un `tsc` vert ne prouve pas un bundle/test vert. | [R10] + `PROPOSED` | Un adaptateur TS doit invoquer l'outil natif de test/build ; le critère « TS rouge→vert » de RF-018 est insuffisant. |
| C-13 | Un `import()` à spécificateur **calculé** n'est pas énumérable statiquement ; seul le littéral est typable. | [R10] + `PROPOSED` (conséquence du langage) | Détecter les `import(` non littéraux (heuristique AST) et escalader ; ne jamais promettre l'exhaustivité. |
| C-14 | L'état incrémental `composite` est stocké dans un `.tsbuildinfo` dont l'emplacement dépend de la config ; un tree matérialisé à un autre chemin peut invalider ou brouiller cet état. | [R9] `SOURCE_READ` | Ne pas copier aveuglément `.tsbuildinfo` ; inclure chemins de sortie et racine de projet dans le contexte. |

### 2.3 Python

| ID | Constat | Preuve + étiquette | Impact Flow |
|---|---|---|---|
| C-15 | pytest importe les `conftest.py` et les modules de test pour les exécuter ; un `conftest.py` est un plugin local par répertoire, découvert automatiquement. | [R11][R12] `SOURCE_READ` | La collecte exécute du code arbitraire : un graphe statique ne peut pas prédire les effets ; sélection = heuristique, pas preuve. |
| C-16 | Le mode d'import par défaut (`prepend`) insère le répertoire du module en tête de `sys.path` puis importe via `importlib.import_module` ; le mode change la version du package testée (src layout). | [R11] `SOURCE_READ` | Le même tree peut donner deux verdicts selon `sys.path`, le mode d'import et le paquet installé : ces éléments appartiennent au contexte. |
| C-17 | `importlib.import_module()` et `__import__()` sont des voies d'import documentées, pas seulement l'instruction `import`. | [R13] `SOURCE_READ` | Un nom calculé est invisible statiquement : incomplétude intrinsèque, fallback full suite (C-20). |
| C-18 | Les entry points (`console_scripts`, groupes de plugins) font dépendre le comportement de distributions **installées**, pas du seul tree. | [R14] `SOURCE_READ` | L'environnement (venv, versions installées, plugins) est une entrée du build ; sans lui, `UNKNOWN`. |
| C-19 | `--collect-only` n'est pas une sonde « sans exécution » : la collecte importe les modules (conftest inclus) et exécute donc du code de niveau module. | [R11][R38] + `PROPOSED` | Ne pas utiliser la collecte comme gate bon marché non exécuté ; elle doit être sandboxée et comptée comme exécution. |
| C-20 | Pour le MVP, la politique conservatrice crédible est « **toujours full pytest** » ; la sélection par impact n'est admissible qu'après mesure S9 et avec fallback. | `PROPOSED` | Coût potentiellement faible face au risque de faux vert ; à mesurer, pas à supposer. |

### 2.4 C/C++

| ID | Constat | Preuve + étiquette | Impact Flow |
|---|---|---|---|
| C-21 | Les dépendances d'en-têtes viennent du compilateur : GCC `-MMD/-MF`, MSVC `/sourceDependencies` (JSON), Ninja `deps = gcc` ou `deps = msvc`, qui traite le depfile après compilation. | [R15][R16][R17] `SOURCE_READ` | Le graphe fichier n'existe qu'**après** une compilation ; au premier build, ou pour un header jamais inclus, il n'y a pas d'arête. |
| C-22 | Un header **nouvellement créé** qui serait désormais inclus n'est pas dans le depfile précédent : aucun rebuild n'est déclenché. sccache documente exactement ce cas de résultat périmé silencieux. | [R23] `SOURCE_READ` + `PROPOSED` (transfert du cas) | Création/suppression de header ⇒ rebuild de tous les dépendants ou full ; un graphe antérieur n'est pas une preuve. |
| C-23 | CMake sépare configure et build ; `add_custom_command` ne suit que `DEPENDS`/`DEPFILE` déclarés ; `CMAKE_DEPENDS_USE_COMPILER` (3.20+) délègue les deps au compilateur selon le générateur. | [R18][R19] `SOURCE_READ` | Consommer `build.ninja`/`compile_commands.json` et reconfigurer sur changement de CMakeLists/options/toolchain ; ne pas re-dériver le graphe. |
| C-24 | L'ABI n'est pas visible du graphe source : une modification compatible à la compilation peut casser un consommateur binaire ; la matrice Debug/Release/LTO/sanitizers/MSVC-MinGW multiplie les contextes. | [R28 §4.4] + `PROPOSED` | Pas de réutilisation de cache entre configurations ; un check ABI exige un outil dédié, hors MVP sauf politique explicite. |
| C-25 | Les headers/codes générés par custom commands doivent être des entrées figées ; s'ils sont hors tree ou régénérés, le contexte doit les pinner. | [R18] + `PROPOSED` | Entrées générées non figées ⇒ `UNKNOWN`/full, jamais PASS silencieux. |
| C-26 | Sur Windows, les deps MSVC passent par `/sourceDependencies` (JSON), avec casse insensible, longueur de chemins et PDB ; les garanties Linux ne se transposent pas. | [R17][R28 §9 OS-18] `SOURCE_READ` | L'adaptateur doit être qualifié par OS ; aucune revendication Windows sans test OS (H3). |
| C-27 | `compile_commands.json` est une base de **commandes**, pas un graphe de dépendances ; il ne remplace ni depfiles ni build.ninja. | [R19] + `PROPOSED` | Ne pas confondre « je sais compiler » et « je sais ce qui dépend de quoi ». |

### 2.5 Go

| ID | Constat | Preuve + étiquette | Impact Flow |
|---|---|---|---|
| C-28 | `go list -deps` parcourt les dépendances en post-ordre ; le graphe dépend du contexte de build (GOOS/GOARCH/tags). | [R20] `SOURCE_READ` | Graphe fiable **si** le contexte est figé ; sinon deux listes pour le même tree. |
| C-29 | Les contraintes `//go:build` et les suffixes de nom de fichier changent l'ensemble des fichiers du package. | [R21] `SOURCE_READ` | Inclure tags/OS/arch dans la clé ; un fichier `_windows.go` n'existe pas pour une cible Linux. |
| C-30 | `go list -test` rapporte aussi les binaires de test et leurs dépendances, distincts du build normal. | [R20] `SOURCE_READ` | La sélection de tests exige le graphe `-test`, sinon des erreurs de compilation `_test.go` sont manquées. |
| C-31 | `go generate` est une commande distincte, non lancée par `go build` ; `//go:embed` ajoute des fichiers non-imports (exposés par `go list -json`). | [R20] `SOURCE_READ` | Fichiers générés/embeddés à pinner ; sinon `UNKNOWN`. |
| C-32 | Go fournit déjà un cache de build/test documenté ; cgo dépend de `CGO_ENABLED`/`CC` et change les sorties. | [R20] `SOURCE_READ` | Déléguer le cache à Go ; inclure l'environnement cgo dans le contexte ; ne pas réimplémenter. |

### 2.6 Cache, invalidation, poisoning

| ID | Constat | Preuve + étiquette | Impact Flow |
|---|---|---|---|
| C-33 | La clé V6 « tree + context + toolchain + target + flags + dependencies + policy + execution boundary » est **nécessaire mais non suffisante** : manquent env lus par build scripts/tests, cwd (chemins embarqués), bibliothèques système, macros temporelles, réseau, fixtures, version d'adaptateur et schéma de cache. | [R28 §5.3] + [R2][R23] `SOURCE_READ` + `PROPOSED` | Une clé « exacte » annoncée sans ces entrées autorise des faux hits. Doit être corrigée avant D2. |
| C-34 | sccache local : un seul serveur à la fois (serveurs concurrents = courses et échecs), cache disque 10 Go par défaut, mode préprocesseur pouvant produire des résultats **périmés silencieux** (header absent au moment du cache, macros temporelles, facteurs externes inconnus), `hash_working_directory` actif par défaut. | [R22][R23] `SOURCE_READ` | Un seul écrivain de cache ; pas de parallélisme de cache ; conserver les options conservatrices ; sinon faux vert. |
| C-35 | Bazel/Buck2 : le cache d'actions repose sur des entrées/sorties/commande/env **déclarées** par action ; la réutilisation est correcte seulement si l'action est hermétique. | [R24][R25] `SOURCE_READ` | Non transposable à Cargo/pytest/npm sans réécrire les règles hermétiques : hors MVP solo. |
| C-36 | Buck2 appartient à la même famille « graphe d'actions + cache distant » ; ses détails ne sont pas revus ici. | [R25] `NOT_REVIEWED` | Ne pas l'invoquer comme preuve ; même conclusion que C-35. |
| C-37 | Poisoning : le cache local est inscriptible par tout processus du compte ; build scripts et tests exécutent du code arbitraire ; aucune intégrité cryptographique n'est documentée dans les pages consultées ; le mode read-only existe comme garde partielle. | [R23] + `PROPOSED` + `NOT_REVIEWED` (absence) | Namespaces par classe de confiance ; un candidat untrusted **n'écrit jamais** dans le cache partagé ; vérifier les hashes d'artefacts avant usage. |
| C-38 | Un résultat de test requis ne doit pas être servi depuis le cache : un test peut être non déterministe et un cache hit n'est pas une exécution dans ce contexte. | [R28 §5.3][R30 E2E-13] + `PROPOSED` | E2E requis toujours exécuté (ou `UNSUPPORTED` visible) ; cache réservé aux checks non requis. |
| C-39 | Lattice honnête : `PASSED` ssi toutes les obligations **requises** ont été exécutées sur la clé exacte, avec artefact/preuve ; `STALE/UNKNOWN/UNSUPPORTED/TIMEOUT/FAILED/SKIPPED` ne satisfont jamais. | [R28 §4.1–4.2][R36] `SOURCE_READ` | Implémenter un enum fermé ; « impacted green » interdit par construction. |
| C-40 | Les caches natifs existent déjà (Cargo target, `.tsbuildinfo`, `.ninja_deps`, GOCACHE) ; un second cache Flow crée deux autorités qui divergent. | [R1][R5][R9][R15][R20] `SOURCE_READ` | Faire du cache natif l'autorité ; Flow ne cache que des **décisions/receipts** liés à l'état natif. |
| C-41 | Toute clé doit être versionnée (`cache_schema_version`, versions d'adaptateur/outils) ; sinon un changement de sémantique réutilise silencieusement d'anciennes entrées. | `PROPOSED` | Politique de migration/éviction obligatoire avant tout cache. |
| C-42 | Un cache partagé/remote pour un solo offline n'apporte rien et ajoute poisoning/éviction/coût ; RF-025 est hors MVP. | [R24] + `PROPOSED` | Différer ; aucune promesse de « cache partagé » dans le MVP. |

### 2.7 E2E, CI et requalification

| ID | Constat | Preuve + étiquette | Impact Flow |
|---|---|---|---|
| C-43 | La merge queue GitHub exige une protection/ruleset ; elle teste le PR **avec la dernière base et les PR déjà en file** via l'événement `merge_group` ; un échec ou un conflit retire le PR. | [R26] `SOURCE_READ` | Un receipt au SHA de la tête de PR est insuffisant : requalifier au tree du merge group. |
| C-44 | Squash/rebase/merge créent de nouveaux commits ; la merge queue combine des changements et peut donc changer l'arbre. Un rebase à base inchangée conserve généralement le tree. | [R26] `SOURCE_READ` | Lier le receipt à `tree + context`, pas au SHA ; réutilisation si tree identique = politique explicite seulement. |
| C-45 | État réel de Flow : 0 workflow, PR #33 ouverte, aucune protection (baseline `gh` authentifié, 404), 47 issues. | [R34][R33] `API_READ` | Aucun gate distant aujourd'hui : le receipt local est le seul gate ; ne pas présupposer la merge queue. |
| C-46 | « Impacted green » est injustifié : un consommateur inchangé peut casser (API/ABI, `.d.ts`, migration, protobuf, config/env) sans être dans le sous-ensemble testé. | [R28 §5.3][R30 E2E-13] `SOURCE_READ` | E2E requis bloquants au tree final ; un sous-ensemble vert ne vaut jamais « tout vert ». |
| C-47 | E2E hors ligne : services/DB/réseau souvent nécessaires ; sans sandbox/prérequis, le statut honnête est `UNSUPPORTED`, jamais `PASSED`. | [R28 §5.1][R30] `SOURCE_READ` | La politique déclare les prérequis E2E et échoue fermé. |
| C-48 | Retries/timeouts : un PASS après retry doit porter le nombre de tentatives ; un timeout n'est pas un pass. | [R28][R36] `SOURCE_READ` | Les receipts doivent inclure tentatives, durée, raison d'échec. |
| C-49 | Provenance minimale d'un check : ligne de commande exacte, versions d'outils, hash d'environnement, hash d'artefact. | [R28 §4.2][R36] `SOURCE_READ` | Sans cela, un check vert n'est pas reproduisible et ne vaut pas preuve. |
| C-50 | Le coût du full E2E par candidat peut dominer ; la sélection par impact ne paie que si le full est cher. | `PROPOSED` + [R29] | Mesurer d'abord (S9) ; ne pas construire graphe/cache avant preuve de gain. |

### 2.8 Coût réaliste (16 Go, offline, Windows/Linux)

| ID | Constat | Preuve + étiquette | Impact Flow |
|---|---|---|---|
| C-51 | Machine mesurée : 15,7 GiB, 10C/16T, 188 Go libres ; Go absent, MSVC présent hors PATH. | [R35] `MEASURED` | Formule de workers et limites par capacité réelle, pas par nombre d'agents. |
| C-52 | Matérialisation : `git worktree` ≈ checkout (métadonnées + fichiers, objet partagé) ; copie complète dominée par `node_modules`/`target` ; hardlinks dangereux (écriture partagée) ; sur Windows beaucoup de petits fichiers = coût métadonnées. | `PROPOSED` + [R4] | Préférer worktree/sparse ; mesurer le coût par fichier ; ne pas copier les caches. |
| C-53 | Temps de compilation incrémentale : dominé par taille de crate, codegen et link ; aucune valeur publiable sans mesure. | `UNKNOWN` + [R29] | S9 doit mesurer cold/warm par écosystème ; aucune promesse chiffrée avant. |
| C-54 | Plus de parallélisme peut augmenter le wall-clock (contention) ; il faut des budgets **agrégés** CPU/RAM. | [R28 §5.4] `SOURCE_READ` | Un gouverneur de ressources est nécessaire même sans ordonnanceur de graphe. |
| C-55 | Disque : target dirs + caches + worktrees peuvent saturer ; un disque plein produit `FAILED`, pas `PASSED`. | `PROPOSED` + [R35] | Quotas, rétention, surveillance ; GC borné. |

### 2.9 Audit des tickets

| ID | Ticket | Constat | Étiquette |
|---|---|---|---|
| T-01 | **RF-012** (adapter Rust) | Ticket utile mais **sous-spécifié** : manquent workspace vs package, `--all-targets`/doctests/benches, matrice de features, triple, build scripts/proc-macros, `--locked --offline`, profil, `CARGO_TARGET_DIR`/`RUSTFLAGS`, toolchain absent ⇒ `UNSUPPORTED`. À scinder en 012a (fidélité du graphe) / 012b (exécution + capture). Reste bloqué par GO. | `PROPOSED` |
| T-02 | **RF-013** (scheduler) | **Prématuré** : l'appropriation de l'ordonnanceur n'est pas tranchée (§3) ; « coalescence OFF » n'est pas défini ; chevauche RF-015 (« quoi lancer ») et V6 §5.4 (budgets). Critères manquants : formule de concurrence, cap RAM par job, kill d'arbre de processus (Job Objects Windows / pgid Linux), préemption stale, retries, logs bornés. | `PROPOSED` |
| T-03 | **RF-014** (cache/identité) | **Prématuré** et **chevauchant RF-012** : la clé listée est incomplète (C-33) ; manquent `cache_schema_version`, politique de poisoning, intégrité d'artefact, éviction, partage par classe de confiance, comportement si toolchain change, distinction required/optional. Renommer « identité de contexte + binding receipt » et séparer le cache. | `PROPOSED` |
| T-04 | **RF-015** (CI impact + E2E + merge suite) | **Prématuré/ambigu** : confond gate local, CI foraine et merge queue ; « merge suite » non défini ; aucune gouvernance de branche n'existe (C-45). Critères manquants : requalification au tree final, `merge_group`, squash/rebase, liste E2E requise, timeouts/retries, provenance, interdiction du « impacted green ». À scinder local/forge/E2E. | `PROPOSED` |
| T-05 | **RF-018** (TypeScript) | **Prématuré** (R5 alors que le contrat d'adaptateur est non ratifié) ; critère « TS rouge→vert + lockfile » insuffisant : project references/`composite`, `paths`, `import()` calculé, bundler et test runner, workspaces monorepo, ESM/CJS, `.tsbuildinfo`, version Node, lockfile du package manager. | `PROPOSED` |
| T-06 | **RF-019** (Python) | **Prématuré** ; « imports dynamiques fallback conservateur » n'est pas un critère : définir le déclencheur, le full-suite par défaut, `conftest`/plugins/entry points, venv/interpréteur, mode d'import, doctests, xdist. Simplification recommandée : full pytest en MVP (C-20). | `PROPOSED` |
| T-07 | **RF-020** (C/C++) | **Le plus prématuré** : « headers/options/ABI invalidations » est un slogan. Manquent configure vs build, générateur, depfiles, headers générés, matrice d'options, toolchain files, cross-compilation, MSVC/Clang, modules C++20, PCH. Scinder CMake/Ninja (R5+) et ABI (hors MVP). | `PROPOSED` |
| T-08 | **RF-003** (S8 A/B/C) | **Confirmation** : `BACKLOG.md` exige la reprise sous S9 ; S8 n'était ni worktrees ni Flow et reste spécifique à sa fixture. Le critère « baselines reproductibles » exige le harness d'origine, absent du dépôt ⇒ soit le retrouver, soit re-dériver. `MEASURED_HISTORICAL` via V6 seulement. | `SOURCE_READ` [R31] |
| T-09 | **RF-022** (S8 D vs alternatives) | **Confirmation** de reprise sous S9, mais **incohérence** : dépendances originales RF-016/015 (R5) alors que le mapping proposé est D3/R0. Corriger les dépendances ou le mapping ; sinon le ticket est soit bloqué par R5, soit exécutable en R0, pas les deux. | `SOURCE_READ` [R31] |
| T-10 | Doublons | RF-012/RF-014 (identité de graphe vs cache) ; RF-013/RF-015 (quoi lancer) ; RF-015/RF-021 (merge suite vs publication forge) ; RF-018/019/020 partagent le **même** contrat d'adaptateur ; RF-003/RF-022 forment une paire pré-enregistrement/exécution. Conserver les IDs, fusionner les contenus par renvoi. | `PROPOSED` |
| T-11 | Critères manquants transverses | Aucun ticket ne dit : toolchain absente ⇒ `UNSUPPORTED` ; offline strict ; comportement Windows ; déterminisme ; format de preuve ; stop condition ; test de falsification ; budget 16 Go. À ajouter avant toute implémentation. | `PROPOSED` |
| T-12 | Statut | Tous restent `BACKLOG / BLOCKED-BY-GO` ; une issue ouverte n'est pas une autorisation (AGENTS.md, BACKLOG §Gestion). | `SOURCE_READ` [R36][R31] |

### 2.10 Règle de bascule vers la validation complète

Basculer en validation **complète** (toutes les obligations requises sur l'arbre exact) dès qu'un des cas suivants est vrai :

1. changement de configuration de build : `Cargo.toml`/`Cargo.lock`, `tsconfig*`, `pyproject.toml`/lock, `CMakeLists.txt`/presets/toolchain, `go.mod`/`go.sum`, build tags, flags, profile, target triple ;
2. build script, proc-macro, plugin pytest, bundler, entry point, `go generate` ou codegen concerné ;
3. header ajouté/supprimé/déplacé, fichier généré ou `OUT_DIR` non figé, `embed` non pinné ;
4. arête absente du graphe déclaré (projet TS non référencé, alias hors arbre, package Python non importé statiquement) ;
5. import dynamique calculé détecté, ou outil d'analyse en échec ;
6. toolchain absente ou version non qualifiée ⇒ `UNSUPPORTED` (jamais `PASSED`) ;
7. E2E requis par la politique, ou changement d'API/ABI/IDL/schéma ;
8. merge queue/rebase/squash produisant un tree ou un contexte différent ;
9. premier run sur un dépôt/cache inconnu, ou entrée `UNKNOWN` non résolue.

La « validation complète » signifie **toutes les obligations requises**, pas nécessairement tous les tests
imaginables : la politique peut borner, mais elle doit rendre visibles les checks exécutés et évités.

---

## 3. Réponse contradictoire — Flow doit-il posséder l'ordonnanceur de builds ?

### 3.1 Thèse adverse (la plus forte)

1. **Obligations inter-écosystèmes** : dans un mono-repo Rust + TS + Python + C++, aucun outil natif ne sait
   ordonner build → tests unitaires → intégration → E2E → packaging. Il faut bien un chef d'orchestre.
2. **Cache portable** : les caches natifs sont opaques et liés au target dir ; seul un `BuildContextId`
   Flow permettrait de réutiliser un résultat entre arbres/candidats sans faux hit.
3. **Budgets et préemption** : sans posséder l'exécution, Flow ne peut ni annuler un job périmé, ni empêcher
   la famine RAM sur 16 Go — c'est précisément le problème qu'il doit résoudre.
4. **Preuve d'exécution** : un receipt doit lier les commandes réellement lancées à l'arbre ; déléguer
   aveuglément revient à croire un code de sortie.
5. **Gain de wall-clock** : la coalescence et la préemption sont les vrais leviers quand plusieurs agents
   compilent en parallèle (S8 : le dossier partagé est plus lent que les copies).

### 3.2 Réfutation

1. **Un DAG d'obligations n'est pas un graphe de compilation.** Il a quelques nœuds (build, unit, intégration,
   E2E, package) ; Cargo/tsc/CMake/Go exécutent leur propre DAG en interne. Flow peut ordonner les
   *obligations* sans ordonnancer les compilations.
2. **Un cache natif opaque reste plus sûr qu'un second cache.** Flow peut lier le receipt à l'état natif,
   relancer la commande autoritaire en cas de doute, et ne revendiquer un cache portable qu'après mesure S9.
   La portabilité est une optimisation, pas une condition de correction.
3. **Budgets et annulation = supervision de processus, pas ordonnancement.** Flow peut imposer une
   concurrence agrégée, tuer un arbre de processus (Job Objects/pgid), préempter un job stale et refuser
   d'en lancer un de plus, sans connaître la topologie des crates.
4. **Un receipt honnête n'exige pas la propriété de l'ordre** : commande exacte, versions, hash d'env,
   code de sortie, hash d'artefact suffisent à prouver ce qui a tourné.
5. **Les gains de coalescence sont spéculatifs sur un solo offline.** Bazel/Buck2 ciblent des flottes ;
   leur cache distant n'a de sens qu'avec des actions hermétiques et des machines multiples.

### 3.3 Verdict (position de l'agent C)

**Non, Flow ne doit pas posséder l'ordonnanceur de builds ni le moteur de cache dans le MVP.** Il doit
posséder quatre choses, et seulement quatre :

| Flow possède | Flow délègue |
|---|---|
| Le **graphe d'obligations** (quels checks sont requis pour ce tree + contexte + politique) | La topologie et l'ordre **intra-outil** à Cargo/tsc/CMake-Ninja/Go/pytest |
| La **supervision d'exécution** : concurrence agrégée, timeouts, kill d'arbre, annulation, env allowlist | Le **parallélisme** et l'ordonnancement interne aux outils natifs |
| Le **receipt** lié à tree + BuildContextId + ExecutionBoundaryId + PolicyVersion, avec preuves | Le **cache d'artefacts** aux caches natifs (Cargo, Go, Ninja, `.tsbuildinfo`) ; sccache seulement en accélérateur optionnel |
| La **décision d'escalade** (ciblé → élargi → complet) et la **requalification** au tree final | La **CI distante** et la merge queue, quand elles existent et sont gouvernées |

La frontière exacte : Flow est un **vérificateur d'obligations avec exécution supervisée**, pas un build
system. Il peut *lire* un graphe natif pour proposer un ensemble d'impact, mais cet ensemble est une
**heuristique** : la preuve reste l'exécution de l'outil natif (ou l'escalade).

### 3.4 Ce qui renverserait ce verdict

- S9 démontre qu'un ordonnanceur Flow réduit le temps jusqu'au livrable vérifié de plus que le seuil H4,
  à sûreté et coût de maintenance égaux ou inférieurs.
- Un cas réel où les obligations inter-écosystèmes ne peuvent pas être exprimées comme un petit DAG
  au-dessus des outils natifs.
- Une mesure montrant qu'un cache portable Flow évite des reconstructions majeures **sans** aucun hit
  incorrect dans le corpus adversarial CACHE-13.

Sans ces preuves, posséder l'ordonnanceur est une complexité prématurée et un risque de faux vert.

---

## 4. Désaccords réels

| ID | Désaccord | Position de l'agent C | Référence |
|---|---|---|---|
| D-01 | **Propriété de l'ordonnanceur.** V6 §1.3 attribue à Flow « l'exécution ordonnancée des validations » et RF-013 spécifie un scheduler ; j'argumente pour un **graphe d'obligations + supervision** seulement. | Désaccord assumé : le texte V6 surestime ce que Flow doit posséder ; la supervision est indispensable, le scheduler de compilations ne l'est pas. | [R28 §1.3, §3.2, §5.4] ; [R31 RF-013] |
| D-02 | **Clé de cache « exacte ».** V6 §5.3 présente une liste de composants comme clé de cache ; cette liste omet env, cwd, bibliothèques système, macros temporelles, fixtures, schéma/version. | La liste est nécessaire mais non suffisante ; sans politique d'entrées inconnues, elle produit des faux hits. Corriger avant D2. | [R28 §5.3] ; [R23] |
| D-03 | **Graphe conservateur pour Python/TS runtime.** V6 §5.3 dit « modifié + consommateurs transitifs inchangés » ; pour Python, les consommateurs ne sont pas énumérables (imports dynamiques, plugins) et pour TS le runtime (bundler/runner) n'est pas le graphe `tsc`. | Pour Python, le défaut MVP doit être le **full suite**, pas un graphe ; pour TS, l'adaptateur doit invoquer le runner réel, pas seulement `tsc`. | [R13][R11][R10] |
| D-04 | **Merge queue supposée disponible.** RF-015 et une partie de la V6 raisonnent comme si une merge queue/CI foraine existait ; Flow n'a aucun workflow, la PR #33 est ouverte et la protection est absente. | La requalification au SHA final est correcte **comme exigence conditionnelle** ; elle ne peut pas être un critère d'acceptation actuel. La gouvernance de branche est une décision humaine, pas un ticket d'implémentation. | [R26][R34][R33] |
| D-05 | **Ambiguïté baseline « aucun workflow Actions ».** La baseline fournie au sous-agent peut se lire comme visant Ravum ; or Ravum a **3 fichiers** de workflow au SHA épinglé, Flow en a **0**. | Corriger la formulation : Flow = 0 workflow (vérifié) ; Ravum = 3 workflows observés (API, avant rate-limit). Ne pas confondre les deux dépôts. | [R34] |
| D-06 | **RF-022 incohérent.** Dépendances originales RF-016/015 (R5) vs mapping proposé D3/R0. | Le ticket doit être soit un pré-enregistrement S9 en R0, soit une exécution après R5 ; pas les deux. Choisir et mettre à jour la traçabilité. | [R31] |

---

## 5. Recommandations bornées + falsification

Chaque recommandation est **documentaire ou de contrat** : aucune n'autorise un build, un test ou du code.

| ID | Recommandation bornée | Falsification / test négatif |
|---|---|---|
| R-01 | **Ne pas construire d'ordonnanceur de builds en R1–R3.** Flow exécute les outils natifs via un superviseur (timeouts, kill d'arbre, concurrence agrégée, env allowlist) et émet des receipts. | S9 : le superviseur ajoute > seuil H4 au wall-clock, ou ne peut pas tuer un arbre de processus sous Windows (test `RESOURCE-14`). |
| R-02 | **Contrat d'adaptateur minimal** : capacités déclarées, détection toolchain (chemin + version + hash), commande de graphe, commandes build/test, parsing de sortie, toolchain absente ⇒ `UNSUPPORTED`. | Un écosystème ne peut pas exprimer ses obligations sans logique spécifique Flow, ou le contrat dépasse ~10 champs. |
| R-03 | **Politique d'impact = sur-ensemble + escalade** : Rust/Go/C++ au niveau package/header via graphe natif ; Python full par défaut ; TS via projets référencés + runner réel. | Un test `E2E-13`/`TREE-01` produit un vert avec un consommateur cassé, ou le full coûte plus que le seuil en S9. |
| R-04 | **Cache : natif d'abord.** Pas de cache d'artefacts Flow en MVP ; sccache optionnel, un seul serveur, options conservatrices, read-only pour l'untrusted. Flow ne cache que des **décisions/receipts**. | Un stale-hit est reproduit (`CACHE-13`), ou le gain mesuré est nul. |
| R-05 | **Clé corrigée** : `cache_schema_version`, versions/hashes d'outils, lignes de commande, cwd/mode, hash d'env allowlist, lockfiles, triple/features/profil/générateur, libs système déclarées, politique, boundary. | Deux contextes différant d'une de ces entrées produisent la même clé, ou un artefact incorrect dans `CACHE-13`. |
| R-06 | **Requalification** : receipt lié à `tree + context` ; réutilisation si tree identique seulement par politique explicite ; checks requis au tree du merge group quand la gouvernance existe. | La merge queue change le tree sans requalification, ou la réutilisation same-tree échoue `E2E-12`. |
| R-07 | **Ne pas adopter Bazel/Buck2/REAPI en MVP** ; garder O0/O1 comme comparateurs ; réévaluer seulement si S9 montre un besoin et si des règles hermétiques existent. | Intégration mesurée < budget et gain > seuil, offline, sur le corpus S9. |
| R-08 | **S9 d'abord** : pré-enregistrer workloads par écosystème, cold/warm, plafond 16 Go, paires, stop rules ; rejouer RF-003/RF-022 ; aucune optimisation avant résultats. | Les différences appariées ne sont pas significatives, ou O0 gagne : ne rien construire. |
| R-09 | **Hygiène des tickets** : scinder RF-012 (graphe/exécution), RF-015 (local/forge/E2E), RF-014 (contexte/cache) ; lier RF-018/019/020 à un contrat unique ; corriger RF-022 ; ajouter les AC de T-11. | Un AC reste intestable ou non falsifiable ⇒ le réécrire avant R1. |
| R-10 | **Qualification OS** : depfiles MSVC (`/sourceDependencies`), Job Objects, casse/chemins Windows, PDB ; aucune revendication Windows sans test OS. | Un test d'annulation laisse un `rustc`/`msbuild` orphelin, ou les chemins échouent sur casse/Unicode. |
| R-11 | **Budget de workers par défaut** : 2 jobs lourds Rust max tant que l'autre session Rust tourne, 4 jobs légers TS/Python, plafond agrégé mesuré ; formule `min(cœurs, RAM_dispo / RSS_job)` avec RSS mesurée. | La RSS mesurée permet 4 jobs lourds sans swap sur les workloads S9, ou 2 jobs saturent déjà. |
| R-12 | **Coût de matérialisation mesuré** : worktree/sparse par défaut, pas de copie de `node_modules`/`target`, pas de hardlinks ; overhead cible < 5 % du wall-clock chaud. | L'overhead dépasse le seuil sur des changements d'un fichier, ou O0 (full test natif) est plus rapide. |

---

## 6. Limites / NOT_REVIEWED

| Sujet | Statut | Raison |
|---|---|---|
| Temps de compilation incrémentale réels, RSS des compilateurs, wall-clock | `UNKNOWN` | Interdiction de build/test ; aucune mesure possible dans ce mandat. |
| Comportement sccache/Bazel/Buck2 en conditions réelles sur ce dépôt | `NOT_REVIEWED` | Pas de fixture autorisée, pas de GO_R0_FIXTURES. |
| Détails Buck2 (cache, REAPI) au-delà de la page « Remote Execution » | `NOT_REVIEWED` | Page consultée mais contenu détaillé non extrait ; rate-limit GitHub ensuite. |
| Go sur la machine | `NOT_REVIEWED` | Toolchain absente ; aucune vérification possible. |
| WSL / sandbox Windows (Job Objects, AppContainer) | `NOT_REVIEWED` | WSL a bloqué ; sandbox non testée (H3 appartient à l'agent B). |
| Protection de branche Flow | `SOURCE_READ` via baseline | Vérifiée par `gh` authentifié (404) dans `DEEPSEEK-00` ; mon appel non authentifié a renvoyé 401. |
| Workflows Ravum détaillés | `API_READ` partiel | 3 fichiers observés ; noms/contents non récupérés (rate-limit 403 ensuite). |
| Code Ravum (git_writer, etc.) | `NOT_REVIEWED` ici | Hors périmètre build/CI ; lecture déléguée aux agents A/B/D. |
| Licences/droits | `NOT_REVIEWED` ici | Hors périmètre ; traité par l'agent D / baseline. |
| Java/.NET/autres écosystèmes | `NOT_REVIEWED` | Le mandat cite 5 écosystèmes ; l'absence de revue est une limite si « universel » est revendiqué. |
| Coût réel des tokens LLM dans S9 | `NOT_REVIEWED` | Aucun run d'agents. |
| Fiabilité des chiffres S8 | `MEASURED_HISTORICAL` via V6 | Artefacts de mesure absents du dépôt ; non reproductibles ici. |
| `compile_commands.json` comme graphe | `PROPOSED` | Affirmation de conception, non mesurée. |

**Limite générale :** ce rapport est une analyse documentaire et de conception. Aucune de ses affirmations
de performance ou de sûreté n'est une mesure ; les étiquettes `PROPOSED`/`UNKNOWN` doivent être conservées
dans la consolidation.

---

## 7. Références

Sources primaires consultées le **2026-11-09** (Europe/Paris), sauf mention contraire.

| ID | Source | Usage |
|---|---|---|
| R1 | Cargo Book — `cargo metadata` : https://doc.rust-lang.org/cargo/commands/cargo-metadata.html | Schéma, `--filter-platform`, `--no-deps`, `--locked`, `--offline` |
| R2 | Cargo Book — Build Scripts : https://doc.rust-lang.org/cargo/reference/build-scripts.html | `rerun-if-changed`, `OUT_DIR`, rerun par défaut |
| R3 | Cargo Book — Features : https://doc.rust-lang.org/cargo/reference/features.html | Unification, resolver v2 |
| R4 | Cargo Book — Config : https://doc.rust-lang.org/cargo/reference/config.html | `build.target-dir` |
| R5 | Cargo Book — Profiles : https://doc.rust-lang.org/cargo/reference/profiles.html | `incremental` |
| R6 | rustc — Codegen options : https://doc.rust-lang.org/rustc/codegen-options/index.html | Incrémental / unités de codegen |
| R7 | TypeScript — Project References : https://www.typescriptlang.org/docs/handbook/project-references.html | `tsc -b`, `composite`, `references` |
| R8 | TypeScript — `paths` : https://www.typescriptlang.org/tsconfig/paths.html | Alias de résolution |
| R9 | TypeScript — `tsBuildInfoFile` : https://www.typescriptlang.org/tsconfig/tsBuildInfoFile.html | État incrémental |
| R10 | TypeScript — Modules Reference : https://www.typescriptlang.org/docs/handbook/modules/reference.html | `import()`, résolution |
| R11 | pytest — Import mechanisms and `sys.path` : https://docs.pytest.org/en/stable/explanation/pythonpath.html | Modes d'import, `conftest` |
| R12 | pytest — Fixtures / `conftest.py` : https://docs.pytest.org/en/stable/reference/fixtures.html | Plugins locaux par répertoire |
| R13 | Python — The import system : https://docs.python.org/3/reference/import.html | `importlib.import_module`, `__import__` |
| R14 | Python Packaging — Entry points : https://packaging.python.org/en/latest/specifications/entry-points/ | `console_scripts`, groupes de plugins |
| R15 | Ninja — Manual (depfile, deps, `-t deps`) : https://ninja-build.org/manual.html | Dépendances d'en-têtes |
| R16 | GCC — Preprocessor Options (`-MMD`, `-MF`) : https://gcc.gnu.org/onlinedocs/gcc/Preprocessor-Options.html | Depfiles |
| R17 | MSVC — `/sourceDependencies` : https://learn.microsoft.com/en-us/cpp/build/reference/sourcedependencies?view=msvc-170 | Dépendances JSON, headers/PCH/modules |
| R18 | CMake — `add_custom_command` : https://cmake.org/cmake/help/latest/command/add_custom_command.html | `DEPENDS`, `DEPFILE` |
| R19 | CMake — `CMAKE_DEPENDS_USE_COMPILER` : https://cmake.org/cmake/help/latest/variable/CMAKE_DEPENDS_USE_COMPILER.html | Dépendances générées par le compilateur |
| R20 | Go — `cmd/go` : https://go.dev/cmd/go/ | `list -deps`, `-test`, `generate`, `embed`, cache, cgo |
| R21 | Go — `go/build` (build constraints) : https://pkg.go.dev/go/build | `//go:build`, suffixes de fichiers |
| R22 | sccache — README : https://github.com/mozilla/sccache | Cache de compilation, backends, wrapper |
| R23 | sccache — `docs/Local.md` : https://github.com/mozilla/sccache/blob/main/docs/Local.md | Serveur unique, stale silencieux, `hash_working_directory`, read-only |
| R24 | Bazel — Remote Caching : https://bazel.build/remote/caching | Actions déclarées, réutilisation d'outputs |
| R25 | Buck2 — Remote Execution : https://buck2.build/docs/users/remote_execution/ | Famille graphe d'actions (détails `NOT_REVIEWED`) |
| R26 | GitHub — Managing a merge queue : https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/configuring-pull-request-merges/managing-a-merge-queue | `merge_group`, checks sur base + file, méthodes |
| R27 | GitHub — Events that trigger workflows (`merge_group`) : https://docs.github.com/en/actions/reference/events-that-trigger-workflows#merge_group | Déclenchement CI |
| R28 | V6 locale : `docs/reviews/RAVUM-FLOW-V6-POST-CLAUDE-REFERENCE.md` (l. 1–460) | Contrats tree/context/receipt, §5.3, §5.4, §6, §9 |
| R29 | `docs/BENCHMARK_S9.md` | Comparateurs, métriques, stop rules |
| R30 | `docs/SECURITY_TESTS.md` | `E2E-13`, `CACHE-14`, `TREE-01`… |
| R31 | `docs/BACKLOG.md`, `docs/ROADMAP.md` | RF-012..015, RF-018..020, RF-003/022, mapping |
| R32 | `docs/adr/FLOW-ADR-007.md`, `docs/rfc/RF-RFC-008.md` | Décisions candidates build/CI |
| R33 | `docs/reviews/DEEPSEEK-00-BASELINE.md` | Faits de dépôt, protection, 47 issues |
| R34 | API GitHub publique (`Rwanbt/Ravum-flow`, `Rwanbt/Ravum`), 2026-11-09 : repo public, 47 issues + PR #33 ouverte, 0 workflow Flow, 3 workflows Ravum, licence absente | `API_READ` |
| R35 | Sondage local lecture seule, 2026-11-09 : RAM 15,7 GiB, i7-13620H 10C/16T, D: 188,4 Go libres, versions cargo/rustc 1.95.0, node 26.13.0, npm 11.20.0, Python 3.13.7/pytest 9.1.1, CMake 4.1.1, Ninja 1.13.1, sccache 0.18.0, Git 2.51.1, VS 2026 (toolset C++), Go absent | `MEASURED` |
| R36 | `AGENTS.md` | Règles de preuve, statuts non promouvables, ressources |
| R37 | `docs/DEEPSEEK_GOAL.md` | Périmètre, interdictions, exigences de sources |
| R38 | pytest — How to invoke pytest (`--collect-only`) : https://docs.pytest.org/en/stable/how-to/usage.html | Existence de la collecte seule |

---

*Fin du rapport brut C. Aucun build, test, benchmark, CI, migration ou écriture hors de ce fichier n'a été
effectué. Les affirmations `PROPOSED`/`UNKNOWN` ne doivent pas être promues en preuves par la consolidation.*
