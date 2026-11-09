# User stories — proposées, non validées

Format : « En tant que [rôle], je veux [capacité], afin de [résultat] ». Une user story n'autorise aucun développement sans GO.

## US-001 — Développeur solo

**En tant que Développeur solo, je veux** initialiser ou examiner un dépôt local sans compte ni service, **afin de** afficher capacités et prérequis hors ligne sans altérer les fichiers.

**Acceptation :** scénario positif + négatif ; le résultat refusé/UNKNOWN est visible ; aucun effet non autorisé sur Ravum. **Trace :** INV-07/08, D0, RF-006.

## US-002 — Agent intégré

**En tant que Agent intégré, je veux** soumettre un patch attribué depuis une base immuable, **afin de** recevoir un statut explicite lorsqu'une précondition a changé.

**Acceptation :** scénario positif + négatif ; le résultat refusé/UNKNOWN est visible ; aucun effet non autorisé sur Ravum. **Trace :** INV-02/05, RF-007/011.

## US-003 — Développeur

**En tant que Développeur, je veux** conserver toutes mes modifications dirty/untracked pendant la publication, **afin de** pouvoir récupérer sans reset destructif après crash.

**Acceptation :** scénario positif + négatif ; le résultat refusé/UNKNOWN est visible ; aucun effet non autorisé sur Ravum. **Trace :** INV-02/03, RF-009/010.

## US-004 — Équipe d'agents

**En tant que Équipe d'agents, je veux** travailler sur tâches distinctes sans worktrees permanents imposés, **afin de** mesurer le gain réel tout en isolant les outils classiques dans une vue temporaire.

**Acceptation :** scénario positif + négatif ; le résultat refusé/UNKNOWN est visible ; aucun effet non autorisé sur Ravum. **Trace :** INV-01-A proposé, S9, RF-016/022.

## US-005 — Mainteneur

**En tant que Mainteneur, je veux** obtenir un reçu de validation lié à l'arbre Git exact, **afin de** empêcher qu'un commit non testé soit publié avec un ancien PASS.

**Acceptation :** scénario positif + négatif ; le résultat refusé/UNKNOWN est visible ; aucun effet non autorisé sur Ravum. **Trace :** INV-04/05, R3/R4.

## US-006 — Responsable CI

**En tant que Responsable CI, je veux** exécuter les tests impactés et leurs dépendants sans ignorer les E2E, **afin de** déclencher un fallback conservateur si le graphe est inconnu.

**Acceptation :** scénario positif + négatif ; le résultat refusé/UNKNOWN est visible ; aucun effet non autorisé sur Ravum. **Trace :** INV-10, RF-012/015.

## US-007 — Auditeur sécurité

**En tant que Auditeur sécurité, je veux** exécuter code candidat non fiable hors secrets et hors workspace privé, **afin de** obtenir refus si sandbox OS ne peut garantir la politique.

**Acceptation :** scénario positif + négatif ; le résultat refusé/UNKNOWN est visible ; aucun effet non autorisé sur Ravum. **Trace :** RF-RFC-010, P0-ENV, TRUST-15.

## US-008 — Développeur Windows/Linux

**En tant que Développeur Windows/Linux, je veux** faire fonctionner les hooks Git compatibles sans écraser mon workspace, **afin de** respecter leur ordre, leurs refus et les nouveaux fichiers stagés.

**Acceptation :** scénario positif + négatif ; le résultat refusé/UNKNOWN est visible ; aucun effet non autorisé sur Ravum. **Trace :** P0-HOOK-ORDER, RF-010.

## US-009 — Mainteneur d'outils

**En tant que Mainteneur d'outils, je veux** ouvrir et relire un commit avec Git normal sans Flow, **afin de** conserver la réversibilité et les métadonnées de provenance.

**Acceptation :** scénario positif + négatif ; le résultat refusé/UNKNOWN est visible ; aucun effet non autorisé sur Ravum. **Trace :** INV-07/11, R4.

## US-010 — Équipe polyglotte

**En tant que Équipe polyglotte, je veux** utiliser les outils natifs Cargo/tsc/pytest/CMake, **afin de** ne pas imposer un compilateur ou serveur Flow universel.

**Acceptation :** scénario positif + négatif ; le résultat refusé/UNKNOWN est visible ; aucun effet non autorisé sur Ravum. **Trace :** INV-06/09, RF-018/020.

## US-011 — Responsable technique

**En tant que Responsable technique, je veux** comparer Flow à l'option ne rien construire, **afin de** arrêter le développement si le coût total n'est pas meilleur.

**Acceptation :** scénario positif + négatif ; le résultat refusé/UNKNOWN est visible ; aucun effet non autorisé sur Ravum. **Trace :** P0-I, D3, RF-003/022.

## US-012 — Mainteneur de Ravum

**En tant que Mainteneur de Ravum, je veux** consommer Flow via un adaptateur stable sans cycles, **afin de** laisser hot swap, preuves du langage et audio RT dans Ravum.

**Acceptation :** scénario positif + négatif ; le résultat refusé/UNKNOWN est visible ; aucun effet non autorisé sur Ravum. **Trace :** Ravum ADR-019/021/024, RF-026.

## US-013 — Auditeur de revue

**En tant que Auditeur de revue, je veux** distinguer les preuves observées et hypothèses d'IA, **afin de** retrouver SHA, sources et statuts sans faux consensus.

**Acceptation :** scénario positif + négatif ; le résultat refusé/UNKNOWN est visible ; aucun effet non autorisé sur Ravum. **Trace :** D0/D1, V6.

## US-014 — Utilisateur d'un outil non intégré

**En tant que Utilisateur d'un outil non intégré, je veux** comprendre que mes écritures directes ne sont pas garanties, **afin de** bénéficier d'un mode isolé sûr ou d'un refus explicite.

**Acceptation :** scénario positif + négatif ; le résultat refusé/UNKNOWN est visible ; aucun effet non autorisé sur Ravum. **Trace :** RF-RFC-004, R2.

## US-015 — Administrateur

**En tant que Administrateur, je veux** voir les décisions encore humaines avant des opérations mutantes, **afin de** exiger GO borné par dépôt, budget, chemins et tests.

**Acceptation :** scénario positif + négatif ; le résultat refusé/UNKNOWN est visible ; aucun effet non autorisé sur Ravum. **Trace :** D4, H1-H6.

## Définition de terminé (future, après GO)

Un ticket technique est DONE seulement après preuves reproductibles et CI/E2E nécessaires, tests négatifs, provenance SHA et budget ; aucun `NOT_TESTED` ou `UNSUPPORTED` ne devient `PASSED`. En phase documentaire, DONE signifie uniquement que la proposition a été rédigée et revue, **pas codée**.
