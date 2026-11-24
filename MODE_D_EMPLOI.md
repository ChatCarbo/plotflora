# Mode d'emploi de PlotFlora

<p align="center"><img src="../assets/logo.png" alt="PlotFlora" width="420"></p>

PlotFlora vous aide à choisir **quelle plante mettre à quel endroit** de votre jardin. Vous partez
d'une photo vue du ciel (ou d'un plan dessiné), vous indiquez à quoi ressemble votre jardin, et le
logiciel vous propose des plantes adaptées, en expliquant pourquoi.

Ce guide ne suppose **aucune connaissance** en informatique ni en jardinage. Comptez une heure pour
votre premier jardin, dont une bonne partie à observer votre terrain.

---

## Sommaire

1. [Installer et lancer PlotFlora](#1-installer-et-lancer-plotflora)
2. [Avant de commencer : observer son jardin](#2-avant-de-commencer--observer-son-jardin)
3. [Obtenir une vue aérienne de son jardin](#3-obtenir-une-vue-aérienne-de-son-jardin)
4. [Créer son projet](#4-créer-son-projet)
5. [Délimiter, orienter et mettre à l'échelle](#5-délimiter-orienter-et-mettre-à-léchelle)
6. [Décrire son jardin](#6-décrire-son-jardin)
7. [Plusieurs sols dans le même jardin](#7-plusieurs-sols-dans-le-même-jardin)
8. [Placer ses zones de plantation](#8-placer-ses-zones-de-plantation)
9. [Obtenir des conseils de plantes](#9-obtenir-des-conseils-de-plantes)
10. [Chercher des idées par ambiance](#10-chercher-des-idées-par-ambiance)
11. [Le catalogue de plantes](#11-le-catalogue-de-plantes)
12. [Enregistrer, ouvrir, annuler](#12-enregistrer-ouvrir-annuler)
13. [Questions fréquentes et problèmes](#13-questions-fréquentes-et-problèmes)
14. [Petit lexique](#14-petit-lexique)

---

## 1. Installer et lancer PlotFlora

1. Sur la page GitHub du projet, ouvrez **Releases** et téléchargez le fichier
   `PlotFlora-…-windows-x64.zip`.
2. Faites un clic droit sur le fichier téléchargé › **Extraire tout…**, puis choisissez un dossier
   (par exemple `Documents\PlotFlora`).
3. Ouvrez ce dossier et double-cliquez sur **`plotflora.exe`**.

> **Windows affiche « Windows a protégé votre ordinateur » ?** C'est normal pour un logiciel récent
> qui n'est pas signé par un éditeur payant. Cliquez sur **Informations complémentaires**, puis
> **Exécuter quand même**.

Il n'y a rien à installer d'autre. PlotFlora fonctionne **sans Internet** ; une connexion sert
seulement à télécharger les photos des plantes et à chercher des plantes absentes du catalogue.

<p align="center"><img src="captures/accueil.png" alt="Écran d'accueil" width="600"></p>

L'écran se compose de quatre parties :

| Partie | Rôle |
| :--- | :--- |
| **En haut** : les menus | Fichier, Édition, Options, Aide |
| **À gauche** : les outils | ce que fait votre souris sur la carte (sélectionner, dessiner…) |
| **Au centre** : la carte | votre jardin |
| **À droite** : les onglets | Jardin, Zone, Catalogue, Idées |
| **En bas** : la barre d'état | messages du logiciel (en rouge en cas de problème) |

---

## 2. Avant de commencer : observer son jardin

Les conseils de PlotFlora ne valent que ce que valent les informations que vous lui donnez. Rien
n'est obligatoire — tout peut rester « Inconnu » — mais chaque information connue rend les conseils
plus justes. Voici comment les obtenir **sans matériel spécialisé**.

### Le froid de l'hiver
Notez la **température la plus basse** atteinte chez vous lors d'un hiver normal (pas
exceptionnel). Repères : environ −5 °C sur la côte atlantique ou méditerranéenne, −10 à −15 °C dans
une grande partie de la France, −20 °C et moins en montagne ou dans l'Est. Les voisins jardiniers et
les relevés de la station météo la plus proche sont de bonnes sources.

### Le soleil
Pour chaque endroit où vous voulez planter, regardez **s'il est au soleil** à 9 h, 12 h, 15 h et
18 h, un jour d'été. Chaque observation au soleil compte pour environ 3 heures :

| Observations au soleil | Exposition à choisir |
| :--- | :--- |
| 3 ou 4 | Plein soleil |
| 2 | Mi-ombre |
| 1, ou soleil filtré à travers un arbre | Ombre légère |
| 0 | Ombre dense |

### Le type de sol : le test du boudin
Prenez une poignée de terre humide (à 10–15 cm de profondeur) et roulez-la entre vos mains.

| Ce qui se passe | Type de sol |
| :--- | :--- |
| Elle ne tient pas, s'effrite, crisse sous les doigts | **Sableux** |
| Elle forme un boudin qui casse quand on le courbe | **Limoneux** (le sol « idéal ») |
| Elle forme un boudin souple qu'on peut plier en anneau, colle aux doigts | **Argileux** |
| Elle est claire, avec des cailloux blancs, et mousse au vinaigre | **Calcaire** |
| Elle est très sombre, légère, sent le sous-bois | **Humifère** |

### Le drainage : le test du trou
Creusez un trou de 30 cm de profondeur, remplissez-le d'eau, laissez-le se vider, puis remplissez-le
une seconde fois et notez le temps qu'il met à se vider :

| Temps | Drainage |
| :--- | :--- |
| moins de 10 minutes | Rapide |
| 10 minutes à 1 heure | Bon |
| 1 à 4 heures | Moyen |
| plus de 4 heures, ou flaques qui restent après la pluie | Mauvais |

### Le pH (acide ou calcaire)
Un **kit de test du pH** (quelques euros en jardinerie) donne la réponse en 5 minutes. Sans kit,
quelques indices : des hortensias bleus, des bruyères et des rhododendrons en pleine forme chez vous
ou chez vos voisins indiquent un sol **acide** ; une terre qui mousse au vinaigre est **alcaline**
(calcaire). Dans le doute, choisissez « Neutre » ou laissez « Inconnu ».

### L'humidité, le vent et l'arrosage
* **Humidité** d'un endroit : sec (terre souvent sèche en été), normal, frais (reste un peu humide),
  humide (souvent détrempé).
* **Vent** : abrité (protégé par un mur, une haie), modéré, exposé (en plein vent, bord de mer…).
* **Arrosage** : pourrez-vous arroser l'été ? Faible (rarement), moyen, élevé (arrosage automatique,
  récupérateur d'eau…).

---

## 3. Obtenir une vue aérienne de son jardin

La meilleure source est **Géoportail**, le site de l'IGN, gratuit et utilisable librement :

1. Allez sur **geoportail.gouv.fr** et tapez votre adresse dans la barre de recherche.
2. Dans les cartes proposées, choisissez **Photographies aériennes**.
3. Zoomez avec la molette jusqu'à ce que votre jardin occupe la plus grande partie de l'écran, en
   gardant un peu de marge autour. **Ne tournez pas la carte** : le Nord doit rester en haut.
4. **Mesurez une longueur** : utilisez l'outil de mesure de Géoportail (menu des outils, « Mesurer
   une distance ») sur un élément net et rectiligne, par exemple un mur de votre maison ou une
   clôture. **Notez cette longueur** (par exemple 9,4 m) : elle servira à l'étape 5.
5. Faites une capture d'écran : appuyez sur **Windows + Maj + S**, puis sélectionnez la zone de la
   carte avec la souris. La capture est copiée ; une notification apparaît en bas à droite :
   cliquez dessus, puis sur l'icône de disquette pour **l'enregistrer** (format PNG), par exemple
   dans `Documents\Jardin`.

> Une capture Google Maps (vue satellite) fonctionne aussi, pour un usage strictement personnel.

Vous n'avez pas de vue aérienne exploitable (jardin caché par les arbres, maison neuve) ? Vous
pouvez **dessiner** votre terrain à la place : voir l'étape 4, « plan dessiné ».

---

## 4. Créer son projet

### Avec une vue aérienne
1. Menu **Fichier › Nouveau projet (image aérienne)…**
2. Choisissez votre capture d'écran.
3. Choisissez où enregistrer le projet et donnez-lui un nom (par exemple `Mon jardin.json`).
   L'image est copiée à côté du projet : gardez-les ensemble si vous déplacez le dossier.

### Avec un plan dessiné
1. Menu **Fichier › Nouveau projet (plan dessiné)…** puis choisissez où l'enregistrer.
2. L'outil **Contour du terrain** est actif : cliquez sur la carte pour placer les coins de votre
   terrain (la grille est en mètres), puis **double-cliquez** pour terminer.
3. Ajoutez si vous le souhaitez la maison, l'abri, la terrasse ou les arbres existants avec l'outil
   **Obstacle** (choisissez le type sous l'outil). Pour un arbre existant, un seul clic suffit.

Le plan dessiné est déjà à l'échelle : passez directement à l'étape 6.

---

## 5. Délimiter, orienter et mettre à l'échelle

### Délimiter le jardin (détection automatique)
À la création du projet, l'outil **🔍 Détecter le jardin** est déjà actif.

1. **Cliquez au milieu de votre jardin**, sur la pelouse par exemple.
2. PlotFlora dessine un contour blanc autour de la zone de couleur semblable, puis **agrandit la vue
   sur votre jardin**.
3. Le contour n'est pas parfait ? Trois solutions :
   * il **déborde** chez le voisin : baissez le curseur **Tolérance** sous l'outil et cliquez de
     nouveau ;
   * il est **trop petit** : montez la tolérance et cliquez de nouveau ;
   * il est presque bon : prenez l'outil **Sélection** et **faites glisser les petits carrés
     blancs** (les coins du contour) à la bonne place.
4. Vous préférez le tracer vous-même ? Outil **Contour du terrain** : cliquez chaque coin, puis
   double-cliquez.

### Mettre à l'échelle (indispensable pour les quantités)
1. Outil **↔ Calibrer l'échelle**.
2. Cliquez sur **les deux extrémités** de l'élément que vous avez mesuré sur Géoportail (le mur, la
   clôture).
3. Tapez sa **longueur réelle** en mètres dans la fenêtre qui s'ouvre, puis **Valider**.

Une barre d'échelle apparaît en bas à gauche de la carte. Vérifiez avec l'outil **📏 Mesurer** :
cliquez deux points, la distance s'affiche en bas de l'écran.

### Orienter le Nord
Avec une vue Géoportail non tournée, il n'y a **rien à faire** : le Nord est en haut. Sinon, outil
**Orienter le Nord**, puis cliquez dans la direction du Nord ; ou réglez l'angle dans l'onglet
Jardin. La rose des vents en haut à droite de la carte se met à jour.

---

## 6. Décrire son jardin

Ouvrez l'onglet **🏡 Jardin** à droite et remplissez la partie **Contexte du jardin** avec vos
observations de l'étape 2 :

* Température hivernale minimale (cochez la case pour saisir une valeur) ;
* Risque de sécheresse estivale ;
* Type de sol, Drainage, pH : les valeurs **générales** de votre jardin ;
* Disponibilité d'arrosage ;
* **Ambiance recherchée** : le style que vous aimez (voir le tableau ci-dessous). Facultatif.

| Ambiance | En quelques mots |
| :--- | :--- |
| Méditerranéen | lavandes, oliviers, cistes : soleil, sécheresse, parfums |
| Japonais / zen | érables du Japon, bambous non traçants, mousses, sobriété |
| Champêtre / cottage | fleurs abondantes, roses, vivaces en mélange |
| Contemporain / graphique | graminées, formes nettes, feuillages structurés |
| Sous-bois / ombre | fougères, hostas, hellébores : pour les coins ombragés |
| Nourricier / potager | fruits, légumes, aromatiques |
| Sauvage / biodiversité | plantes locales et mellifères, pour les oiseaux et les insectes |
| Exotique | palmiers, bananiers, grands feuillages |

Pensez aussi au champ **Attribution** : si votre image vient de Géoportail, cliquez sur le petit
bouton **IGN** pour afficher la mention « © IGN – BD ORTHO » demandée par l'IGN.

---

## 7. Plusieurs sols dans le même jardin

Votre jardin a un coin argileux qui reste humide, un massif de terre de bruyère, une bande caillouteuse
au pied d'un mur ? Créez des **secteurs de sol** :

1. Dans l'onglet **Jardin**, rubrique **Secteurs de sol**, cliquez sur **+ Tracer** (ou outil
   **Secteur de sol** à gauche).
2. Sur la carte, cliquez les coins de la partie concernée, puis **double-cliquez**.
3. Dans l'onglet Jardin, donnez un nom au secteur (par exemple « Coin argileux ») et choisissez son
   **type de sol**, son **drainage** et son **pH**. « Comme le jardin » garde la valeur générale.

Le secteur apparaît en couleur sur la carte. Toute zone de plantation placée **dans** ce secteur
utilisera ce sol ; l'onglet Zone vous indique toujours quel sol est pris en compte. Si deux secteurs
se chevauchent, c'est le **dernier tracé** qui compte. Pour en supprimer un : bouton 🗑 à côté de son
nom.

---

## 8. Placer ses zones de plantation

Une **zone** est un endroit où vous voulez planter. Il en existe trois sortes :

| Outil | Pour… | Comment la tracer |
| :--- | :--- | :--- |
| 📍 **Plante isolée** | un arbre, un arbuste, une plante seule | un clic |
| ⬟ **Massif** | un massif de fleurs, un couvre-sol, un potager | cliquez les coins, double-cliquez pour finir |
| 〰 **Haie / bordure** | une haie, une bordure le long d'une allée | cliquez les points, double-cliquez pour finir |

Pendant un tracé : **Retour arrière** retire le dernier point, **Échap** abandonne.

Une fois la zone créée, l'onglet **📍 Zone** s'ouvre. Renseignez :

* **Libellé** : un nom qui vous parle (« Haie du fond », « Massif devant la terrasse ») ;
* **Statut** : À planter, Existant (une plante déjà là, juste pour mémoire) ou À remplacer ;
* **Exposition**, **Vent**, **Humidité du sol** de cet endroit précis ;
* **Hauteur max** (et le **Rayon disponible** pour une plante isolée, la **Largeur max** pour une
  haie) : cochez la case et indiquez la place réelle. Ex. : « pas plus de 1,80 m pour ne pas cacher
  la vue » ;
* **Feuillage souhaité** (persistant = garde ses feuilles l'hiver, caduc = les perd), **Entretien
  maximum** ;
* **Ambiance** : laissez « Celle du jardin » ou choisissez-en une autre pour cette zone ;
* **Usages souhaités** : ornemental, fruits comestibles, mellifère (bon pour les abeilles), haie…

Pour **modifier une zone** : outil **Sélection**, cliquez dessus. Faites-la glisser pour la déplacer,
ou faites glisser ses petits carrés jaunes pour changer sa forme. Touche **Suppr** ou bouton
« Supprimer la zone » pour l'effacer.

La légende à gauche rappelle les couleurs : vert = à planter, gris = existant, orange = à remplacer.

---

## 9. Obtenir des conseils de plantes

<p align="center"><img src="captures/editeur.png" alt="Éditeur" width="800"></p>

1. Sélectionnez une zone (outil Sélection, clic sur la zone).
2. Dans l'onglet **Zone**, cliquez sur **▶ Lancer l'algorithme**.
3. La liste des plantes conseillées apparaît, **de la plus adaptée à la moins adaptée**. Cliquez sur
   une ligne pour voir le détail.

### Comprendre le résultat
Chaque plante reçoit une **note sur 100** et un **verdict** :

| Verdict | Signification |
| :--- | :--- |
| 🟢 **Très adapté** | 80 points ou plus, sans réserve |
| 🟡 **Adapté avec réserves** | bonne note, mais lisez les alertes (taille à prévoir, arrosage…) |
| 🟠 **Peu adapté** | possible, mais de nombreux points faibles |
| 🔴 **Déconseillé** | ne survivrait pas ou ne tiendrait pas dans l'espace |

La note se décompose en 7 critères : exposition (25 points), rusticité au froid (15), sécheresse et
arrosage (10), sol et humidité (20), dimensions (15), usages, feuillage et ambiance (10), entretien
et vent (5). Dans le détail :

* ✔ en vert : les **points forts** ;
* ⚠ en orange : les **alertes** à prendre en compte (protection hivernale, taille régulière…) ;
* ✖ en rouge : un **problème rédhibitoire** (gel, ombre trop forte, plante trop grande, sol
  calcaire pour une plante de terre acide…) ;
* ℹ en gris : des **informations** (donnée inconnue, fiche non vérifiée).

> Les informations « Inconnu » ne bloquent rien, mais elles valent moins de points qu'une
> information favorable : plus vous renseignez votre jardin, plus le classement est fiable.

### Choisir une plante
Cliquez sur **Choisir cette plante**. Son nom apparaît sur la carte et dans la zone, avec la
**quantité à planter** pour un massif ou une haie (si l'échelle est calibrée). Si vous êtes connecté
à Internet, sa **photo est téléchargée** automatiquement (avec le nom de son auteur et sa licence).

En haut de la liste, des filtres permettent de n'afficher que les fiches vérifiées, de masquer les
plantes déconseillées ou de choisir une catégorie (arbre, arbuste…).

---

## 10. Chercher des idées par ambiance

L'onglet **💡 Idées** répond à la question « qu'est-ce que je pourrais planter ? » sans avoir à
tracer de zone :

1. Choisissez une **ambiance** (ou « Toutes ») ;
2. Choisissez le **sol** : celui du jardin, ou l'un de vos secteurs de sol ;
3. Si vous le souhaitez, précisez l'**exposition**, la **catégorie** (arbre, arbuste, vivace…) et une
   **hauteur maximale**.

La liste se met à jour immédiatement : seules les plantes **compatibles avec ce sol et cette
ambiance** sont proposées, de la mieux notée à la moins bien notée. **Fiche** ouvre la fiche de la
plante ; **Assigner à la zone sélectionnée** la place dans la zone choisie sur la carte.

---

## 11. Le catalogue de plantes

L'onglet **🌿 Catalogue** contient environ **630 plantes et arbres** de jardin.

* **Chercher** : tapez un nom, commun ou latin (les fautes de frappe légères sont tolérées).
* **Ouvrir une fiche** : cliquez sur une ligne. ✔ = fiche vérifiée, ⚠ = fiche à vérifier.
* **Corriger une fiche** : modifiez les champs puis **💾 Enregistrer**. Les champs marqués ⚠ ont été
  remplis automatiquement : vérifiez-les en priorité (survolez-les pour voir leur source).
* **Valider une fiche** : quand vous l'avez relue, **✔ Valider la fiche**.
* **Ajouter une plante absente** : tapez son nom, puis **🌐 Rechercher en ligne (Wikidata)**, et
  **Importer** le bon résultat. Une fiche brouillon est créée : vérifiez-la, puis enregistrez-la. Ou
  **Nouvelle fiche** pour tout saisir à la main.
* **Photos** : **Choisir une photo…** pour mettre votre propre photo ; **🌐 Télécharger la photo
  libre** pour récupérer la photo de Wikimedia Commons.

Vos modifications sont enregistrées dans votre catalogue personnel (dans
`%APPDATA%\PlotFlora`) : elles ne sont jamais perdues lors d'une mise à jour du logiciel.

> **Photos et Internet** : le téléchargement automatique des photos se désactive dans le menu
> **Options**. Aucune autre information n'est envoyée sur Internet.

---

## 12. Enregistrer, ouvrir, annuler

| Action | Comment |
| :--- | :--- |
| Enregistrer | **Ctrl + S** ou Fichier › Enregistrer (un ● dans le titre de la fenêtre signale des modifications non enregistrées) |
| Enregistrer ailleurs | Fichier › Enregistrer sous… |
| Ouvrir un projet | Fichier › Ouvrir… et choisissez le fichier `.json` |
| Annuler / rétablir | **Ctrl + Z** / **Ctrl + Y** (jusqu'à 100 étapes) |
| Recentrer la carte | bouton 🔍 sous les outils |
| Zoomer | molette de la souris |
| Déplacer la carte | clic sur la molette et glisser, ou glisser sur un endroit vide avec l'outil Sélection |

À chaque enregistrement, la version précédente est gardée dans un fichier `.bak` à côté du projet.

---

## 13. Questions fréquentes et problèmes

**La détection du jardin prend aussi le jardin du voisin.**
Baissez la tolérance (par exemple à 25) et cliquez à nouveau, ou tracez le contour à la main.

**La détection dit que la zone est trop petite.**
Cliquez sur une grande surface uniforme (la pelouse plutôt qu'un massif) ou montez la tolérance.

**Aucune quantité n'apparaît.**
L'échelle n'est pas calibrée (étape 5), ou la fiche de la plante n'indique ni espacement ni largeur
adulte.

**Toutes les plantes sont « Déconseillé ».**
Ouvrez le détail d'une plante : la ligne ✖ explique pourquoi. Souvent, la **hauteur max** ou le
**rayon disponible** est trop petit, ou l'**exposition** est « Ombre dense ». Décochez « Masquer
Déconseillé » pour voir toute la liste.

**Une plante n'est pas dans le catalogue.**
Onglet Catalogue › tapez son nom › 🌐 Rechercher en ligne › Importer.

**L'image a disparu à l'ouverture du projet.**
Le projet et son image ont été séparés. Onglet Jardin › **Relier / changer l'image…** et
sélectionnez l'image.

**La photo d'une plante ne se télécharge pas.**
Vérifiez la connexion Internet et l'option dans le menu **Options**. Certaines plantes n'ont pas de
photo libre disponible : vous pouvez ajouter la vôtre (Choisir une photo…).

**Puis-je partager mon projet ?**
Oui : envoyez le fichier `.json` **et** l'image qui est à côté. Les fiches des plantes choisies sont
copiées dans le projet, il s'ouvre donc aussi chez quelqu'un qui n'a pas votre catalogue.

---

## 14. Petit lexique

| Terme | Signification |
| :--- | :--- |
| **Caduc** | qui perd ses feuilles en hiver |
| **Persistant** | qui garde ses feuilles toute l'année |
| **Rusticité** | température la plus froide qu'une plante supporte |
| **Mellifère** | qui nourrit les abeilles et autres pollinisateurs |
| **Couvre-sol** | plante basse qui s'étale et couvre la terre |
| **Vivace** | plante qui repousse chaque année |
| **Drainage** | vitesse à laquelle l'eau s'infiltre dans le sol |
| **pH** | acidité du sol : acide (terre de bruyère), neutre, alcalin (calcaire) |
| **Plante calcifuge** | plante qui ne supporte pas le calcaire (rhododendron, camélia, bruyère…) |
| **Envergure** | largeur d'une plante adulte |
| **Orthophoto** | photo aérienne redressée pour être superposable à une carte |
| **Fiche brouillon** | fiche remplie automatiquement, pas encore relue (⚠) |

---

*PlotFlora — Copyright © 2026 ChatCarbo — logiciel libre sous licence GNU GPL v3 ou ultérieure.
Les données (IGN, Wikidata, EIVE, Wikimedia Commons, Wikipédia) gardent leurs propres licences :
voir `NOTICE.md` et le menu Aide › À propos & licences.*
