# DEEPSEEK-06 — DÉCISIONS À FIGER (H1–H6, + H7–H8 proposés)

**Statut :** `HUMAN-DECISION-REQUIRED`. Aucune décision ci-dessous n'est prise, ni présumée. Les recommandations sont **non contraignantes**.
**Règle :** une décision se consigne avec **acteur, date, preuve, périmètre**. Une absence de réponse ne vaut pas acceptation.
**Base :** l'ensemble du dossier (`00`–`05`, `07`), les rapports A–H, et les six questions de la V6 §10.1.

---

## 0. Vue d'ensemble

| ID | Objet | État actuel | Bloque | Recommandation (non contraignante) |
|---|---|---|---|---|
| **H1** | option architecturale provisoire O0/O1/O2/O3 | OPEN | R1, tout code | **O0 instrumenté par défaut ; O2-mince en hypothèse non financée ; pas de O3 avant H3** |
| **H2** | licence, droits, extraction vs clean-room | OPEN / **BLOCKER** | toute copie de code, distribution | **aucune extraction ; grant écrit ou from-scratch ; licence Flow à choisir** |
| **H3** | politique sandbox Windows/Linux | OPEN | O3, hooks isolés, toute promesse de sûreté | **fail-closed par OS ; Windows = `UNSUPPORTED`/`MANUAL_REQUIRED`** |
| **H4** | seuil de gain et budget S9 | OPEN | D3, R0-c | **δ exprimé en % apparié + heures/an, plancher absolu ; figé avant R0-c** |
| **H5** | `GO_R0_FIXTURES` | NOT GIVEN | R0 | **oui, borné : R0-a/b/c uniquement, fixtures jetables** |
| **H6** | handoff P0 à la session Ravum | OPEN | rien côté Flow | **oui, signalement seulement ; aucun patch émis par Flow** |
| **H7** *(proposé)* | Flow = outil interne Ravum ou produit public ? | **ABSENT du registre** | périmètre, licence, support, stories | **interne d'abord ; produit seulement après second utilisateur** |
| **H8** *(proposé)* | règle d'arrêt de la phase documentaire | **ABSENTE** | risque de 8e revue | **geler après ce dossier + 4 corrections ; aucune revue sans fait nouveau** |

---

## H1 — Option architecturale provisoire

**Question :** quelle option retenir **provisoirement** pour orienter la suite, sans la ratifier ?

**Options :** O0 · O1-A (Git + worktree éphémère + hooks natifs + CI impactée + script CAS) · O1-B/C (jj, GitButler) · O2-mince (reçu + politique + writer contrôlé) · O3 (O2 + exécution isolée) · O4 (différé).

**Preuves disponibles :**

- O0/O1-A couvrent la tranche Git/hooks/CAS sans nouveau moteur ([A](raw/DEEPSEEK-RAW-A-GIT-CONCURRENCY.md) §3.5 ; [F](raw/DEEPSEEK-RAW-F-PRIOR-ART.md) §5).
- Le noyau revendiqué par Flow (publication transactionnelle, crash tests, CI d'impact) **existe déjà dans Ravum** (PR #85/#87, `mvp.yml`, `mvp-impact.yml`) → triad dupliquer/dépendre/réécrire ([H](raw/DEEPSEEK-RAW-H-ADVERSARIAL.md) §S1, `DEEPSEEK-01` §5).
- S8 ne démontre **aucun** classement et ne mesure **aucun** worktree (`DEEPSEEK-04` §2).
- O3 dépend d'une sandbox OS réelle, non qualifiée sur Windows ([B](raw/DEEPSEEK-RAW-B-SECURITY.md) §2.2).

**Recommandation :** **O0 instrumenté par défaut** (Git + worktree éphémère + hooks natifs + CI existante + un script de reçu), **O2-mince conservé comme hypothèse non financée**. Ne pas retenir O3 avant H3 et un test AppContainer. Ne pas retenir O4.

**Ce qui changerait la recommandation :** R0-c montrant un écart de `T_verified` ≥ δ avec zéro condition éliminatoire **et** un test de demande montrant ≥ 3 agents concurrents fréquents.

**Propriétaire :** le responsable. **Échéance logique :** avant tout R1.

---

## H2 — Licence, droits, extraction

**Question :** (a) quelle licence pour Flow ? (b) autorise-t-on une réutilisation de code Ravum, et sous quelles conditions ?

**Faits :**

- Ravum : **aucun** `LICENSE` (404 sur `LICENSE`, `LICENSE.md`, `COPYING`), `license: null`, et le dépôt **le dit lui-même** (`README.md:216-218`, `CONTRIBUTING.md:3`) → aucun grant implicite.
- Flow : **aucun** `LICENSE` non plus → le projet n'est pas open source aujourd'hui.
- `RAVUM-ADR-041` exige des licences évaluées avant ajout → sa précondition est absente.
- Authorship (100 derniers commits) : 57 `Rwanbt`, 39 `Ravum Agent`, 4 `Erwan Barat` ; aucun CLA/DCO visible → **fait**, pas conclusion juridique.
- Clean-room **affaibli** : les agents Flow ont déjà lu `git_writer.rs`, `ravum-store`, `patch-host`.

**Recommandation :** (a) choisir une licence Flow **explicitement** (MIT OR Apache-2.0 est une option raisonnable, non ratifiée) ou assumer « interne, non distribué » ; (b) **aucune extraction de code Ravum** tant qu'un grant écrit ne nomme pas chemins et droits ; (c) `from scratch` par défaut ; (d) documenter SPDX/provenance des deux côtés.

**Ce qui changerait la recommandation :** un grant écrit du titulaire couvrant les chemins visés.

**Propriétaire :** le responsable (titulaire des droits). **Bloque :** H7, R6, toute distribution.

---

## H3 — Politique de sandbox

**Question :** quelle garantie d'isolation promet-on, sur quel OS, pour quel mode ?

**Faits :** sur Windows 11 **famille**, aucune primitive non-VM, non-admin ne donne une frontière FS+réseau ; Job Objects sont des quotas, pas une sandbox ; Windows Sandbox est Pro/Ent/Edu, mono-instance ; AppContainer est le seul candidat non-VM mais sa compatibilité avec `rustc/cargo/node` **n'est pas prouvée** ; sur Linux, userns + bubblewrap donnent une frontière crédible, conditionnée à la distribution.

**Recommandation :** **fail-closed par OS** :
- Linux : sandbox obligatoire pour `UNTRUSTED_CANDIDATE` (bwrap ; Landlock en profondeur si ABI ≥ 4) ; sinon `UNSUPPORTED`.
- Windows : `UNTRUSTED_CANDIDATE` → **`MANUAL_REQUIRED` par défaut** ; AppContainer seulement après test de compatibilité ; Windows Sandbox = option assistée documentée.
- Aucune promesse de parité Windows. Les hooks restent du code arbitraire, même « approuvés ».

**Ce qui changerait la recommandation :** un test reproductible (≈ 5 jours) montrant AppContainer + Job Objects fonctionnels pour `cargo`, `node`, `python` sur la machine de référence.

**Propriétaire :** le responsable. **Bloque :** O3, RF-004/FLOW-ADR-004, US-007/US-008.

---

## H4 — Seuil de gain et budget S9

**Question :** quel effet est « suffisant » pour justifier la construction, et avec quel budget ?

**Faits :** les seuils 15/20/25 % circulent sans ratification (V6 `:714`, `:900`, `:1762`) ; un seuil en % de wall-clock ne paie pas la maintenance ; S8 ne peut pas servir de référence (n = 3, plages chevauchantes, rapport de médianes).

**Recommandation :**
1. Critère primaire : **réduction du ratio médian apparié de `T_verified`** vs le meilleur baseline, IC 95 % bas > 0.
2. Choisir **un** seuil : `H4-20` (≥ 20 %) est le compromis le plus défendable ; `H4-15` si le bruit observé est faible ; `H4-25` si l'on veut un plancher absolu.
3. Ajouter un **plancher absolu** en secondes par campagne et une **traduction en heures humaines/an** ; si la valeur annuelle estimée < coût de maintenance, ne pas construire.
4. Budget : **plafonner** la campagne (ex. ≤ 5 jours d'outillage, ≤ 30 h humaines, ≤ 20 h machine pour l'exploratoire), une seule campagne à la fois sur 16 Go.
5. `δ_eq` (marge d'équivalence O0) doit être fixé **avant** les résultats, sinon la règle O0 est infalsifiable.

**Propriétaire :** le responsable. **Bloque :** D3, R0-c. **Ne peut pas être délégué à un modèle.**

---

## H5 — `GO_R0_FIXTURES` borné

**Question :** autorise-t-on des expériences sur fixtures jetables ?

**Recommandation : oui, borné**, avec un périmètre explicite :

| Autorisé | Interdit |
|---|---|
| Fixtures synthétiques jetables, secrets **fictifs** | Le dépôt Ravum actif, le clone local, le workspace utilisateur |
| Reproduction des P0 (TREE/HOOK/ENV/FENCE/FILE/PATH) | Correctifs dans Ravum |
| S9-baseline A/B/C/D/E (+F si dispo), n = 8 | Écrire du code Flow (R1+) |
| Harnais de mesure neutre, une campagne à la fois | CI Flow, publication, licence, PR |
| Budget plafonné (H4) | Compilations lourdes concurrentes de la session Ravum |

**Sortie attendue :** `REPRODUCED / DISPROVED / NOT_REPRODUCED` par test, `σ_d`, `T_coord/T_total`, coût worktree calibré, décision de poursuite.
**Coût indicatif** (`ESTIMATE`) : 3–5 j d'outillage, 15–30 h humaines, 7–20 h machine.

**Propriétaire :** le responsable. **Condition :** H4 provisoire fixé avant de lancer.

---

## H6 — Handoff P0 à la session Ravum

**Question :** transmet-on les constats P0 à l'autre session ?

**Recommandation : oui, comme signalement**, jamais comme ordre :
- transmettre SHA, blobs, lignes, et tests à exécuter (`HOOK-02a/b/c`, `TREE-01`, `ENV-04a/b/c`, `FILE-09`) ;
- préciser que **rien n'est reproduit** et que la priorisation appartient à la session Ravum ;
- **ne pas** émettre de patch, de branche ou de PR depuis Flow ;
- si la session Ravum corrige, re-pinner le SHA et invalider les constats devenus obsolètes.

**Note importante :** la session Ravum a déjà fusionné #85/#87 (publication transactionnelle, overlays) et ouvre #101. Le handoff doit donc commencer par **re-vérifier** que les chemins P0 existent encore au SHA courant — ils existaient au `5408d9b3`, et `dev` n'a pas bougé à l'heure de cette revue.

**Propriétaire :** le responsable. **Aucun impact bloquant côté Flow.**

---

## H7 — *(proposé)* Flow : outil interne Ravum ou produit public ?

**Pourquoi cette décision manque :** elle conditionne le périmètre (7 verbes vs 26 RF), la licence (H2), le support, la documentation, et la pertinence même des 15 stories. Le corpus la présuppose (« solo d'abord, équipes ensuite ») sans la poser.

**Faits :** un seul utilisateur identifié (l'auteur) ; aucun second utilisateur, aucune issue externe, aucun fork ; coût de maintenance 300–500 h/an pour O2-mince contre une valeur plausible de 100–200 h/an ([G](raw/DEEPSEEK-RAW-G-PRODUCT-UX.md) §9.5).

**Recommandation :** **interne d'abord** — Flow sert d'abord l'auteur sur Ravum et un dépôt polyglotte, avec `disable` prouvé ; le passage en produit exige un **second utilisateur réel** et une licence publiée.

**Ce qui changerait la recommandation :** un utilisateur externe, un fork actif, ou une demande documentée.

---

## H8 — *(proposé)* Règle d'arrêt de la phase documentaire

**Pourquoi :** la V6 dit « on ne lance pas une sixième contre-expertise par défaut » (l. 457) et une septième a été lancée le même jour. Le corpus récompense la production de documents et jamais l'arrêt ([H](raw/DEEPSEEK-RAW-H-ADVERSARIAL.md) §S5).

**Recommandation :** clore la phase documentaire après ce dossier et la correction des quatre défauts de forme (`X-05`, `X-06`, `X-07`, `X-11`) ; **aucune nouvelle revue** sans fait empirique nouveau (R0-b/R0-c) ou décision humaine non consignée. Une revue supplémentaire n'est justifiée que par une **preuve**, pas par une insatisfaction.

---

## 9. Ce qu'il faut retenir pour la réunion de décision

1. **Deux décisions débloquent tout : H2 et H4.** Sans elles, R0 ne peut pas être borné et aucune extraction n'est légale.
2. **H1 n'a pas besoin d'être tranchée maintenant** : la recommandation par défaut (O0 instrumenté) est réversible et n'engage aucune dépense.
3. **H3 conditionne l'existence de O3** : sans sandbox Windows, le mode phare n'existe pas sur l'OS de référence.
4. **H5 est peu coûteuse et très informative** : c'est le meilleur rapport décision/coût du dossier.
5. **H6 ne coûte rien** et évite un conflit de propriété.
6. **H7/H8 ne sont pas dans la V6** mais sont les deux vraies questions de gouvernance : pour qui, et jusqu'à quand.

---

*Sources : `DECISION_REGISTER.md`, V6 §10.1, rapports A/B/C/D/E/F/G/H, `DEEPSEEK-00..05`. Aucune décision prise, aucune signature simulée.*
