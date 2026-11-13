# R0-b — Sémantique de vérification (agent C)

**Date :** 2026-11-09 (Europe/Paris). **Branche observée :** `feat/flow-verified-foundation`,
HEAD `19cd880f5b554b1adc4a4d5f886d1911b4922951` (lecture seule ; aucun commit, aucun `git add`).
**Statut :** résultats d'exécution sur fixtures jetables ; **aucune** preuve générale, **aucune** autorisation.

---

## 1. Mission et méthode

Exécuter les cinq tests négatifs de sémantique de vérification demandés par le Lead :

| Test | Objet |
|---|---|
| `E2E-12` | Un même consommateur passe sur `main` et échoue sur `breaking` ; une politique naïve produit un faux vert ; une gate qui exige le code retour bloque. |
| `CACHE-13` | Un artefact dépend d'une entrée non déclarée ; `context_id` inchangé, artefact différent ⇒ empoisonnement de cache. |
| `CONTEXT-03` | Une dépendance de build non listée rend la portée de validation indéterminable (complétude non prouvable). |
| `LEGACY-06` | Un patch s'applique textuellement mais casse la sémantique ; `AUTO_SAFE` textuel ≠ validation. |
| `RECOVERY-16` | Journal tronqué en cours de ligne : lecteur naïf (crash ou reprise silencieuse) vs lecteur robuste (refus). |

Méthode : `run_tests.py` recrée les fixtures dans `scratch/`, exécute les commandes avec un
environnement Git privé, capture les sorties, les rédige (chemins utilisateur remplacés) et écrit
`RESULTS.json`, `logs/<id>.log`, `logs/<id>.json`. **Python et `git` uniquement** : aucun compilateur,
aucun build system, aucun réseau, aucun secret réel.

Règles respectées : écriture uniquement sous `experiments/r0b/verification/` ; aucun accès en écriture à
`experiments/r0a/`, à `.git` du dépôt Flow ou à `Rwanbt/Ravum` ; aucun commit dans le dépôt Flow ; les
commits créés le sont uniquement dans les dépôts jetables de `scratch/`.

---

## 2. Environnement mesuré

| Élément | Valeur |
|---|---|
| OS | win32 / Windows-11 / `Windows-11-10.0.26100-SP0` |
| Python | 3.13.7 (`sys.executable`) |
| Git | 2.51.1.windows.1 |
| CPU | 16 processeurs logiques (`os.cpu_count()`) |
| Réseau | non utilisé (`network_used: false`) |
| Compilateur | non utilisé (`compiler_used: false`) |

Machine d'audit Windows uniquement : **aucun résultat POSIX n'est revendiqué**.

---

## 3. Provenance des fixtures (point important)

Le harnais R0-a a été validé par le Lead : `experiments/r0a/out/control.json` porte
`"passed": true` (gate B, généré le 2026-11-09T16:48:37Z). **Mais** `experiments/r0a/work/fixtures/`
était **vide** au démarrage de R0-b et `work/created-paths.json` valait `[]` : la commande `control`
se termine par `cleanup()` (cf. `harness/cli.py`), qui supprime les fixtures inventoriées.

Conséquences et choix :

1. **Aucune commande `run_r0a.py` n'a été lancée ici** (ni `control`, ni `build`, ni `clean`, ni `all`).
2. Les trois fixtures nécessaires (`f4-e2e`, `f7-context`, `f5-crash`) ont été **recréées à partir de
   leurs définitions** dans `experiments/r0a/harness/fixtures.py` (lecture seule), dans
   `experiments/r0b/verification/scratch/`. La provenance est aussi consignée dans
   `logs/environment.json`.
3. **Écart assumé sur `f7-context`** : le script `tools/build.py` du harnais R0-a calcule un
   `context_id` à partir des entrées déclarées et écrit un artefact qui **ne dépend pas** de
   `.cache/tool.bin`. Tel quel, le test CACHE-13 serait **vacuant** (l'artefact ne peut pas changer).
   La copie scratch ajoute donc une lecture explicite de `.cache/tool.bin` (et, pour CONTEXT-03, de la
   variable non déclarée `FLOW_HIDDEN_INPUT`) **tout en gardant `context_id` déclaré-seul**. Cet écart
   est l'objet du test, pas une modification de R0-a ; il révèle au passage une **faiblesse de la
   fixture d'origine** (voir §5, constat 2).
4. Gate B n'a pas été rejoué (interdiction de toucher R0-a) : la validité du harnais est reprise du
   `control.json` du Lead, pas re-mesurée ici.

---

## 4. Résultats par test

### 4.1 `E2E-12` — verdict `REPRODUCED`

**Dispositif.** `f4-e2e` : `api.py`, `consumer.py` (inchangé), `test_e2e.py` (unittest).
`main` = API `area(width, height)` ; branche `breaking` = `area(width)` (signature cassée,
consommateur identique). Le hash du consommateur est identique sur les deux révisions :
`91aded9b620de86c195763a6f58eff35c7af1560f4b8b79df8ad2c5686a82533`.

**(a) Même consommateur, deux issues.**

| Révision | Commande | Code retour | Sortie |
|---|---|---|---|
| `main` (green) | `python -m unittest discover -q` | **0** | `Ran 2 tests ... OK` |
| `breaking` | idem | **1** | `FAILED (errors=2)` — `TypeError: area() takes 1 positional argument but 2 were given` |

**(b) Faux vert avec code retour ignoré.**

| Politique | Code retour gate | Code retour tests | Preuve |
|---|---|---|---|
| `naive_gate.py` (ignore le rc) | **0** | **1** | `{"gate_rc": 0, "policy": "ignore_return_code", "test_rc": 1, ... "FAILED (errors=2)"}` |
| `cmd /c python -m unittest discover -q & exit /b 0` (analogue Windows de `|| true`) | **0** | **1** (les tests ont bien tourné : `FAILED` dans la sortie) | `cmd_and_exit0_rc: 0`, `cmd_and_exit0_ran_tests: true` |
**(c) Gate qui exige le code retour.** `strict_gate.py` renvoie `gate_rc=1` et bloque.

**Interprétation.** Le faux vert est **reproduit** : une politique qui lance les tests mais ignore le
code retour (ou utilise `|| true` / `& exit /b 0`) déclare un succès alors que l'E2E est rouge. La gate
correcte doit exiger le code retour **et** enregistrer la preuve (commande, rc, sortie, arbre exact).
Un sous-ensemble vert ne vaut jamais « tout vert » : ici le consommateur n'a pas changé et casse quand
même.

**Limites.** Une seule fixture Python, unittest seulement. Cela démontre la classe de faux vert, pas
le comportement de toutes les CI ni la merge queue (voir `RECOVERY-16`/`CACHE-13` pour d'autres
propriétés). Aucune exécution E2E réelle (services, réseau) n'est testée.

---

### 4.2 `CACHE-13` — verdict `REPRODUCED`

**Dispositif.** `f7-context` (variante scratch) : le build déclare `inputs/flags.txt`,
`inputs/lock.txt`, `env.FLOW_TOOLCHAIN`, `env.LC_ALL` et calcule
`context_id = SHA256(manifeste trié)`. Il lit **en plus** `.cache/tool.bin` (fichier ignoré par Git,
absent du manifeste) et l'inclut dans l'artefact. `.cache/` et `out/` sont dans `.gitignore`.

| Run | Entrée modifiée | `context_id` | SHA-256 de l'artefact |
|---|---|---|---|
| 1 | `.cache/tool.bin` = `v1` | `729ce6e5c199…` | `6a83140514a6…` |
| 2 | `.cache/tool.bin` = `v2` (entrées déclarées inchangées) | `729ce6e5c199…` **identique** | `d231d9bf8081…` **différent** |
| 3 | `inputs/flags.txt` `-O1` → `-O2` (entrée déclarée) | `7b8a48760d2e…` **différent** | `a2266165924b…` |

État Git : avant le changement de flags, `git status --porcelain` est **vide** (le cache et `out/`
sont ignorés) ; l'arbre Git reste `7acceddfc449…`. Après le changement déclaré, seul
` M inputs/flags.txt` apparaît.

**Interprétation.** Le même `context_id` peut produire **deux artefacts différents** dès qu'une entrée
non déclarée change : c'est la classe d'empoisonnement de cache visée. Une clé qui ne couvre que les
entrées déclarées n'est pas une clé exacte. Règle : **entrée non déclarée ⇒ `UNKNOWN`, pas de cache** ;
ou la clé doit inclure toutes les entrées réellement lues (fichiers, env, version/script de l'outil),
ce qui suppose que l'outil les déclare ou qu'un boundary les observe. Le contrôle (flags déclarés)
montre que le mécanisme détecte bien les entrées déclarées : le défaut est l'**absence de fermeture**
de l'ensemble, pas l'absence de hachage.

**Limites.** Un script Python synthétique ; aucun cache réel (sccache/Bazel/Cargo) n'est instrumenté.
Cela prouve la classe de faille, pas qu'un cache donné la possède. L'écart de fixture (§3.3) est
explicite dans `RESULTS.json` (`scope`) : sans lui, le test serait vacuant.

---

### 4.3 `CONTEXT-03` — verdict `REPRODUCED`

**Dispositif.** `f7-context-completeness` (variante scratch) : le build lit deux entrées **hors
manifeste** — `.cache/tool.bin` (fichier ignoré) et `FLOW_HIDDEN_INPUT` (variable d'environnement).

| Run | État non déclaré | `context_id` | SHA-256 de l'artefact |
|---|---|---|---|
| A | rien | `729ce6e5c199…` | `5bd72e3a4377…` |
| B | `.cache/tool.bin` créé | `729ce6e5c199…` | `54caefcc0edc…` |
| C | `FLOW_HIDDEN_INPUT=hidden-variant` | `729ce6e5c199…` | `e03941ba0cd1…` |

Trois artefacts distincts, un seul `context_id`. Un « vérificateur de complétude » naïf qui compare
les clés du manifeste aux entrées nommées dans le code renvoie `PASS` — et ne voit rien.

**Interprétation.** La complétude de l'ensemble des entrées **ne peut pas être prouvée de l'intérieur**
de l'outil : l'absence de preuve d'une entrée supplémentaire n'est pas une preuve d'absence. La portée
de validation est donc **indéterminée** tant qu'un boundary (sandbox, tracing, hermeticité imposée)
n'énumère pas les lectures effectives, ou que l'outil n'est pas construit pour refuser les lectures non
déclarées. C'est une limite **structurelle**, pas un bug de la fixture.

**Limites.** Démonstration sur un script ; l'argument général est logique (on ne prouve pas une
négative sans frontière d'exécution), pas une mesure sur tous les build systems. Un outil hermétique
par construction (Bazel-like) échappe à cette critique ; ce n'est pas le cas de Cargo/pytest/npm sans
travail supplémentaire.

---

### 4.4 `LEGACY-06` — verdict `REPRODUCED`

**Dispositif.** `legacy06` : dépôt avec `api.py`, `consumer.py` (importe `area`), `test_e2e.py`.
Un commit `semantic-break` renomme `area` → `surface` **dans `api.py` seul**. Patch :

```diff
-def area(width: int, height: int) -> int:
+def surface(width: int, height: int) -> int:
     return width * height
```

| Étape | Commande | Code retour |
|---|---|---|
| Applicabilité textuelle | `git apply --check patch` | **0** |
| Application | `git apply patch` | **0** |
| Plausibilité syntaxique | `compile(open('api.py').read(), 'api.py', 'exec')` | **0** |
| Sémantique | `python -m unittest discover -q` | **1** |

Échec observé : `ImportError: cannot import name 'area' from 'api'` — le consommateur n'a pas bougé.
Une gate « textuelle » (`apply_ok and compile_ok`) vaudrait `promote = True`.

**Interprétation.** `AUTO_SAFE` / applicabilité textuelle ≠ validation. La seule traduction correcte
est `TEXTUALLY_APPLICABLE`, jamais `VALIDATED_FOR_SCOPE` : un patch peut être applicable et
syntaxiquement plausible tout en cassant un consommateur inchangé. Il faut exécuter les obligations
sémantiques sur l'arbre exact.

**Limites.** Un seul type de rupture (renommage). Le test n'énumère pas toutes les ruptures
sémantiques (types, comportement, ABI, schéma) ; il démontre la classe et la mauvaise traduction.

---

### 4.5 `RECOVERY-16` — verdict `REPRODUCED`

**Dispositif.** `f5-crash` : `publisher.py` écrit `intent`, fait `update-ref` CAS, puis `outcome`
dans `.git/flow-journal.jsonl` (JSONL, `fsync` par ligne). Run normal : rc 0, journal de **271 octets**,
2 enregistrements. Puis **troncature en cours de ligne** : fichier coupé à **259 octets**, sans
newline final (écriture partielle simulée).

| Lecteur | Comportement | Code retour | Sortie |
|---|---|---|---|
| naïf strict (`json.loads` par ligne) | **crash** non rattrapé | **1** | `json.decoder.JSONDecodeError: Unterminated string ...` |
| naïf indulgent (`try/except: continue`) | ignore la ligne invalide, conclut « pas d'outcome » | **0** | `{"bad_lines": 1, "last_event": "intent", "would_republish": true}` |
| robuste (refus fail-closed) | refuse le journal corrompu | **2** | `{"status": "REFUSED_CORRUPT_JOURNAL", "reason": "last_line_not_terminated"}` |
| robuste sur journal intact | `COMPLETE` | **0** | `{"status": "COMPLETE", "records": 2, "ref": "…"}` |
| robuste sur journal valide `intent` seul | réconciliation par la ref | **0** | `{"status": "RECONCILED_PUBLISHED_FROM_REF", "records": 1, "ref": "…"}` |

**Interprétation.** Un lecteur naïf a deux modes d'échec : **crash** (au moins visible) ou **reprise
silencieuse** (dangereuse : il conclut à l'absence d'outcome et republierait alors que la ref a déjà
bougé). Un lecteur robuste doit : (i) exiger des lignes complètes et terminées ; (ii) valider le
schéma et la séquence (`intent` puis `outcome`, pas d'événement inconnu) ; (iii) **refuser** sans
décider sur une ligne invalide ; (iv) ne réconcilier qu'à partir d'enregistrements valides **et** de la
ref ; (v) distinguer `PUBLISHED` (outcome) de `RECONCILED_PUBLISHED_FROM_REF` (intent + ref qui
correspond, sans re-publier). Une ligne invalide ne doit jamais être « réparée » silencieusement.

**Limites.** Troncature d'octets seulement : pas de coupure d'alimentation, pas de perte de secteur,
pas de corruption FS, pas de run réel de Flow. Le format JSONL est local à la fixture ; le test établit
l'invariant de lecture requis, pas la durabilité d'un magasin.

---

## 5. Synthèse

### 5.1 Verdicts

| Test | Verdict | Ce qui est établi | Ce qui n'est pas établi |
|---|---|---|---|
| `E2E-12` | `REPRODUCED` | Faux vert par code retour ignoré ; gate rc bloque | Comportement de toute CI, merge queue, E2E réseau |
| `CACHE-13` | `REPRODUCED` | Même `context_id`, artefacts différents sur entrée non déclarée | Qu'un cache réel (sccache/Cargo) a cette faille |
| `CONTEXT-03` | `REPRODUCED` | Complétude non prouvable sans boundary | Théorème général sur tous les outils |
| `LEGACY-06` | `REPRODUCED` | Applicabilité textuelle + syntaxe OK + tests rouges | Toutes les classes de rupture sémantique |
| `RECOVERY-16` | `REPRODUCED` | Naïf : crash ou reprise silencieuse ; robuste : refus | Durabilité réelle, crash/power-loss |

**Aucun test `BLOCKED` ou `NOT_RUN`.** Aucun test hors périmètre n'est présenté comme exécuté
(`RESOURCE-14`, `ENV-04`, `TREE-01`, `f6-canary`, `FENCE-07`, etc. ne sont pas revendiqués ici).

### 5.2 Cinq constats majeurs

1. **Un faux vert E2E est trivial et silencieux.** Le même consommateur passe (`rc=0`) et échoue
   (`rc=1`) ; `naive_gate` et l'analogue `|| true` (`& exit /b 0`) renvoient **0** en ayant lancé les
   tests. Une gate honnête doit exiger le code retour **et** la preuve d'exécution sur l'arbre exact ;
   un sous-ensemble vert n'implique jamais « tout vert ».
2. **La clé de cache déclarée n'est pas exacte.** `context_id` identique, artefact différent : une
   entrée non déclarée suffit à empoisonner le cache. Règle : **entrée non déclarée ⇒ `UNKNOWN`, pas
   de cache**. Corollaire de fixture : le `build.py` R0-a de `f7-context` ne consomme pas
   `.cache/tool.bin` ; **CACHE-13 est vacuant sur la fixture d'origine** et doit être corrigé
   (consommation explicite ou test dédié).
3. **La complétude de la portée n'est pas prouvable de l'intérieur.** Trois exécutions, un
   `context_id`, trois artefacts, aucun signal. Sans boundary d'exécution qui observe les lectures (ou
   hermeticité imposée), la portée de validation est une **hypothèse**, pas un fait ; il faut donc
   élargir ou déclarer `UNKNOWN`.
4. **Applicabilité textuelle ≠ validation.** `git apply --check` = 0, application = 0, syntaxe = 0,
   tests = 1 : `AUTO_SAFE` textuel doit rester `TEXTUALLY_APPLICABLE` ; seule l'exécution des
   obligations sur l'arbre exact peut donner `VALIDATED_FOR_SCOPE`.
5. **Le journal doit être lu en fail-closed.** Ligne partielle ⇒ refus explicite, jamais de reprise
   silencieuse. La réconciliation légitime se fait sur des enregistrements **valides** + la ref
   (`RECONCILED_PUBLISHED_FROM_REF`), pas sur un parsage indulgent qui transforme une corruption en
   « pas d'outcome ».

### 5.3 Recommandations bornées pour le Lead

1. **Conserver** ces cinq cas comme corpus adversarial minimal : ils sont reproductibles, rapides,
   sans réseau ni compilateur.
2. **Corriger la fixture `f7-context`** (ou documenter l'écart) : sans lecture explicite de l'entrée
   ignorée, CACHE-13 ne teste rien. Ajouter aussi une variante d'entrée d'environnement non déclarée
   (déjà utilisée par CONTEXT-03).
3. **Inscrire dans le contrat D2** : (a) gate = rc + preuve + arbre exact ; (b) `UNKNOWN` si entrée
   non déclarée ; (c) lecteur de journal fail-closed + réconciliation par ref ; (d) `AUTO_SAFE` textuel
   non promouvable.
4. **Ne pas conclure** de ces tests qu'un cache ou une CI réels sont sûrs : ils établissent des
   **classes de défaillance** et des invariants de contrat, pas l'état d'une implémentation.

---

## 6. Limites générales

- **Un seul OS** (Windows 11) ; aucun test POSIX, aucune permission/symlink/mode Unix.
- **Pas de réseau, pas de sandbox, pas de compilateur** : les tests sont Python + git.
- **Pas d'implémentation Flow** : les fixtures modélisent des propriétés ; aucun composant Flow n'est
  exercé.
- **Pas de crash/power-loss** : `RECOVERY-16` tronque des octets, il ne coupe pas l'alimentation ni le
  noyau ; la durabilité réelle (fsync, ordre, secteur) n'est pas mesurée.
- **Pas de mesure de performance** ni de RAM : aucun résultat S9 n'est produit ici.
- **Gate B non rejoué** : validé par le `control.json` du Lead ; les fixtures ont été recréées à partir
  des définitions, avec l'écart `f7` documenté.
- **Généralisation interdite** : un verdict `REPRODUCED` sur une fixture ne prouve pas que la classe
  est présente partout ; un `DISPROVED`/`NOT_REPRODUCED` n'aurait pas prouvé l'absence universelle.
- **Périmètre d'écriture** : seuls `experiments/r0b/verification/**` ont été écrits. Des fichiers
  `Cargo.toml`, `Cargo.lock` et `crates/` appartenant à un autre travail en cours sont apparus dans
  l'arbre du dépôt Flow pendant l'exécution : ils n'ont été ni lus en détail, ni modifiés, ni commités
  par cet agent.

---

## 7. Reproduction et fichiers

```bash
python experiments/r0b/verification/run_tests.py
```

- Code retour : `0` si les cinq tests ont produit un verdict ; `1` si un test est `BLOCKED`/`NOT_RUN`.
- Sorties : `RESULTS.json` (liste de 5 enregistrements avec `id, verdict, fixture, os, commands,
  observation, scope, evidence`), `logs/<id>.log` (sorties brutes rédigées), `logs/<id>.json`
  (preuves structurées), `logs/environment.json`, `logs/artifacts/legacy06-textual-break.patch`.
- Fixtures : `scratch/f4-e2e`, `scratch/f7-context`, `scratch/f7-context-completeness`,
  `scratch/f5-crash`, `scratch/legacy06`, `scratch/worktrees/{green,breaking}`.
- Les commits visibles dans `scratch/**` sont ceux des dépôts jetables ; le dépôt Flow n'a reçu aucun
  commit de cet agent.

---

## 8. Références locales

- `experiments/r0a/README.md`, `experiments/r0a/harness/fixtures.py` (définitions des fixtures),
  `experiments/r0a/out/control.json` (gate B `passed: true`).
- `docs/SECURITY_TESTS.md` (`E2E-13`, `CACHE-14`, `TREE-01`), `docs/reviews/RAVUM-FLOW-V6-POST-CLAUDE-REFERENCE.md`
  §4.1–4.2 (statuts non promouvables, `STALE/UNKNOWN`), §9 (matrice de tests).
- `AGENTS.md` (règles de preuve, `FAILED/TIMEOUT/UNKNOWN` non `PASSED`, ressources).
