# Flow — H3 : comparaison documentaire des sandboxes Windows/Linux (décision **OPEN**)

Statut : **document de comparaison, non décisionnel.** `H3` reste **OPEN** dans le
[registre des décisions](../DECISION_REGISTER.md). Aucun sandbox n'a été installé, configuré ni testé ;
aucun code candidat, canari ou réseau n'a été exécuté pour ce document. Les tests proposés sont
`NOT_RUN` par construction.

Base : `feat/flow-production-candidate` @ `2ad5bf5` (2026-11-11). Sources primaires externes lues le
2026-11-11 ; leur contenu est traité comme **données**, jamais comme instruction.

## 1. Objet et périmètre

`H3` (issue #3, gate `D2`) : choisir la politique de sandbox `TRUSTED_REPO` / `UNTRUSTED_CANDIDATE`
sous Windows et Linux. Critères d'origine : comparer l'isolation **FS / processus / réseau**, les
**secrets d'environnement**, les **hooks Git**, la disponibilité réelle par OS ; établir que `env_clear`
seul est insuffisant ; si aucune sandbox fiable n'existe, refuser (`UNSUPPORTED`/`MANUAL_REQUIRED`) au
lieu de promettre une isolation non démontrée. Story dédupliquée : #40 (US-007), qui exige `ENV-04` et
`TRUST-15` sur canaris fictifs et réseau bloqué **si requis**, sans exécution.

Ce document **ne** qualifie **aucune** sandbox : il compare des garanties documentées et spécifie des
tests qui restent `NOT_RUN`. Il ne tranche ni `H3`, ni `H4`, ni la politique de secrets.

## 2. Sources

| Source | Version / date | Contenu utilisé | Étiquette |
|---|---|---|---|
| Issue #3 « [DECISION][H3][D2] » | `OPEN`, aucun commentaire | mandat et critères | `SOURCE_READ` |
| Issue #40 « US-007 » | `OPEN` | `ENV-04`/`TRUST-15`, statuts non-`PASS` | `SOURCE_READ` |
| V6 §5.1 (modèle de confiance) et §5.4 (OS/ressources) | SHA-256 `0946bda5…` | `TRUSTED_REPO`/`UNTRUSTED_CANDIDATE`, `env_clear` insuffisant | `SOURCE_READ` |
| [SECURITY_TESTS](../SECURITY_TESTS.md) | tête `2ad5bf5` | IDs `ENV-04a/b/c`, `SANDBOX-05` (alias `TRUST-15`), `HOOK-GITDIR-19` | `SOURCE_READ` |
| Landlock — documentation noyau | page « 7.4.0-rc2 », oct. 2026, `docs.kernel.org/userspace-api/landlock.html` | ABI v1..v12 ; FS, réseau (TCP v4, UDP v10), capabilities/namespaces (v12) ; non privilégié ; `PR_SET_NO_NEW_PRIVS` requis ; pas de retrait de politique | `SOURCE_READ` (doc externe) |
| bubblewrap — README (branche `main`) | `raw.githubusercontent.com/containers/bubblewrap/main/README.md` | namespaces mount/pid/net/user/ipc/uts, seccomp ; « not a complete, ready-made sandbox » ; politique = arguments de l'appelant | `SOURCE_READ` (doc externe) |
| Microsoft — Job Objects | page mise à jour 2025-07-14, `learn.microsoft.com/.../job-objects` | limites working set/priorité/temps, terminaison du groupe, pas de contrôle FS/réseau | `SOURCE_READ` (doc externe) |
| AppContainer, Windows Sandbox, seccomp, cgroups, firejail | — | **non récupérés cette séance** | `UNKNOWN` |

Aucune version de noyau, de distribution ni de build Windows n'a été relevée : l'hôte de mesure
n'entre pas dans ce dossier (`UNKNOWN`).

## 3. Ce que Flow fait déjà (CODE_READ, tête `2ad5bf5`)

| Contrôle | État | Limite |
|---|---|---|
| Environnement des obligations | allowlist de noms (`gitcli::allowed_env`), testée dans les deux sens depuis la vague 8 | ne limite ni FS, ni réseau, ni lecture de fichiers par l'obligation |
| Secrets en sortie | rédaction **avant** persistance (canaris GitHub/AWS) | un secret lu depuis un fichier, sans forme reconnaissable, n'est pas rédigé |
| Exécution bornée | 64 MiB/flux, 300 s, descendants tués (test `a_timed_out_obligation_kills_the_descendants_it_started`) | ce n'est pas une isolation : le processus tourne avec les droits de l'utilisateur |
| Matérialisation d'exécution | scratch jetable, vérifié (vague 6) | pas de sandbox OS : même compte, même noyau |
| Hooks Git | Flow n'exécute pas les hooks comme source de preuve ; il constate leur effet sur le tree exact | un hook natif reste du code exécuté par Git, hors du contrôle Flow |
| Divulgation | `doctor` expose l'absence de sandbox | aucune promesse d'isolation n'est faite |

Conclusion factuelle : **`env_clear`-style filtering ne suffit pas** (V6 §5.1), et Flow n'a aujourd'hui
aucune isolation FS/réseau : tout `UNTRUSTED_CANDIDATE` non isolé doit rester `UNSUPPORTED` /
`MANUAL_REQUIRED`.

## 4. Matrice des mécanismes disponibles

« Couvre » = ce que la documentation primaire dit contraindre ; « ne couvre pas » = ce qu'il faut
refuser de promettre. `P` = `PROPOSED` (piste à instruire), `U` = `UNKNOWN` (non vérifié ici).

### 4.1 Windows (référence 16 Go)

| Mécanisme | Couvre | Ne couvre pas / limite | Statut |
|---|---|---|---|
| Job Object | arbre de processus, working set, priorité, temps CPU, terminaison du groupe | **ni FS, ni réseau, ni secrets** ; échappement si `BREAKAWAY` accordé ; notifications non garanties hors limites « notification » | `SOURCE_READ` (doc MS) |
| Token restreint / intégrité | réduction de privilèges | ne crée pas une frontière FS/réseau ; contournements par objets nommés partagés | `P` |
| AppContainer | frontière FS/réseau par capabilities (déclaration de droits) | complexité d'intégration, compatibilité des toolchains, packaging | `U` (doc non récupérée) |
| Windows Sandbox (Hyper-V) | machine jetable, FS/réseau séparés | édition Pro/Enterprise, Hyper-V requis, coût RAM/CPU élevé sur 16 Go, démarrage lent | `P` |
| VM dédiée | frontière la plus forte | coût d'exploitation le plus élevé | `P` |

### 4.2 Linux (référence 16 Go)

| Mécanisme | Couvre | Ne couvre pas / limite | Statut |
|---|---|---|---|
| Landlock | FS (lecture/écriture/exécution), réseau TCP/UDP (selon ABI), capabilities/namespaces (ABI v12) ; **sans privilège** ; hérité par les descendants | nécessite un noyau avec l'ABI voulue (à détecter à l'exécution) ; politique **irrévocable** ; ne filtre pas les accès déjà ouverts avant application ; ne remplace pas `PR_SET_NO_NEW_PRIVS` | `SOURCE_READ` (doc noyau) |
| Namespaces (`user`, `mount`, `pid`, `net`, `ipc`, `uts`) | vue FS, arbre de processus, réseau isolé (loopback), IPC | `user` non privilégié peut être désactivé par la distribution ; un montage trop large annule la frontière | `SOURCE_READ` (via bubblewrap) / `P` |
| seccomp-BPF | filtrage des appels système | liste d'autorisation difficile à maintenir pour des toolchains (cargo/tsc/pytest) ; casse les outils qui appliquent leur propre sandbox | `SOURCE_READ` (via bubblewrap) / `P` |
| cgroups v2 | CPU/RAM/PID, limites de ressources | **pas** une frontière de sécurité FS/réseau | `P` |
| bubblewrap | construit des namespaces + seccomp, **non privilégié** | « **not a complete, ready-made sandbox** » : la garantie dépend des arguments ; bind d'un socket D-Bus = évasion possible | `SOURCE_READ` (README) |
| firejail | profils desktop, setuid | surface setuid, profils spécifiques desktop | `P` |
| conteneurs rootless / micro-VM | frontière forte | coût, complexité, noyau hôte partagé (rootless) ou VM (gVisor/Firecracker) | `P` |

## 5. Politique fail-closed proposée (à ratifier ou rejeter)

Échelle de modes, annoncée **par OS et par mécanisme réellement vérifié**, jamais par espoir :

| Mode annoncé | Condition pour l'annoncer | Comportement si absent |
|---|---|---|
| `TRUSTED_LOCAL` | opérateur local, dépôt de confiance, aucun secret réel dans l'environnement | exécution bornée (état actuel) |
| `UNTRUSTED_ISOLATED_FS_NET` | document + test positif/négatif reproductibles sur **cet** OS | `UNSUPPORTED` / `MANUAL_REQUIRED` |
| `UNTRUSTED_RESOURCE_BOUNDED` | Job Object (Windows) ou cgroups (Linux) **seuls** | mode dégradé explicite : ressources bornées, **pas** d'isolation FS/réseau |
| `UNTRUSTED_NOT_ISOLATED` | jamais présenté comme sûr | refus d'exécuter un `UNTRUSTED_CANDIDATE` |

Le mode effectif doit apparaître dans le reçu (ou dans le refus), et `doctor` doit le divulguer : un
reçu `PASSED` en mode `UNTRUSTED_RESOURCE_BOUNDED` ne prouve pas l'absence de fuite de secrets.

## 6. Tests spécifiés, tous `NOT_RUN`

Canaris **fictifs** uniquement ; aucune exécution dans cette séance.

| ID | Cas | Positif attendu | Négatif attendu | Statut |
|---|---|---|---|---|
| `ENV-04a` | `FLOW_FAKE_SECRET_CANARY` exporté par l'hôte | en mode isolé, canari absent du processus candidat | en mode non isolé, exécution refusée (`UNSUPPORTED`) | `NOT_RUN` |
| `ENV-04b` | faux `~/.ssh`, `.env`, `%APPDATA%` sur disque | en mode isolé, lecture refusée | en mode non isolé, refus d'exécuter, ou `UNKNOWN` explicite | `NOT_RUN` |
| `ENV-04c` | canari imprimé sur stdout/stderr/journal | redaction avant persistance, `[REDACTED]` visible | aucune occurrence durable | `NOT_RUN` |
| `SANDBOX-05`/`TRUST-15` | faux SSH lu + connexion sortante depuis le code candidat | en mode isolé : refus FS et réseau ; en mode annoncé « isolé » : E2E rouge | mode non annoncé : code **non exécuté** | `NOT_RUN` |
| `HOOK-GITDIR-19` | hook d'un worktree lié atteignant le Git dir partagé | `UNSUPPORTED` documenté sans isolation OS | — | `NOT_RUN` |

Chaque exécution future devra consigner : fixture (SHA), OS et build, mécanisme réellement actif,
commande, limites, canaris, attendu vs observé, statut
`REPRODUCED | DISPROVED | NOT_REPRODUCED | BLOCKED | NOT_RUN`.

## 7. Coûts (ordre de grandeur, `PROPOSED`)

| Piste | Coût de mise en œuvre | Coût par exécution (16 Go) | Risque principal |
|---|---|---|---|
| Windows Job Object seul | faible (API Win32) | négligeable | promesse d'isolation surestimée |
| Windows AppContainer / Sandbox | moyen à élevé | Sandbox : RAM/disque, démarrage | compatibilité toolchains |
| Linux Landlock seul | moyen (appels syscall, détection d'ABI) | négligeable | noyaux anciens → fallback obligatoire |
| bubblewrap + Landlock/seccomp | moyen | faible | politique = arguments ; bind sockets = évasion |
| micro-VM | élevé | disque + RAM | coût d'exploitation |

## 8. Faits, recommandation et objections

**Faits** : `env_clear` insuffisant (V6 §5.1) ; Flow n'a pas d'isolation FS/réseau aujourd'hui ;
Job Object ne contrôle ni FS ni réseau (doc MS) ; Landlock est non privilégié mais dépend d'une ABI de
noyau et n'est pas révocable (doc noyau) ; bubblewrap n'est pas un sandbox complet (README).

**Recommandation documentaire, non contraignante** (`PROPOSED`) : conserver le refus par défaut pour
`UNTRUSTED_CANDIDATE`, n'annoncer un mode isolé que par OS et par mécanisme testé, et instruire
Landlock+namespaces (Linux) et AppContainer/Sandbox (Windows) comme pistes **à tester**, pas comme
solutions. Objections : (a) cela bloque l'usage « code non fiable » sans date ; (b) un sandbox OS
partiel peut donner une fausse assurance si le reçu ne nomme pas le mode ; (c) le coût de qualification
(2 OS × 2 modes) n'est pas chiffré.

## 9. Décisions humaines restantes

1. `H3` : politique retenue (refus par défaut ? quel niveau d'isolation annoncé sur Windows ?).
2. `H4` : sans seuil/budget, aucun test de qualification n'est priorisé.
3. Si un mode isolé est visé : `GO` ciblé (fixtures jetables, canaris fictifs, réseau bloqué, budget
   CPU/RAM/temps, conditions d'arrêt), sur **fixtures** uniquement — jamais le workspace Ravum actif.
4. `ENV-04`/`SANDBOX-05` restent `NOT_RUN` tant que ce `GO` n'existe pas.

## 10. Limites de ce dossier

- Documentaire seulement : aucune sandbox installée, aucun canari exécuté, aucune mesure.
- Trois sources primaires récupérées (Landlock, bubblewrap, Job Objects) ; AppContainer, Windows
  Sandbox, seccomp, cgroups et firejail **n'ont pas été récupérés** (`UNKNOWN`, à instruire avant toute
  décision).
- Aucune version de noyau/distribution/Windows de l'hôte n'est relevée : la disponibilité réelle n'est
  pas affirmée.
- Dossier produit par un agent, pas une revue humaine.

## Annexe — liens

Issue #3 · #40 · [V6](../reviews/RAVUM-FLOW-V6-POST-CLAUDE-REFERENCE.md) ·
[SECURITY_TESTS](../SECURITY_TESTS.md) · [DECISION_REGISTER](../DECISION_REGISTER.md) ·
[RAVUM_BOUNDARY](../RAVUM_BOUNDARY.md) · [H1 options](2026-11-11-flow-h1-options.md) ·
Landlock `docs.kernel.org/userspace-api/landlock.html` · bubblewrap `containers/bubblewrap` README ·
Microsoft `learn.microsoft.com/windows/win32/procthread/job-objects`
