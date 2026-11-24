# Cahier des spécifications — PlotFlora (version 1.2)

**Projet :** PlotFlora (`plotflora`) — Copyright © 2026 ChatCarbo — GPL-3.0-or-later
**Description :** logiciel local de planification de jardin. À partir d'une orthophoto ou d'un plan dessiné, l'utilisateur délimite son jardin, décrit son sol et son climat, trace des zones de plantation et obtient des recommandations de plantes explicables, adaptées au sol et à l'ambiance recherchée.
**Date :** 22 novembre 2026
**Statut :** décrit le logiciel tel qu'implémenté en version 1.2. Remplace les versions 1.0 et 1.1.
**Mode d'emploi utilisateur :** `docs/MODE_D_EMPLOI.md`

---

## 0. Évolutions

| Version | Apports |
| :--- | :--- |
| 1.1 | Orthophoto IGN ou plan dessiné ; échelle ; zones point / massif / haie avec quantités ; données ouvertes (Wikidata, EIVE, Commons, Wikipédia) ; scoring en 7 critères avec règles bloquantes ; catalogue de démarrage. |
| **1.2** | Détection automatique du contour du jardin et cadrage de la vue ; **secteurs de sol** (plusieurs sols dans un jardin) ; **ambiances** de jardin prises en compte dans le score ; onglet **Idées** ; catalogue d'environ **630 espèces** ; **photos téléchargées seulement à l'ajout au jardin**. |

---

## 1. Objectifs et périmètre

### 1.1 Objectif
Aider un jardinier amateur à choisir **quelle plante mettre à quel endroit**, en justifiant chaque conseil :
1. obtenir un fond de plan (photo aérienne ou dessin) ;
2. délimiter le jardin, l'orienter (Nord) et le mettre à l'échelle ;
3. décrire le jardin : climat, sol général, secteurs de sol particuliers, ambiance souhaitée ;
4. tracer des zones de plantation (plante isolée, massif, haie) et les décrire (lumière, vent, humidité, contraintes) ;
5. obtenir pour chaque zone un classement explicable des plantes et les quantités à planter ;
6. explorer des idées de plantes par ambiance et par sol ;
7. tout sauvegarder localement.

### 1.2 Périmètre de la version 1.2
* Projets : création, ouverture, enregistrement atomique, « Enregistrer sous ».
* Fond de plan : image (IGN conseillé) ou plan dessiné en mètres.
* Contour du jardin : détection automatique (image) ou tracé manuel ; sommets déplaçables.
* Orientation, calibration d'échelle, mesure de distance.
* Obstacles (mode dessin) ; secteurs de sol (tous modes).
* Zones `point`, `area`, `line` : création, déplacement, édition des sommets, suppression.
* Catalogue embarqué (~630 espèces) + catalogue utilisateur ; recherche floue ; recherche en ligne sur action explicite ; éditeur de fiche avec validation.
* Scoring sur 100 points, règles bloquantes, ambiance, quantités ; onglet Idées.
* Photos : photo libre téléchargée à l'ajout au jardin, ou photo personnelle.
* Annuler / Rétablir, confirmation de fermeture, écran « À propos & licences ».

### 1.3 Hors périmètre (évolutions envisagées)
* Récupération automatique de l'orthophoto IGN depuis une adresse (service Géoplateforme).
* Simulation de l'ombre portée des obstacles.
* Export d'un plan annoté et d'une liste d'achats (PDF / CSV).
* Import / export CSV du catalogue.
* Ajout / suppression de sommets sur une zone existante ; liste des projets récents.
* Mémorisation des réglages entre deux sessions (ex. option « photos automatiques »).

### 1.4 Choix techniques
* **Rust** (≥ 1.82), édition 2021. Interface **egui / eframe 0.29**. Windows 10/11 en priorité.
* Workspace de quatre crates :

| Crate | Rôle | Dépendances |
| :--- | :--- | :--- |
| `plotflora-core` | Vocabulaire, modèle, géométrie, détection du contour, recherche, conversions, scoring. **Aucune entrée/sortie.** | aucune obligatoire (`serde` en option) |
| `plotflora-data` | Fichiers JSON, catalogue, cache, index EIVE, Wikidata / Wikipédia / Commons | serde, serde_json, ureq, tempfile, dirs, thiserror |
| `plotflora-app` | Interface graphique, binaire `plotflora.exe` | eframe, rfd, image, winresource (icône Windows) |
| `plotflora-catalog-builder` | Outil développeur : génère les données embarquées | calamine |

---

## 2. Fond de plan

### 2.1 Mode `image`
* Formats : `.png`, `.jpg`, `.jpeg`, `.webp`. L'image est **copiée dans le dossier du projet** à la création (nom dédoublonné si besoin).
* Source conseillée : **Géoportail / IGN BD ORTHO** (Licence Ouverte Etalab 2.0, mention « © IGN – BD ORTHO » ; bouton « IGN » dans l'onglet Jardin). Une capture Google Maps est tolérée pour un usage strictement personnel ; le logiciel ne télécharge jamais d'image Google.
* Image introuvable à l'ouverture : message sur la carte et bouton « Relier / changer l'image… ».

### 2.2 Mode `drawing`
* Contour du terrain en polygone, unité = mètre, grille adaptative (pas de 0,5 à 200 m).
* Obstacles : `house`, `shed`, `terrace`, `wall`, `other` (polygones) et `existing_tree` (cercle de 2 m par défaut, rayon et hauteur modifiables).

### 2.3 Contour du jardin et cadrage
* **Détection automatique** (mode image, outil « Détecter le jardin », actif par défaut à la création) : l'utilisateur clique au milieu du jardin ; le contour est calculé (§2.4), enregistré dans `map.boundary`, et la vue est recadrée dessus.
* **Tracé manuel** : outil « Contour du terrain » (clics, double-clic ou Entrée pour terminer).
* **Ajustement** : avec l'outil Sélection, les sommets du contour (carrés blancs) se déplacent à la souris. « Effacer » dans l'onglet Jardin supprime le contour.
* **Cadrage** : quand un contour existe, « Recentrer la vue » et l'ouverture cadrent sur son emprise + 4 % de marge, en occupant 92 % de la zone d'affichage. Sinon : toute l'image, ou le contenu du dessin.

### 2.4 Algorithme de détection (« baguette magique »)
1. Image réduite à 640 px de côté maximum (facteurs d'échelle X et Y conservés).
2. Couleur de référence : moyenne d'un carré de 7 × 7 px autour du clic.
3. Croissance de région 4-connexe : pixels dont la distance de couleur (euclidienne RVB) à la référence est ≤ **tolérance** (défaut 40, réglable de 10 à 120).
4. Fermeture morphologique (dilatation puis érosion, rayon `max(2, côté/200)`), puis bouchage des trous (allées, massifs de couleur différente).
5. Contrôles : surface < 0,2 % de l'image → « trop petite » ; > 85 % → « déborde » (message invitant à ajuster la tolérance).
6. Contour extérieur le long des bords de pixels ; en cas de pincement, la plus grande boucle est retenue.
7. Simplification de Douglas-Peucker (seuil 0,4 % de la diagonale, augmenté jusqu'à ≤ 40 sommets).
8. Le polygone doit être simple (sans côtés croisés), sinon « contour inexploitable » → tracé manuel.

Le résultat est **approximatif** par nature (haie voisine de même couleur, ombres) : il est présenté comme une proposition à ajuster.

### 2.5 Orientation et échelle
* `north_angle_deg` ∈ [0, 360), sens horaire depuis le haut du plan ; 0 = Nord en haut (orthophotos IGN). Réglage par valeur numérique ou outil « Orienter le Nord » (clic dans la direction du Nord depuis le centre de la vue).
* Échelle (mode image) : deux clics sur une longueur connue + distance réelle en mètres → `meters_per_unit`. Mode dessin : 1 unité = 1 m.
* Sans échelle : pas de surface, de longueur ni de quantité ; seules les contraintes saisies à la main jouent dans le critère Dimensions.

### 2.6 Repère
Coordonnées en unités du plan : pixels de l'image d'origine (mode image) ou mètres (mode dessin) ; origine en haut à gauche, Y vers le bas.

---

## 3. Contexte du jardin, secteurs de sol et ambiance

### 3.1 Contexte général (onglet Jardin)
Température hivernale minimale (°C), risque de sécheresse estivale, type de sol, drainage, pH, disponibilité d'arrosage, **ambiance recherchée**. Chaque valeur peut rester « Inconnu ».

### 3.2 Secteurs de sol
* Un **secteur de sol** est un polygone tracé avec l'outil « Secteur de sol », avec un libellé et trois valeurs facultatives : type de sol, drainage, pH. « Comme le jardin » (`null`) = la valeur générale s'applique.
* **Sol effectif d'une zone** : on prend le **point d'ancrage** de la zone (§4.3) ; s'il est dans un ou plusieurs secteurs, le **dernier tracé** s'applique ; ses valeurs renseignées remplacent celles du jardin, les autres restent celles du jardin.
* L'onglet Zone affiche le sol effectif et son origine ; l'algorithme et l'onglet Idées l'utilisent.
* Affichage : remplissage coloré selon le type de sol, étiquette « Secteur : type ».

### 3.3 Ambiances
| Clé | Libellé |
| :--- | :--- |
| `mediterranean` | Méditerranéen |
| `japanese` | Japonais / zen |
| `cottage` | Champêtre / cottage |
| `contemporary` | Contemporain / graphique |
| `woodland` | Sous-bois / ombre |
| `edible` | Nourricier / potager |
| `wildlife` | Sauvage / biodiversité |
| `exotic` | Exotique |

* Ambiance du jardin (onglet Jardin) ; une zone peut avoir **sa propre ambiance** (« Celle du jardin » par défaut).
* Chaque fiche plante liste ses ambiances (`characteristics.ambiances`), renseignées pour tout le catalogue embarqué.

---

## 4. Zones de plantation

### 4.1 Types
| `kind` | Usage | Géométrie | Mesure |
| :--- | :--- | :--- | :--- |
| `point` | Plante isolée | 1 point | — |
| `area` | Massif, couvre-sol | Polygone simple ≥ 3 sommets | Surface |
| `line` | Haie, bordure | Polyligne ≥ 2 points | Longueur |

### 4.2 Paramètres
Libellé ; statut (`to_plant`, `existing`, `to_replace`) ; exposition, vent, humidité du sol (facultatifs) ; hauteur max (tous), rayon disponible (point), largeur max (haie) ; usages souhaités ; feuillage souhaité ; entretien maximum ; **ambiance** ; plante assignée ; quantité forcée (massif, haie) ; photo personnelle ; notes.

| Exposition | Définition |
| :--- | :--- |
| Plein soleil | ≥ 6 h de soleil direct en été |
| Mi-ombre | 3 à 6 h |
| Ombre légère | < 3 h, lumière filtrée |
| Ombre dense | pas de soleil direct |

### 4.3 Point d'ancrage
Point de référence d'une zone (étiquette, secteur de sol) : le point lui-même ; le milieu de la longueur d'une haie ; pour un massif, son centre de gravité **s'il est à l'intérieur**, sinon le milieu du plus large segment intérieur sur l'horizontale passant par ce centre (garanti à l'intérieur, y compris pour un massif en L).

### 4.4 Quantités
Espacement `e` = espacement de plantation de la fiche, à défaut l'envergure adulte. Point : 1. Haie : `⌊longueur / e⌋ + 1`. Massif : `⌈surface / e²⌉`. Quantité forcée prioritaire. Rien n'est affiché sans échelle ou sans `e`.

### 4.5 Statut `existing`
Zone de repère : pas de recommandation.

---

## 5. Données plantes

### 5.1 Sources
| Source | Licence | Usage |
| :--- | :--- | :--- |
| Wikidata | CC0 | Nom latin, noms français, lien Wikipédia, **nom du fichier photo Commons** |
| EIVE (Tichý et al. 2023, onglet `Tab-IVs-Tichy-et-al2023`) | CC BY 4.0 | Lumière, humidité, pH — index embarqué, hors ligne |
| Wikimedia Commons | licence de chaque fichier | Photos, téléchargées à la demande avec crédits |
| Wikipédia FR | CC BY-SA 4.0 | Extraction de valeurs (rusticité, hauteur, envergure, toxicité) ; aucun texte stocké |

Sources exclues : sites marchands (photos et textes protégés), bases sous licence non commerciale, images Google.

### 5.2 Catalogue embarqué
* `assets/seed_species.txt` : **628 espèces** de jardin courantes en France (arbres, arbustes, grimpantes, vivaces, bulbes, graminées, fougères, couvre-sols, aromatiques, potager, annuelles), format `Nom latin | ambiance, ambiance`. Les espèces envahissantes en France sont exclues.
* `plotflora-catalog-builder` (outil développeur) produit `assets/eive_index.json` et `assets/plants_catalog.seed.json`, intégrés au binaire :
  * lecture d'EIVE : recherche de l'onglet et des colonnes (`--sheet`, `--taxon-col`, `--l-col`, `--m-col`, `--r-col`), en-têtes sur une ou deux lignes, affichage des plages de valeurs pour contrôle ;
  * pour chaque espèce : recherche Wikidata (noms comparés après normalisation, « × » des hybrides ignoré, second essai sans « × ») ; nouvelle fiche brouillon, ou **complément** d'une fiche existante (ambiances, liens, référence photo) **sans rien écraser** ;
  * progression affichée, enregistrement tous les 25 noms, liste finale des taxons introuvables et des échecs réseau ; relance sans refaire le travail fait.
* Fiches embarquées : `verified = false` sauf saisie manuelle relue.

### 5.3 Recherche locale
Normalisation (minuscules, accents, ponctuation, mots parasites « plante », « pied de »…), recherche floue (exact > début > contient > distance de Levenshtein ≤ longueur / 3) sur nom commun, latin et cultivar.

### 5.4 Recherche en ligne (bouton explicite)
Wikidata (candidats), détails, texte Wikipédia (extraction de valeurs), index EIVE, photo Commons ; le résultat est un brouillon à vérifier. Si la fiche existe déjà, seuls les champs vides sont complétés.

### 5.5 Photos à la demande
* Le catalogue ne contient que `links.commons_image` (nom du fichier).
* Quand une plante sans photo est **assignée à une zone** (onglet Zone, Idées ou fiche), sa photo est mise en file d'attente et téléchargée en arrière-plan (vignette 800 px, auteur, licence, URL) dans `%APPDATA%\PlotFlora\photos\`, puis enregistrée dans le catalogue utilisateur et dans la copie du projet.
* Option « Télécharger la photo des plantes ajoutées au jardin » (menu Options, activée par défaut). Bouton « Télécharger la photo libre » dans la fiche pour une demande ponctuelle.
* Photo personnelle possible pour une fiche (copiée dans le dossier photos) ou pour une zone (chemin relatif au projet).

### 5.6 Réseau
Aucune requête sans action de l'utilisateur. En-tête `User-Agent` avec contact (`PLOTFLORA_CONTACT`, lu à l'exécution puis à la compilation) ; 1 requête / seconde au plus ; délai 10 s ; sur 429 / 503, jusqu'à 4 réessais en respectant `Retry-After` (sinon 5, 10, 15, 20 s). Cache `lookup_cache.json` de 180 jours.

### 5.7 Provenance, validation, protection
`meta.field_sources` (source par champ), `meta.uncertain_fields` (champs à vérifier, marqués ⚠ dans l'éditeur), `meta.verified`. Valider une fiche exige noms et catégorie, met `verified = true` et vide les champs incertains. Une fiche existante n'est jamais écrasée par un import : seuls les champs vides sont complétés. L'identifiant d'une fiche ne change jamais.

---

## 6. Modèle de données

### 6.1 Fichiers
| Fichier | Emplacement |
| :--- | :--- |
| `projet.json` + image | dossier choisi par l'utilisateur |
| `plants_catalog.json` (catalogue utilisateur), `photos\`, `lookup_cache.json` | `%APPDATA%\PlotFlora\` |
| catalogue embarqué, index EIVE, icône, logo | intégrés au binaire |

### 6.2 Conventions
* `null` = inconnu (ou « comme le jardin » pour un secteur, « celle du jardin » pour l'ambiance d'une zone). Pas de valeur `unknown`.
* Énumérations en `snake_case` ; types « préférence » (avec `any`) distincts des types « plante ».
* Chemins relatifs au fichier qui les contient, séparateur `/`.
* Identifiants : plante = `snake_case` du nom latin (+ cultivar), suffixe `_2`… en cas de collision, immuable ; zone = `z_N` jamais réutilisé ; secteur = `s_N`.
* `schema_version` **1.2.0** ; toute version 1.x est lue (champs absents = valeurs par défaut, champs inconnus ignorés) ; version 2.x refusée.

### 6.3 Projet (extrait)
```json
{
  "schema_version": "1.2.0",
  "name": "Mon jardin",
  "map": {
    "source": "image",
    "image_path": "vue_aerienne.png",
    "attribution": "© IGN – BD ORTHO",
    "north_angle_deg": 0.0,
    "scale": { "p1": [412.0, 380.5], "p2": [903.0, 391.0], "distance_m": 9.4 },
    "boundary": [[120.0, 80.0], [1480.0, 92.0], [1460.0, 990.0], [130.0, 975.0]],
    "obstacles": []
  },
  "environment": {
    "min_winter_temp_c": -12, "summer_drought_risk": "medium",
    "soil_type": "loamy", "soil_drainage": "good", "soil_ph": "neutral",
    "watering_availability": "medium", "ambiance": "cottage"
  },
  "soil_sectors": [
    { "id": "s_1", "label": "Coin argileux", "points": [[900.0, 600.0], [1400.0, 610.0], [1390.0, 960.0], [910.0, 950.0]],
      "soil_type": "clay", "soil_drainage": "poor", "soil_ph": null }
  ],
  "next_zone_seq": 2,
  "zones": [
    { "id": "z_1", "kind": "point", "label": "Massif Sud-Est", "points": [[1240.0, 562.0]],
      "status": "to_plant", "sun_exposure": "full_sun", "wind_exposure": "sheltered", "moisture": "normal",
      "max_height_m": 1.8, "available_radius_m": 1.2, "max_width_m": null,
      "desired_uses": ["ornamental", "pollinator"], "foliage_preference": "any", "max_maintenance": "low",
      "ambiance": null, "assigned_plant_id": "lavandula_angustifolia", "quantity_override": null,
      "custom_photo_path": null, "notes": "" }
  ],
  "plants_snapshot": { "lavandula_angustifolia": { "…": "copie de la fiche assignée" } }
}
```

### 6.4 Fiche plante (champs ajoutés en 1.2)
* `characteristics.ambiances` : liste d'ambiances.
* `links.commons_image` : nom du fichier photo sur Wikimedia Commons.

Les autres champs sont inchangés depuis la 1.1 : identité, `photo` (fichier, URL, auteur, licence), `climate`, `exposure`, `soil`, `dimensions` (dont `planting_spacing_m`), `characteristics`, `care` (`water_need`, `pruning_months`…), `notes`, `tags`, `links`, `meta`.

### 6.5 Vocabulaire contrôlé
| Champ | Valeurs |
| :--- | :--- |
| `status` | `to_plant`, `existing`, `to_replace` |
| `kind` | `point`, `area`, `line` |
| exposition | `full_sun`, `partial_shade`, `light_shade`, `dense_shade` |
| `wind_exposure` | `sheltered`, `moderate`, `exposed` |
| `moisture` | `dry`, `normal`, `moist`, `wet` |
| sol | `loamy`, `clay`, `sandy`, `chalky`, `humus` |
| drainage | `fast`, `good`, `moderate`, `poor` |
| pH | `acid`, `neutral`, `alkaline` |
| niveaux (sécheresse, vent, arrosage) | `low`, `medium`, `high` |
| `water_need`, `maintenance_level` | `very_low`, `low`, `medium`, `high` |
| `max_maintenance` | `any` + niveaux ci-dessus |
| `foliage` / `foliage_preference` | `evergreen`, `semi_evergreen`, `deciduous` / `any`, `evergreen`, `deciduous` |
| usages | `ornamental`, `edible_fruit`, `edible_leaf`, `aromatic`, `pollinator`, `hedge`, `groundcover`, `shade_provider` |
| `category` | `tree`, `shrub`, `perennial`, `climber`, `groundcover`, `vegetable_herb` |
| ambiance | voir §3.3 |
| obstacle | `house`, `shed`, `terrace`, `existing_tree`, `wall`, `other` |

---

## 7. Algorithme de recommandation

### 7.1 Principe
Pour une zone : contexte effectif = jardin + secteur de sol (§3.2) ; ambiance = celle de la zone, sinon celle du jardin. Les règles bloquantes sont évaluées, puis 7 critères notés, puis le verdict.

### 7.2 Arithmétique
Facteurs en millièmes (entiers 0–1000) ; points d'un critère = `(poids × facteur + 500) / 1000` ; score = somme des points (le détail affiché égale toujours le total). Donnée inconnue (zone, jardin ou plante) : facteur **600** et une information « donnée inconnue ».

### 7.3 Critères
| # | Critère | Poids | Règle |
| :---: | :--- | :---: | :--- |
| 1 | Exposition | 25 | idéale 1000 ; tolérée 600 (alerte) ; sinon 0 et **bloquant** |
| 2 | Rusticité | 15 | marge = T° jardin − T° plante ; ≥ 0 → 1000 ; −3 à 0 → 400 (alerte « protection hivernale ») ; < −3 → 0 et **bloquant** |
| 3 | Sécheresse & arrosage | 10 | `(600·d + 400·w)/1000` ; `d` = table tolérance × risque (+250 / +500 si arrosage moyen / élevé, plafonné) ; `w` = besoin en eau vs disponibilité (écart ≤ 0 → 1000, 1 → 500, ≥ 2 → 0) ; alerte si < 500 |
| 4 | Sol & humidité | 20 | moyenne de 4 sous-facteurs (type, drainage, pH du sol effectif ; humidité de la zone) : présent dans la liste de la plante 1000, sinon 0 ; **bloquant** si sol alcalin et plante strictement acide ; alertes pH hors préférences, sol mal drainé |
| 5 | Dimensions | 15 | ratio plante / limite : ≤ 1 → 1000 ; ≤ 1,25 → 600 ; ≤ 1,5 → 200 (alertes) ; > 1,5 → 200 et **bloquant** ; sans limite → 1000 |
| 6 | Usages, feuillage & ambiance | 10 | sans ambiance : `(700·u + 300·f)/1000` ; avec ambiance : `(500·u + 200·f + 300·a)/1000` |
| 7 | Entretien & vent | 5 | moyenne entretien (écart au maximum) et vent (exposition vs tolérance) ; alerte si trop exposé |

`u` = part des usages souhaités couverts ; `f` = feuillage (indifférent 1000, identique 1000, semi-persistant 500, autre 0).

**Adéquation à l'ambiance `a`** : 1000 si la plante est rattachée à l'ambiance ; 600 si ses caractéristiques s'y prêtent ; 0 sinon.

| Ambiance | Caractéristiques qui s'y prêtent (600) |
| :--- | :--- |
| Méditerranéen | tolérance à la sécheresse élevée et plein soleil idéal |
| Sous-bois | ombre légère ou dense idéale |
| Nourricier | fruits ou feuilles comestibles |
| Sauvage / biodiversité | mellifère |
| Champêtre | ornementale + mellifère + au moins 2 mois de floraison |
| Japonais, Contemporain, Exotique | aucune (rattachement explicite uniquement) |

### 7.4 Verdicts et tri
* Au moins une alerte bloquante → **Déconseillé**.
* ≥ 80 sans alerte « warning » → **Très adapté** ; ≥ 55 → **Adapté avec réserves** ; ≥ 30 → **Peu adapté** ; sinon **Déconseillé**.
* Tri : non bloquées d'abord, score décroissant, nom, identifiant. Affichage : 60 premières ; filtres « fiches vérifiées », « masquer Déconseillé » (actif par défaut), catégorie.

### 7.5 Exemples de contrôle (tests unitaires)
Jardin −12 °C, sécheresse moyenne, limoneux / bon / neutre, arrosage moyen ; zone point plein soleil, abritée, humidité normale, rayon 1,2 m, hauteur 1,8 m, usages ornemental + mellifère, entretien max faible :
* Lavande vraie : **100/100, Très adapté** ; avec l'ambiance méditerranéenne non étiquetée : critère 6 = 9/10 ; étiquetée : 10/10.
* Érable du Japon : **73/100, Déconseillé** (trop grand : bloquant).

### 7.6 Onglet Idées
Réglages : ambiance, sol (jardin ou un secteur), exposition, catégorie, hauteur max. Zone virtuelle évaluée avec ces réglages ; sont retenues les plantes **non bloquées**, **compatibles avec l'ambiance** (`a > 0`) et, si le sol est renseigné, avec un critère Sol ≥ 15/20. 60 résultats au plus, triés par score ; boutons « Fiche » et « Assigner à la zone sélectionnée ». Résultat recalculé seulement quand un réglage, le sol ou le catalogue change.

---

## 8. Interface

* **Barre de menus** : Fichier (nouveau projet image / dessin, ouvrir, enregistrer, enregistrer sous, quitter), Édition (annuler, rétablir), Options (photos automatiques), Aide (vue IGN, À propos & licences).
* **Barre d'outils** : Sélection, Plante isolée, Massif, Haie, Mesurer, Orienter le Nord, Secteur de sol ; image : Détecter le jardin (+ curseur Tolérance), Contour du terrain, Calibrer l'échelle ; dessin : Contour du terrain, Obstacle (+ type). Annuler, Rétablir, Recentrer ; légende des statuts.
* **Carte** : zoom molette (centré sur le curseur), déplacement clic milieu ou glisser sur le vide, rose des vents, barre d'échelle, attribution, info-bulle de survol (zone, plante, score), aide de l'outil actif.
* **Onglets** : Jardin, Zone, Catalogue, Idées.
* **Raccourcis** : Ctrl+S, Ctrl+Z, Ctrl+Y / Ctrl+Maj+Z, Suppr (zone), Entrée / double-clic (terminer un tracé), Retour arrière (retirer le dernier point), Échap (abandonner / revenir à Sélection).

---

## 9. Exigences non fonctionnelles

* Hors ligne par défaut ; aucune donnée de projet n'est envoyée.
* Scoring de 500 plantes < 50 ms ; détection du contour < 1 s sur une image réduite à 640 px.
* Robustesse : aucune donnée inconnue ou fiche incomplète ne provoque d'erreur ; image manquante signalée.
* Enregistrement atomique (fichier temporaire du même dossier, `fsync`, remplacement, copie `.bak`).
* Historique : 100 niveaux d'annulation.
* Tests unitaires du cœur : vocabulaire, modèle (identifiants, secteurs), géométrie (aires, triangulation, point d'ancrage), recherche, conversions, détection (jardin en L synthétique, débordement, entrée invalide), scoring (exemples §7.5, seuils, ambiance).
* Licences : code GPL-3.0-or-later ; données tierces sous leurs propres licences (`NOTICE.md`), crédits dans « À propos & licences ».

---

## 10. Points ouverts

1. Relecture progressive des fiches brouillon du catalogue embarqué.
2. Fiabilité de la détection du contour sur des orthophotos réelles (réglage par défaut de la tolérance).
3. Persistance des préférences de l'application entre deux sessions.
4. Couverture EIVE limitée aux espèces européennes : compléter l'exposition et le sol des plantes exotiques par d'autres sources ouvertes (ex. Baseflor, USDA PLANTS), licence vérifiée.
