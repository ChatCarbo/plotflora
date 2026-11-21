# Cahier des spécifications — PlotFlora (Version 1.1)

**Nom du projet :** PlotFlora (`plotflora`)
**Description :** Logiciel local de planification de jardin à partir d'une orthophoto IGN ou d'un plan dessiné : zones de plantation (points, massifs, haies), catalogue de plantes alimenté par des sources ouvertes, algorithme de recommandation explicable.
**Date :** 21 novembre 2026
**Statut :** Spécification V1.1 — remplace la V1.0

---

## 0. Changements par rapport à la V1.0

| Sujet | V1.0 | V1.1 |
| :--- | :--- | :--- |
| Fond de plan | Capture d'une vue aérienne quelconque | Orthophoto **IGN / Géoportail** recommandée, ou **plan dessiné** en mètres |
| Échelle | Reportée en V2 | **En V1** (calibration par deux points + distance connue) |
| Emplacements | Pastilles ponctuelles | **Zones** de trois types : `point`, `area` (massif), `line` (haie), avec calcul des **quantités** |
| Données plantes | « Source structurée / web / IA » non précisée | **Wikidata** (CC0), **EIVE** (CC BY 4.0), **Wikimedia Commons**, **Wikipédia** (faits extraits uniquement) |
| Données manquantes | `null` **et** `"unknown"` | **`null` uniquement** |
| Scoring | 6 critères, règles ambiguës, exemple faux | 7 critères définis par des tables, **alertes bloquantes**, arithmétique entière, exemples recalculés |
| CSV | Décrit en V1 | Reporté en V2 |
| Portabilité | Le projet dépend du catalogue global | Le projet embarque une **copie des plantes assignées** |

---

## 1. Positionnement et périmètre

### 1.1 Objectif produit
PlotFlora aide un particulier à choisir **quelle plante mettre à quel endroit** de son jardin :
1. Obtenir un fond de plan : orthophoto IGN ou dessin du terrain.
2. Orienter (Nord) et mettre à l'échelle (mètres).
3. Tracer des **zones** : plante isolée, massif, haie.
4. Décrire le jardin (climat, sol) et chaque zone (lumière, vent, humidité, contraintes).
5. Obtenir pour chaque zone un **classement explicable** des plantes adaptées, et les **quantités** à planter.
6. Tout sauvegarder localement.

### 1.2 Périmètre V1
* Projet : création, ouverture, sauvegarde atomique.
* Fond de plan : import d'image (IGN conseillé) **ou** dessin d'un polygone de terrain avec obstacles.
* Orientation Nord et **calibration d'échelle**.
* Zones `point` / `area` / `line` : création, édition des sommets, déplacement, suppression.
* Contexte global du jardin et paramètres par zone.
* Catalogue de plantes livré avec l'application (généré à partir de sources ouvertes), recherche locale floue, **recherche en ligne sur action explicite** (Wikidata + Wikipédia), validation utilisateur.
* Photos libres de Wikimedia Commons ou photo personnelle, avec crédits.
* Scoring sur 100 points, alertes bloquantes, explications, quantités.
* Écran « Crédits & licences ».

### 1.3 Évolutions V2 (hors périmètre)
* Récupération **automatique** de l'orthophoto IGN à partir d'une adresse (service Géoplateforme).
* Simulation d'ombre portée à partir des obstacles et de leur hauteur.
* Export d'une image annotée et d'une liste de plantes avec quantités (PDF/CSV).
* Import / export CSV du catalogue.
* Extraction assistée par IA des fiches Wikipédia (optionnelle, sur consentement).

### 1.4 Choix techniques
* Langage : **Rust** (édition 2021 ou plus récente).
* Interface : framework natif à arrêter (ex. Slint ou egui) — hors du périmètre de ce document.
* Plateforme prioritaire : **Windows 10/11** ; le code métier (modèle, scoring, pipeline) reste indépendant de l'interface et de l'OS.
* Découpage conseillé en crates : `plotflora-core` (modèle + scoring, sans I/O), `plotflora-data` (fichiers, cache, sources en ligne), `plotflora-app` (UI), `plotflora-catalog-builder` (outil hors application, §5.2).

---

## 2. Fond de plan

### 2.1 Mode `image` — orthophoto
**Source recommandée : Géoportail / IGN (BD ORTHO).** Images aériennes orthorectifiées à 20 cm de résolution, sous **Licence Ouverte Etalab 2.0** : réutilisation gratuite, y compris commerciale, avec mention de la source.

Procédure conseillée affichée dans l'aide de l'application :
1. Sur geoportail.gouv.fr, rechercher l'adresse et afficher la couche « Photographies aériennes ».
2. Zoomer pour que le jardin occupe la plus grande partie de l'écran, avec une marge.
3. Garder le Nord en haut (orientation par défaut de Géoportail).
4. Utiliser l'outil de mesure de distance sur un élément net (mur de maison, clôture) et **noter la longueur**.
5. Exporter ou capturer l'image (Windows + Maj + S), enregistrer en PNG.

Autres sources acceptées (`.png`, `.jpg`, `.jpeg`, `.webp`) : photo de drone, capture Google Maps / Google Earth **pour usage personnel**. PlotFlora ne télécharge jamais lui-même d'images Google.

Champ `map.attribution` : texte libre affiché en bas à droite de la carte ; valeur pré-remplie `© IGN – BD ORTHO` quand l'utilisateur indique la source IGN.

### 2.2 Mode `drawing` — plan dessiné
Pour un terrain masqué par la végétation, une maison neuve ou une absence d'image :
* L'utilisateur trace le **contour du terrain** (polygone) par clics, ou saisit les longueurs des côtés en mètres.
* Il ajoute des **obstacles** : `house`, `shed`, `terrace`, `existing_tree`, `wall`, `other` (polygone ou cercle, hauteur optionnelle en mètres, utilisée en V2 pour l'ombre).
* Une grille de fond (pas réglable, 1 m par défaut) facilite le tracé.
* Dans ce mode, l'unité du plan est le mètre : l'échelle est connue d'office.

### 2.3 Orientation
* `north_angle_deg` ∈ `[0, 360)`, flottant, normalisé par `rem_euclid(360.0)`.
* **Convention :** angle mesuré **dans le sens horaire** entre le haut du plan et la direction du Nord géographique. `0` = Nord en haut (cas des orthophotos IGN).
* Réglage par champ numérique, molette ou alignement à la souris (clic-glisser d'une flèche).
* La rose des vents (N, E, S, O) est redessinée en temps réel.

### 2.4 Échelle
* Mode `image` : l'utilisateur clique deux points puis saisit la distance réelle (en mètres) mesurée sur Géoportail.
* `meters_per_unit = distance_m / distance_px`.
* Tant que l'échelle n'est pas calibrée : surfaces, longueurs et quantités ne sont pas affichées, et le critère « Dimensions » n'utilise que les valeurs saisies à la main dans les zones (§3.2).

### 2.5 Repère de coordonnées
Toutes les géométries sont stockées en **unités du plan** :
* mode `image` : pixels de l'image d'origine (flottants), origine en haut à gauche, axe Y vers le bas ;
* mode `drawing` : mètres, mêmes conventions d'axes, `meters_per_unit = 1.0`.

Ce choix évite les pourcentages et rend les conversions triviales : `longueur_m = longueur_unités × meters_per_unit`.

---

## 3. Zones de plantation

### 3.1 Types
| `kind` | Usage | Géométrie | Mesure calculée |
| :--- | :--- | :--- | :--- |
| `point` | Arbre, arbuste ou vivace isolé | 1 point | — |
| `area` | Massif, couvre-sol, potager | Polygone fermé (≥ 3 sommets, non croisé) | Surface (formule du lacet) |
| `line` | Haie, bordure | Polyligne (≥ 2 sommets) | Longueur (somme des segments) |

### 3.2 Paramètres d'une zone
* `label`, `status` (`to_plant`, `existing`, `to_replace`), `notes`.
* Conditions : `sun_exposure`, `wind_exposure`, `moisture` (chacun peut être `null` = inconnu).
* Contraintes (toutes optionnelles) :
  * `max_height_m` — tous types ;
  * `available_radius_m` — `point` uniquement ;
  * `max_width_m` — `line` uniquement (largeur de haie souhaitée).
* Préférences : `desired_uses`, `foliage_preference`, `max_maintenance`.
* Plante : `assigned_plant_id`, `custom_photo_path`.

Définitions affichées dans l'interface pour l'exposition :
| Valeur | Définition |
| :--- | :--- |
| `full_sun` | ≥ 6 h de soleil direct en été |
| `partial_shade` | 3 à 6 h de soleil direct |
| `light_shade` | < 3 h de soleil direct, lumière filtrée |
| `dense_shade` | Pas de soleil direct |

### 3.3 Quantités
Espacement `e` = `plant.dimensions.planting_spacing_m`, à défaut `plant.dimensions.mature_spread_m`. Si `e` est inconnu ou l'échelle non calibrée, aucune quantité n'est affichée.
* `point` : 1.
* `line` : `floor(longueur_m / e) + 1`.
* `area` : `ceil(surface_m² / e²)`.

L'utilisateur peut forcer la quantité (`quantity_override`).

### 3.4 Statuts et scoring
* `to_plant`, `to_replace` : l'algorithme peut être lancé.
* `existing` : la zone sert de repère (et d'obstacle en V2) ; pas de recommandation.

---

## 4. Écrans et parcours

### 4.1 Accueil
* **Nouveau projet** : nom, choix du mode (`image` ou `drawing`), puis assistant : image → orientation → échelle.
* **Ouvrir** : sélection d'un `project.json`. Si l'image est introuvable : visuel de secours + bouton « Relier le fichier ».
* **Récents** : 10 derniers projets.

### 4.2 Éditeur
* **Carte** (centre) : zoom, déplacement, rose des vents, barre d'échelle (si calibrée), attribution.
* **Barre d'outils** : sélection, ajouter point / massif / haie, éditer sommets, mesurer, orientation, échelle, obstacles (mode dessin).
* **Panneau « Contexte du jardin »** : `min_winter_temp_c`, `summer_drought_risk`, `soil_type`, `soil_drainage`, `soil_ph`, `watering_availability`. Chaque liste propose « Inconnu ».
* **Panneau « Zone »** (zone sélectionnée) : paramètres (§3.2), surface / longueur, plante assignée et photo, quantité, recommandations.
* **Annuler / Rétablir** sur toutes les modifications de l'éditeur (historique en mémoire, 100 niveaux).

### 4.3 Recommandations
* Bouton « Lancer l'algorithme » ; recalcul automatique quand un paramètre de la zone ou du jardin change et que la liste est ouverte.
* Liste triée (§7.6). Pour chaque plante : miniature, nom commun, nom latin, catégorie, badge « Fiche non vérifiée » si `verified = false`, score `XX/100`, verdict, détail par critère, points positifs, alertes, quantité, bouton « Choisir cette plante ».
* Filtres : catégorie, fiches vérifiées uniquement, masquer les plantes « Déconseillé ».

### 4.4 Catalogue
* Recherche floue (§5.3), fiche détaillée, édition, bouton « Valider la fiche ».
* Bouton « Rechercher en ligne » (§5.4), désactivé hors connexion avec message explicite.

### 4.5 Crédits & licences
Liste des sources utilisées (IGN, Wikidata, EIVE, Wikipédia) et, pour chaque photo, auteur, licence et lien.

---

## 5. Données plantes : sources et pipeline

### 5.1 Sources retenues

| Source | Licence | Rôle | Accès |
| :--- | :--- | :--- | :--- |
| **Wikidata** | CC0 1.0 | Identité : nom latin, noms communs FR, synonymes, rang, lien Wikipédia, photo | En ligne (SPARQL / API) + catalogue embarqué |
| **EIVE** — valeurs indicatrices de type Ellenberg pour 8 908 espèces européennes (Tichý et al., 2023, Zenodo 7427088) | CC BY 4.0 | Lumière, humidité, pH | **Hors ligne** : index embarqué dans l'application |
| **Wikimedia Commons** | Licence propre à chaque fichier (toujours libre) | Photos | En ligne, puis copie locale |
| **Wikipédia FR** | CC BY-SA 4.0 | Rusticité, hauteur, envergure, toxicité : **valeurs factuelles extraites**, jamais de phrases recopiées | En ligne |
| Saisie utilisateur | — | Complète et valide tout | — |

**Sources exclues :** sites marchands (textes protégés, extraction non autorisée) ; Plants For A Future (licence non commerciale) ; Google Maps comme source téléchargée par l'application.

### 5.2 Catalogue embarqué et outil `plotflora-catalog-builder`
Outil Rust exécuté **par le développeur**, pas par l'utilisateur final :
1. Lit une liste d'espèces de départ (`seed_species.txt`, noms latins, quelques centaines d'espèces de jardin courantes en France).
2. Interroge Wikidata (identité, noms FR, image, lien Wikipédia).
3. Lit le fichier Excel EIVE (crate `calamine`) et en tire un **index compact** `eive_index.json` (nom latin → L, M, R).
4. Applique les conversions (§5.5) et produit `plants_catalog.seed.json` (fiches `verified: false` sauf relecture manuelle) et `eive_index.json`.
5. Ces deux fichiers sont **intégrés au binaire** (`include_bytes!`) : l'application fonctionne entièrement hors ligne.

### 5.3 Recherche locale (hors ligne)
1. Normalisation de la saisie : minuscules, suppression des accents, suppression des mots vides (`plante`, `pied de`, `un`, `une`, `de`, `du`).
2. Recherche floue (distance de Levenshtein ou trigrammes) sur : nom commun, nom latin, synonymes, cultivar.
3. Ordre de recherche : catalogue utilisateur → catalogue embarqué → cache.
4. Si plusieurs candidats sont proches, afficher la liste (nom commun, nom latin, photo) et laisser choisir.

### 5.4 Recherche en ligne (action explicite)
Déclenchée uniquement par le bouton « Rechercher en ligne ».
1. **Wikidata** — recherche par texte (langue `fr`), filtrée sur les taxons. Liste de candidats si ambiguïté.
2. **Détails Wikidata** — exemple de requête SPARQL :
   ```sparql
   SELECT ?taxon ?nomFr ?image ?article WHERE {
     ?taxon wdt:P225 "Lavandula angustifolia" .
     OPTIONAL { ?taxon wdt:P1843 ?nomFr . FILTER(LANG(?nomFr) = "fr") }
     OPTIONAL { ?taxon wdt:P18 ?image }
     OPTIONAL { ?article schema:about ?taxon ;
                         schema:isPartOf <https://fr.wikipedia.org/> }
   }
   ```
3. **EIVE** — recherche du nom latin dans l'index embarqué (aucun appel réseau).
4. **Wikipédia FR** — lecture de l'article ; extraction **par règles** (expressions régulières) des seules valeurs factuelles : rusticité (« rustique jusqu'à −15 °C », « zone USDA 7 »), hauteur, largeur, mentions de toxicité. Le texte de l'article n'est **pas** stocké, seul son lien l'est.
5. **Photo** — téléchargement d'une vignette Commons (largeur 800 px) avec auteur, licence et URL de la page du fichier.
6. Le brouillon obtenu passe par la normalisation (§5.5), la validation (§5.7) puis la revue utilisateur.

Règles réseau :
* En-tête `User-Agent` identifiant l'application et un contact, conformément aux règles d'usage de Wikimedia ; délai d'expiration de 10 s ; pas plus d'une requête par seconde.
* Hors connexion ou en cas d'erreur : message clair, saisie manuelle possible.

### 5.5 Conversions vers le vocabulaire contrôlé
**Lumière (EIVE L, échelle 1–9, valeur arrondie à l'entier) :**
| L | `exposure.ideal` | `exposure.tolerated` |
| :---: | :--- | :--- |
| 9 | `full_sun` | — |
| 8 | `full_sun` | `partial_shade` |
| 7 | `full_sun`, `partial_shade` | `light_shade` |
| 6 | `partial_shade` | `full_sun`, `light_shade` |
| 5 | `partial_shade`, `light_shade` | `full_sun` |
| 4 | `light_shade` | `partial_shade`, `dense_shade` |
| 3 | `light_shade`, `dense_shade` | `partial_shade` |
| 1–2 | `dense_shade` | `light_shade` |

**Humidité (EIVE M, échelle 1–12) :** 1–3 → `dry` ; 4–5 → `dry`, `normal` ; 6 → `normal` ; 7 → `normal`, `moist` ; 8–9 → `moist` ; 10–12 → `moist`, `wet`.

**pH (EIVE R, échelle 1–9) :** 1–3 → `acid` ; 4–5 → `acid`, `neutral` ; 6 → `neutral` ; 7 → `neutral`, `alkaline` ; 8–9 → `alkaline`, `neutral`.

> Les échelles exactes des colonnes harmonisées sont à confirmer à l'import du fichier EIVE. Les valeurs EIVE décrivent l'optimum **en milieu naturel** : tout champ qui en dérive est ajouté à `meta.uncertain_fields`.

**Zones USDA → `min_temp_c` :** borne basse de la zone `n` : `°F = −60 + 10 × (n − 1)`, puis `°C = (°F − 32) × 5/9`, arrondi à l'entier inférieur (ex. zone 8 → −13 °C). Les demi-zones `a`/`b` ajoutent 5 °F pour `b`.

**Unités :** `cm` → `m` (÷ 100) ; plages « 60–80 cm » → valeur haute.

### 5.6 Provenance et licences
Chaque champ renseigné automatiquement est tracé dans `meta.field_sources` :
```json
"field_sources": {
  "latin_name": "wikidata:Q42851",
  "exposure.ideal": "eive:L=7",
  "climate.min_temp_c": "wikipedia_fr:Lavande_vraie",
  "photo": "commons"
}
```
La photo porte ses propres crédits (§6.4). L'écran « Crédits & licences » est construit à partir de ces données.

### 5.7 Validation et statut
* Désérialisation typée (`serde`) : toute valeur hors vocabulaire est rejetée et le champ reste `null`, avec un message dans le rapport d'import.
* Les champs absents ou incertains restent `null` / `[]` — jamais inventés.
* `meta.verified = false` pour tout brouillon ; l'utilisateur corrige, puis « Valider la fiche » → `verified = true`, `meta.source` conservé, `updated_at` mis à jour.
* **Les fiches non vérifiées participent au scoring**, avec le badge « Fiche non vérifiée ».

### 5.8 Protection et fusion
* Une fiche `verified = true` n'est jamais écrasée. Une nouvelle recherche propose seulement de **compléter les champs `null` ou vides**, champ par champ.
* L'`id` d'une fiche ne change jamais après sa création, même si le nom latin est corrigé.

### 5.9 Cache
* `lookup_cache.json` : clé = (source, requête normalisée) ; valeur = réponse brute + date.
* Durée de validité : **180 jours** ; bouton « Actualiser » pour forcer une nouvelle requête.

---

## 6. Modèle de données

### 6.1 Emplacement des fichiers
| Fichier | Emplacement |
| :--- | :--- |
| `project.json` + image | Dossier choisi par l'utilisateur |
| `plants_catalog.json` (catalogue utilisateur) | Dossier de données de l'application (`%APPDATA%\PlotFlora\` sous Windows) |
| `photos/` | Même dossier de données |
| `lookup_cache.json` | Même dossier de données |
| Catalogue embarqué + index EIVE | Intégrés au binaire |

### 6.2 Conventions
* **Inconnu = `null`.** Aucune énumération ne contient de valeur `unknown`. Côté Rust : `Option<T>`.
* **Énumérations** sérialisées en `snake_case` (`#[serde(rename_all = "snake_case")]`).
* **Chemins** relatifs au dossier du fichier qui les contient, séparateur `/` quel que soit l'OS.
* **Identifiants :**
  * plante : `snake_case` du nom latin (+ `_` + cultivar s'il existe) à la création ; en cas de collision, suffixe `_2`, `_3`… ; immuable ensuite ;
  * zone : `z_` + numéro tiré de `next_zone_seq` (jamais réutilisé après suppression).
* **Versions :** champ `schema_version` (SemVer) dans chaque fichier. Version majeure différente → refus avec message ; version mineure plus ancienne → migration automatique étape par étape ; champs inconnus ignorés à la lecture.
* **Nombres :** températures en entiers (°C) ; dimensions en `f64` (mètres).

### 6.3 `project.json`
```json
{
  "schema_version": "1.1.0",
  "project_id": "mon_jardin",
  "name": "Mon jardin",
  "map": {
    "source": "image",
    "image_path": "vue_aerienne.png",
    "attribution": "© IGN – BD ORTHO",
    "north_angle_deg": 0.0,
    "scale": {
      "p1": [412.0, 380.5],
      "p2": [903.0, 391.0],
      "distance_m": 9.4
    },
    "boundary": null,
    "obstacles": []
  },
  "environment": {
    "min_winter_temp_c": -12,
    "summer_drought_risk": "medium",
    "soil_type": "loamy",
    "soil_drainage": "good",
    "soil_ph": "neutral",
    "watering_availability": "medium"
  },
  "next_zone_seq": 3,
  "zones": [
    {
      "id": "z_1",
      "kind": "point",
      "label": "Massif Sud-Est",
      "points": [[1240.0, 562.0]],
      "status": "to_plant",
      "sun_exposure": "full_sun",
      "wind_exposure": "sheltered",
      "moisture": "normal",
      "max_height_m": 1.8,
      "available_radius_m": 1.2,
      "max_width_m": null,
      "desired_uses": ["ornamental", "pollinator"],
      "foliage_preference": "any",
      "max_maintenance": "low",
      "assigned_plant_id": "lavandula_angustifolia",
      "quantity_override": null,
      "custom_photo_path": null,
      "notes": ""
    },
    {
      "id": "z_2",
      "kind": "line",
      "label": "Haie Nord",
      "points": [[120.0, 95.0], [1480.0, 102.0]],
      "status": "to_plant",
      "sun_exposure": "partial_shade",
      "wind_exposure": "exposed",
      "moisture": null,
      "max_height_m": 2.0,
      "available_radius_m": null,
      "max_width_m": 1.0,
      "desired_uses": ["hedge"],
      "foliage_preference": "evergreen",
      "max_maintenance": "any",
      "assigned_plant_id": null,
      "quantity_override": null,
      "custom_photo_path": null,
      "notes": ""
    }
  ],
  "plants_snapshot": {
    "lavandula_angustifolia": { "...": "copie complète de la fiche au moment de l'assignation" }
  }
}
```
* `map.source = "drawing"` : `image_path` et `scale` sont `null`, `boundary` contient le polygone du terrain en mètres.
* `obstacles[]` : `{ "kind": "house", "points": [...], "height_m": 6.0 }` ou `{ "kind": "existing_tree", "center": [x, y], "radius_m": 2.5, "height_m": 8.0 }`.
* `plants_snapshot` : à l'ouverture, si le catalogue contient une version différente d'une plante assignée, proposer « Mettre à jour » ou « Garder la version du projet ». Si la plante est absente du catalogue, la copie du projet est utilisée et peut être importée.

### 6.4 Fiche plante (`plants_catalog.json`)
```json
{
  "id": "lavandula_angustifolia",
  "common_name": "Lavande vraie",
  "latin_name": "Lavandula angustifolia",
  "cultivar": null,
  "category": "perennial",
  "photo": {
    "file": "photos/lavandula_angustifolia.jpg",
    "source_url": "https://commons.wikimedia.org/wiki/File:...",
    "author": "…",
    "license": "CC BY-SA 4.0",
    "license_url": "https://creativecommons.org/licenses/by-sa/4.0/"
  },
  "climate": { "min_temp_c": -15, "drought_tolerance": "high", "wind_tolerance": "medium" },
  "exposure": { "ideal": ["full_sun"], "tolerated": [] },
  "soil": {
    "types": ["sandy", "loamy", "chalky"],
    "drainage": ["fast", "good"],
    "ph": ["neutral", "alkaline"],
    "moisture": ["dry", "normal"]
  },
  "dimensions": {
    "mature_height_m": 0.8,
    "mature_spread_m": 0.9,
    "planting_spacing_m": 0.5,
    "growth_rate": "medium",
    "root_system": "non_invasive"
  },
  "characteristics": {
    "foliage": "evergreen",
    "uses": ["ornamental", "pollinator", "aromatic"],
    "flowering_months": [6, 7, 8],
    "interest_seasons": ["summer"],
    "flower_color": "violet",
    "autumn_color": null
  },
  "care": {
    "maintenance_level": "low",
    "water_need": "very_low",
    "pruning_months": [8, 9],
    "non_toxic_pets": false,
    "has_thorns": false
  },
  "notes": "Redoute les sols lourds détrempés en hiver.",
  "tags": ["soleil", "sec"],
  "links": { "wikidata": null, "wikipedia_fr": null },
  "meta": {
    "source": "manual",
    "verified": true,
    "uncertain_fields": [],
    "field_sources": {},
    "updated_at": "2026-11-21"
  }
}
```
`photo` vaut `null` s'il n'y a pas de photo : un visuel par défaut selon `category` est alors affiché. `flower_color` et `autumn_color` sont des textes libres, uniquement affichés.

### 6.5 Vocabulaire contrôlé

**Valeurs décrivant le jardin ou la zone**
| Champ | Valeurs |
| :--- | :--- |
| `status` | `to_plant`, `existing`, `to_replace` |
| `kind` | `point`, `area`, `line` |
| `sun_exposure` | `full_sun`, `partial_shade`, `light_shade`, `dense_shade` |
| `wind_exposure` | `sheltered`, `moderate`, `exposed` |
| `moisture` | `dry`, `normal`, `moist`, `wet` |
| `soil_type` | `loamy`, `clay`, `sandy`, `chalky`, `humus` |
| `soil_drainage` | `fast`, `good`, `moderate`, `poor` |
| `soil_ph` | `acid`, `neutral`, `alkaline` |
| `summer_drought_risk` | `low`, `medium`, `high` |
| `watering_availability` | `low`, `medium`, `high` |
| `foliage_preference` | `any`, `evergreen`, `deciduous` |
| `max_maintenance` | `any`, `very_low`, `low`, `medium`, `high` |
| `desired_uses` | voir `uses` |
| `obstacle.kind` | `house`, `shed`, `terrace`, `existing_tree`, `wall`, `other` |

**Valeurs décrivant la plante**
| Champ | Valeurs |
| :--- | :--- |
| `category` | `tree`, `shrub`, `perennial`, `climber`, `groundcover`, `vegetable_herb` |
| `exposure.ideal` / `.tolerated` | mêmes valeurs que `sun_exposure` |
| `soil.types` / `.drainage` / `.ph` / `.moisture` | mêmes valeurs que les champs du jardin correspondants |
| `drought_tolerance`, `wind_tolerance` | `low`, `medium`, `high` |
| `water_need` | `very_low`, `low`, `medium`, `high` |
| `maintenance_level` | `very_low`, `low`, `medium`, `high` |
| `foliage` | `evergreen`, `semi_evergreen`, `deciduous` |
| `uses` | `ornamental`, `edible_fruit`, `edible_leaf`, `aromatic`, `pollinator`, `hedge`, `groundcover`, `shade_provider` |
| `growth_rate` | `slow`, `medium`, `fast` |
| `root_system` | `non_invasive`, `spreading`, `suckering`, `clinging` |
| `interest_seasons` | `spring`, `summer`, `autumn`, `winter` |
| `flowering_months`, `pruning_months` | entiers 1 à 12 |
| `meta.source` | `manual`, `seed`, `wikidata`, `wikipedia_fr`, `eive`, `mixed` |

Les énumérations « préférence » (avec `any`) et « plante » (sans `any`) sont des **types Rust distincts**.

---

## 7. Algorithme de scoring

### 7.1 Principe
1. Les **règles bloquantes** (§7.4) sont évaluées : si l'une est vraie, le verdict est « Déconseillé », quel que soit le score.
2. Le **score** est la somme de 7 critères, chacun arrondi à l'entier.
3. Le **verdict** dépend du score et des alertes (§7.5).

### 7.2 Arithmétique
* Chaque critère calcule un facteur `c` en **millièmes** (entier `u32`, de 0 à 1000).
* Points du critère : `(poids × c + 500) / 1000` (division entière, arrondi au plus proche, demi vers le haut).
* Score total `S` = somme des points des 7 critères. Le détail affiché est donc toujours égal au total.
* **Donnée inconnue** (côté jardin/zone **ou** côté plante) : facteur `UNKNOWN = 600`, et une information « donnée inconnue : … » est ajoutée. Une donnée inconnue ne vaut jamais mieux qu'une exposition seulement tolérée.

### 7.3 Critères

| # | Critère | Poids |
| :---: | :--- | :---: |
| 1 | Exposition lumineuse | 25 |
| 2 | Rusticité (gel) | 15 |
| 3 | Sécheresse & arrosage | 10 |
| 4 | Sol & humidité | 20 |
| 5 | Dimensions | 15 |
| 6 | Usages & feuillage | 10 |
| 7 | Entretien & vent | 5 |
| | **Total** | **100** |

**1. Exposition (25)**
* zone `sun_exposure` ∈ `ideal` → 1000 ;
* ∈ `tolerated` → 600, alerte « exposition seulement tolérée » ;
* sinon → 0, **règle bloquante** ;
* `sun_exposure` inconnu ou `ideal` vide → `UNKNOWN`.

**2. Rusticité (15)** — marge = `min_winter_temp_c − plant.min_temp_c` (positive = la plante supporte plus froid que le jardin).
* marge ≥ 0 → 1000 ;
* −3 ≤ marge < 0 → 400, alerte « protection hivernale nécessaire » ;
* marge < −3 → 0, **règle bloquante** ;
* l'une des deux valeurs inconnue → `UNKNOWN`.

**3. Sécheresse & arrosage (10)** — `c = (600 × d + 400 × w) / 1000`
* `d` = table [tolérance de la plante × risque du jardin], puis bonus d'arrosage, plafonné à 1000 :

  | `drought_tolerance` \ `summer_drought_risk` | `low` | `medium` | `high` |
  | :--- | :---: | :---: | :---: |
  | `low` | 1000 | 500 | 0 |
  | `medium` | 1000 | 1000 | 500 |
  | `high` | 1000 | 1000 | 1000 |

  Bonus selon `watering_availability` : `low` +0, `medium` +250, `high` +500. Valeur inconnue → `d = UNKNOWN`.
* `w` = adéquation besoin en eau / disponibilité, avec les rangs `very_low`=0, `low`=1, `medium`=2, `high`=3 (besoin) et `low`=1, `medium`=2, `high`=3 (disponibilité) : écart = besoin − disponibilité ; écart ≤ 0 → 1000 ; écart = 1 → 500 ; écart ≥ 2 → 0. Valeur inconnue → `UNKNOWN`.
* Si `c` < 500 : alerte « arrosage estival indispensable ».

**4. Sol & humidité (20)** — moyenne entière de quatre sous-facteurs : type de sol, drainage, pH (du jardin) et humidité (de la zone). Pour chacun : valeur ∈ liste de la plante → 1000 ; sinon → 0 ; valeur ou liste inconnue → `UNKNOWN`.
* **Règle bloquante** : `soil_ph = alkaline` et `plant.soil.ph = ["acid"]` (plante strictement calcifuge).
* Alerte si `soil_drainage = poor` et `poor` absent de la liste de la plante : « risque d'asphyxie des racines ».

**5. Dimensions (15)** — `c = min(f_hauteur, f_largeur)`. Pour chaque contrainte, ratio `r = valeur_plante / limite` :
* pas de limite saisie → 1000 (l'utilisateur n'a pas de contrainte) ;
* limite saisie mais valeur plante inconnue → `UNKNOWN` ;
* `r ≤ 1,00` → 1000 ;
* `1,00 < r ≤ 1,25` → 600, alerte « taille régulière nécessaire » ;
* `1,25 < r ≤ 1,50` → 200, alerte « dépassement important » ;
* `r > 1,50` → 200, **règle bloquante**.

Contraintes vérifiées :
| Zone | Hauteur | Largeur |
| :--- | :--- | :--- |
| `point` | `mature_height_m` / `max_height_m` | (`mature_spread_m` / 2) / `available_radius_m` |
| `line` | `mature_height_m` / `max_height_m` | `mature_spread_m` / `max_width_m` |
| `area` | `mature_height_m` / `max_height_m` | — (la quantité s'adapte à la surface) |

**6. Usages & feuillage (10)** — `c = (700 × u + 300 × f) / 1000`
* `u` = `1000 × |desired_uses ∩ uses| / |desired_uses|` (division entière) ; 1000 si aucun usage demandé.
* `f` : préférence `any` → 1000 ; feuillage identique → 1000 ; plante `semi_evergreen` → 500 ; autre → 0 ; feuillage inconnu → `UNKNOWN`.

**7. Entretien & vent (5)** — `c = (m + v) / 2`
* `m` : `max_maintenance = any` → 1000 ; sinon, avec les rangs `very_low`=0 … `high`=3, écart = plante − maximum ; ≤ 0 → 1000 ; 1 → 500 ; ≥ 2 → 0.
* `v` : rangs d'exposition `sheltered`=0, `moderate`=1, `exposed`=2 ; rangs de tolérance `low`=0, `medium`=1, `high`=2 ; écart = exposition − tolérance ; ≤ 0 → 1000 ; 1 → 500 ; ≥ 2 → 0 et alerte « trop exposé au vent ».

### 7.4 Règles bloquantes (récapitulatif)
| Règle | Message |
| :--- | :--- |
| Exposition hors `ideal` et hors `tolerated` | « Exposition inadaptée : … » |
| Marge de rusticité < −3 °C | « Gèlera : rustique jusqu'à X °C, jardin à Y °C » |
| Plante strictement calcifuge en sol alcalin | « Sol calcaire incompatible » |
| Hauteur ou largeur > 150 % de la limite | « Beaucoup trop grand pour l'emplacement » |

### 7.5 Verdicts
Les alertes ont trois niveaux : `blocking`, `warning`, `info` (donnée inconnue, fiche non vérifiée).
| Condition | Verdict |
| :--- | :--- |
| Au moins une alerte `blocking` | **Déconseillé** |
| `S ≥ 80` et aucune alerte `warning` | **Très adapté** |
| `S ≥ 55` (y compris `S ≥ 80` avec alertes) | **Adapté avec réserves** |
| `30 ≤ S < 55` | **Peu adapté** |
| `S < 30` | **Déconseillé** |

### 7.6 Tri
1. Plantes sans alerte bloquante d'abord ;
2. score décroissant ;
3. nom commun (ordre alphabétique, insensible à la casse et aux accents) ;
4. `id`.

### 7.7 Format de sortie
```rust
pub struct Recommendation {
    pub plant_id: String,
    pub score: u8,                    // 0..=100
    pub verdict: Verdict,
    pub breakdown: Vec<CriterionScore>, // 7 entrées : critère, points, poids
    pub reasons: Vec<String>,         // points positifs
    pub alerts: Vec<Alert>,           // { level: Blocking | Warning | Info, message }
    pub quantity: Option<u32>,
}
```

### 7.8 Exemples de contrôle
Jardin : −12 °C, sécheresse `medium`, sol `loamy` / `good` / `neutral`, arrosage `medium`.
Zone `point` : `full_sun`, `sheltered`, `normal`, rayon 1,2 m, hauteur max 1,8 m, usages `ornamental` + `pollinator`, feuillage `any`, entretien max `low`.

**Lavande vraie — 100/100, Très adapté**
| Critère | Calcul | Points |
| :--- | :--- | :---: |
| Exposition | `full_sun` ∈ ideal → 1000 | 25 |
| Rusticité | marge −12 − (−15) = 3 → 1000 | 15 |
| Sécheresse & arrosage | d = min(1000, 1000 + 250) ; w = 1000 | 10 |
| Sol & humidité | 4 × 1000 | 20 |
| Dimensions | 0,8/1,8 et 0,45/1,2 → 1000 | 15 |
| Usages & feuillage | u = 1000, f = 1000 | 10 |
| Entretien & vent | m = 1000 ; v = 1000 | 5 |

**Érable du Japon — 73/100, Déconseillé (bloquant)**
| Critère | Calcul | Points |
| :--- | :--- | :---: |
| Exposition | `full_sun` ∈ tolerated → 600 (alerte) | 15 |
| Rusticité | marge 6 → 1000 | 15 |
| Sécheresse & arrosage | d = 500 + 250 = 750 ; w = 1000 (besoin `medium` = dispo `medium`) ; c = 850 | 9 |
| Sol & humidité | 4 × 1000 | 20 |
| Dimensions | hauteur 4,0/1,8 = 2,22 → bloquant ; largeur 2,0/1,2 = 1,67 → bloquant ; c = 200 | 3 |
| Usages & feuillage | u = 500 (1 usage sur 2) ; c = 650 | 7 |
| Entretien & vent | m = 500 (`medium` > `low`) ; v = 1000 ; c = 750 | 4 |

Avec le catalogue de l'annexe A et cette zone : Lavande, Échinacée et Sauge obtiennent 100 (Très adapté) ; Hortensia à feuilles de chêne 86, Géranium 90 et Mûrier 90 passent en « Adapté avec réserves » à cause de leurs alertes ; les arbres, grimpantes et grands arbustes sont « Déconseillé » (trop grands).

---

## 8. Exigences non fonctionnelles
* **Hors ligne par défaut :** aucune requête réseau sans action explicite (recherche en ligne, téléchargement de photo). Aucune donnée de projet n'est jamais envoyée.
* **Performance :** scoring de 500 plantes pour une zone en moins de 50 ms ; recherche floue dans 2 000 fiches en moins de 30 ms.
* **Robustesse :**
  * `null` ou fiche incomplète ne provoquent jamais d'erreur (`Option` partout, pas de `unwrap` sur des données utilisateur) ;
  * image ou photo introuvable → visuel de secours et proposition de relier le fichier.
* **Sauvegarde atomique :** écriture dans un fichier temporaire **du même dossier**, `sync_all()`, puis remplacement (`tempfile::NamedTempFile::persist`) ; conservation de la version précédente en `.bak`. Le projet et le catalogue sont sauvegardés séparément ; grâce à `plants_snapshot`, un échec sur le catalogue ne rend pas le projet incohérent.
* **Tests :** les exemples du §7.8 et un jeu de cas limites (toutes données inconnues, ratio exactement 1,25 et 1,50, marge exactement −3) sont des tests unitaires de `plotflora-core`.
* **Licences :** écran « Crédits & licences » ; mention de l'attribution IGN sur la carte ; aucune phrase de Wikipédia stockée.

---

## 9. Points ouverts
1. Liste des espèces de départ du catalogue embarqué (`seed_species.txt`) et nombre cible.
2. Choix du framework d'interface.
3. Confirmation des échelles des colonnes EIVE harmonisées au moment de l'import.
4. Intérêt d'ajouter les cultivars courants (variétés compactes notamment) au catalogue embarqué.

---

## Annexe A — Catalogue de démarrage (`plants_catalog.json`, 15 plantes, schéma 1.1)

Corrections par rapport à la V1.0 :
* *Jasminum nudiflorum* : `climber` → `shrub` (arbuste sarmenteux à palisser).
* *Betula pendula* : hauteur adulte 12 → 15 m ; tag `pollen_allergisant`.
* *Salvia officinalis* : toxicité pour les animaux mise à `null` (à vérifier), fiche repassée en `verified: false`.
* *Rubus fruticosus* : note précisant qu'il faut un cultivar sans épines.
* Renommages : `care_and_safety` → `care`, `watering_after_establishment` → `water_need`, `pruning_period` → `pruning_months` ; ajout de `cultivar`, `photo`, `planting_spacing_m`, `links`, `meta.field_sources`.

```json
{
  "schema_version": "1.1.0",
  "plants": [
    {
      "id": "amelanchier_lamarckii",
      "common_name": "Amélanchier du Canada",
      "latin_name": "Amelanchier lamarckii",
      "cultivar": null,
      "category": "shrub",
      "photo": null,
      "climate": {
        "min_temp_c": -25,
        "drought_tolerance": "medium",
        "wind_tolerance": "high"
      },
      "exposure": {
        "ideal": [
          "full_sun",
          "partial_shade"
        ],
        "tolerated": [
          "light_shade"
        ]
      },
      "soil": {
        "types": [
          "loamy",
          "sandy",
          "clay"
        ],
        "drainage": [
          "good",
          "moderate"
        ],
        "ph": [
          "acid",
          "neutral"
        ],
        "moisture": [
          "normal",
          "moist"
        ]
      },
      "dimensions": {
        "mature_height_m": 4.5,
        "mature_spread_m": 3.5,
        "planting_spacing_m": null,
        "growth_rate": "medium",
        "root_system": "non_invasive"
      },
      "characteristics": {
        "foliage": "deciduous",
        "uses": [
          "ornamental",
          "edible_fruit",
          "pollinator",
          "hedge"
        ],
        "flowering_months": [
          4,
          5
        ],
        "interest_seasons": [
          "spring",
          "summer",
          "autumn"
        ],
        "flower_color": "white",
        "autumn_color": "orange_red"
      },
      "care": {
        "maintenance_level": "low",
        "water_need": "low",
        "pruning_months": [
          2,
          3
        ],
        "non_toxic_pets": true,
        "has_thorns": false
      },
      "notes": "Floraison printanière blanche, petits fruits comestibles, belles couleurs d'automne.",
      "tags": [
        "4_saisons",
        "oiseaux",
        "facile"
      ],
      "links": {
        "wikidata": null,
        "wikipedia_fr": null
      },
      "meta": {
        "source": "manual",
        "verified": true,
        "uncertain_fields": [],
        "field_sources": {},
        "updated_at": "2026-11-21"
      }
    },
    {
      "id": "acer_palmatum",
      "common_name": "Érable du Japon",
      "latin_name": "Acer palmatum",
      "cultivar": null,
      "category": "tree",
      "photo": null,
      "climate": {
        "min_temp_c": -18,
        "drought_tolerance": "low",
        "wind_tolerance": "low"
      },
      "exposure": {
        "ideal": [
          "partial_shade",
          "light_shade"
        ],
        "tolerated": [
          "full_sun"
        ]
      },
      "soil": {
        "types": [
          "loamy",
          "humus"
        ],
        "drainage": [
          "good",
          "moderate"
        ],
        "ph": [
          "acid",
          "neutral"
        ],
        "moisture": [
          "normal",
          "moist"
        ]
      },
      "dimensions": {
        "mature_height_m": 4.0,
        "mature_spread_m": 4.0,
        "planting_spacing_m": null,
        "growth_rate": "slow",
        "root_system": "non_invasive"
      },
      "characteristics": {
        "foliage": "deciduous",
        "uses": [
          "ornamental"
        ],
        "flowering_months": [
          4,
          5
        ],
        "interest_seasons": [
          "spring",
          "autumn"
        ],
        "flower_color": "red_purple",
        "autumn_color": "crimson"
      },
      "care": {
        "maintenance_level": "medium",
        "water_need": "medium",
        "pruning_months": [
          2,
          3
        ],
        "non_toxic_pets": true,
        "has_thorns": false
      },
      "notes": "À abriter des vents desséchants et du soleil brûlant d'après-midi dans les régions chaudes.",
      "tags": [
        "graphique",
        "automne",
        "ombre_claire"
      ],
      "links": {
        "wikidata": null,
        "wikipedia_fr": null
      },
      "meta": {
        "source": "manual",
        "verified": true,
        "uncertain_fields": [],
        "field_sources": {},
        "updated_at": "2026-11-21"
      }
    },
    {
      "id": "malus_domestica",
      "common_name": "Pommier domestique",
      "latin_name": "Malus domestica",
      "cultivar": null,
      "category": "tree",
      "photo": null,
      "climate": {
        "min_temp_c": -25,
        "drought_tolerance": "medium",
        "wind_tolerance": "medium"
      },
      "exposure": {
        "ideal": [
          "full_sun"
        ],
        "tolerated": [
          "partial_shade"
        ]
      },
      "soil": {
        "types": [
          "loamy",
          "clay"
        ],
        "drainage": [
          "good",
          "moderate"
        ],
        "ph": [
          "neutral",
          "alkaline",
          "acid"
        ],
        "moisture": [
          "normal",
          "moist"
        ]
      },
      "dimensions": {
        "mature_height_m": 5.0,
        "mature_spread_m": 4.5,
        "planting_spacing_m": null,
        "growth_rate": "medium",
        "root_system": "non_invasive"
      },
      "characteristics": {
        "foliage": "deciduous",
        "uses": [
          "edible_fruit",
          "pollinator",
          "ornamental",
          "shade_provider"
        ],
        "flowering_months": [
          4,
          5
        ],
        "interest_seasons": [
          "spring",
          "summer",
          "autumn"
        ],
        "flower_color": "white_pink",
        "autumn_color": "yellow"
      },
      "care": {
        "maintenance_level": "medium",
        "water_need": "low",
        "pruning_months": [
          12,
          1,
          2
        ],
        "non_toxic_pets": false,
        "has_thorns": false
      },
      "notes": "Vérifier la pollinisation croisée selon la variété choisie ; pépins toxiques pour certains animaux.",
      "tags": [
        "verger",
        "fruitier",
        "pollinisateurs"
      ],
      "links": {
        "wikidata": null,
        "wikipedia_fr": null
      },
      "meta": {
        "source": "manual",
        "verified": true,
        "uncertain_fields": [],
        "field_sources": {},
        "updated_at": "2026-11-21"
      }
    },
    {
      "id": "prunus_lusitanica",
      "common_name": "Laurier du Portugal",
      "latin_name": "Prunus lusitanica",
      "cultivar": null,
      "category": "shrub",
      "photo": null,
      "climate": {
        "min_temp_c": -15,
        "drought_tolerance": "medium",
        "wind_tolerance": "high"
      },
      "exposure": {
        "ideal": [
          "full_sun",
          "partial_shade"
        ],
        "tolerated": [
          "light_shade"
        ]
      },
      "soil": {
        "types": [
          "loamy",
          "clay",
          "chalky",
          "sandy"
        ],
        "drainage": [
          "good",
          "moderate"
        ],
        "ph": [
          "neutral",
          "alkaline",
          "acid"
        ],
        "moisture": [
          "normal"
        ]
      },
      "dimensions": {
        "mature_height_m": 4.0,
        "mature_spread_m": 3.0,
        "planting_spacing_m": 0.8,
        "growth_rate": "medium",
        "root_system": "non_invasive"
      },
      "characteristics": {
        "foliage": "evergreen",
        "uses": [
          "hedge",
          "ornamental",
          "pollinator"
        ],
        "flowering_months": [
          6
        ],
        "interest_seasons": [
          "spring",
          "summer",
          "autumn",
          "winter"
        ],
        "flower_color": "white",
        "autumn_color": null
      },
      "care": {
        "maintenance_level": "low",
        "water_need": "low",
        "pruning_months": [
          8,
          9
        ],
        "non_toxic_pets": false,
        "has_thorns": false
      },
      "notes": "Excellente alternative persistante aux haies classiques, supporte bien la taille.",
      "tags": [
        "haie",
        "persistant",
        "brise_vue"
      ],
      "links": {
        "wikidata": null,
        "wikipedia_fr": null
      },
      "meta": {
        "source": "manual",
        "verified": true,
        "uncertain_fields": [],
        "field_sources": {},
        "updated_at": "2026-11-21"
      }
    },
    {
      "id": "hydrangea_quercifolia",
      "common_name": "Hortensia à feuilles de chêne",
      "latin_name": "Hydrangea quercifolia",
      "cultivar": null,
      "category": "shrub",
      "photo": null,
      "climate": {
        "min_temp_c": -20,
        "drought_tolerance": "low",
        "wind_tolerance": "medium"
      },
      "exposure": {
        "ideal": [
          "partial_shade",
          "light_shade"
        ],
        "tolerated": [
          "full_sun"
        ]
      },
      "soil": {
        "types": [
          "loamy",
          "humus"
        ],
        "drainage": [
          "good",
          "moderate"
        ],
        "ph": [
          "acid",
          "neutral"
        ],
        "moisture": [
          "normal",
          "moist"
        ]
      },
      "dimensions": {
        "mature_height_m": 1.8,
        "mature_spread_m": 2.0,
        "planting_spacing_m": null,
        "growth_rate": "medium",
        "root_system": "non_invasive"
      },
      "characteristics": {
        "foliage": "deciduous",
        "uses": [
          "ornamental"
        ],
        "flowering_months": [
          6,
          7,
          8,
          9
        ],
        "interest_seasons": [
          "summer",
          "autumn"
        ],
        "flower_color": "white",
        "autumn_color": "burgundy"
      },
      "care": {
        "maintenance_level": "low",
        "water_need": "medium",
        "pruning_months": [
          3
        ],
        "non_toxic_pets": false,
        "has_thorns": false
      },
      "notes": "Très décoratif en été et à l'automne avec son feuillage pourpre.",
      "tags": [
        "mi_ombre",
        "floraison_ete",
        "automne"
      ],
      "links": {
        "wikidata": null,
        "wikipedia_fr": null
      },
      "meta": {
        "source": "manual",
        "verified": true,
        "uncertain_fields": [],
        "field_sources": {},
        "updated_at": "2026-11-21"
      }
    },
    {
      "id": "cornus_mas",
      "common_name": "Cornouiller mâle",
      "latin_name": "Cornus mas",
      "cultivar": null,
      "category": "shrub",
      "photo": null,
      "climate": {
        "min_temp_c": -25,
        "drought_tolerance": "high",
        "wind_tolerance": "high"
      },
      "exposure": {
        "ideal": [
          "full_sun",
          "partial_shade"
        ],
        "tolerated": [
          "light_shade"
        ]
      },
      "soil": {
        "types": [
          "chalky",
          "loamy",
          "clay"
        ],
        "drainage": [
          "good",
          "moderate",
          "fast"
        ],
        "ph": [
          "alkaline",
          "neutral"
        ],
        "moisture": [
          "dry",
          "normal"
        ]
      },
      "dimensions": {
        "mature_height_m": 3.5,
        "mature_spread_m": 3.0,
        "planting_spacing_m": null,
        "growth_rate": "slow",
        "root_system": "non_invasive"
      },
      "characteristics": {
        "foliage": "deciduous",
        "uses": [
          "ornamental",
          "pollinator",
          "edible_fruit",
          "hedge"
        ],
        "flowering_months": [
          2,
          3
        ],
        "interest_seasons": [
          "winter",
          "spring",
          "summer"
        ],
        "flower_color": "yellow",
        "autumn_color": "red_purple"
      },
      "care": {
        "maintenance_level": "very_low",
        "water_need": "very_low",
        "pruning_months": [
          2,
          3
        ],
        "non_toxic_pets": true,
        "has_thorns": false
      },
      "notes": "Floraison jaune très précoce dès la fin de l'hiver, précieux pour les premiers pollinisateurs.",
      "tags": [
        "calcaire",
        "floraison_hiver",
        "rustique"
      ],
      "links": {
        "wikidata": null,
        "wikipedia_fr": null
      },
      "meta": {
        "source": "manual",
        "verified": true,
        "uncertain_fields": [],
        "field_sources": {},
        "updated_at": "2026-11-21"
      }
    },
    {
      "id": "lavandula_angustifolia",
      "common_name": "Lavande vraie",
      "latin_name": "Lavandula angustifolia",
      "cultivar": null,
      "category": "perennial",
      "photo": null,
      "climate": {
        "min_temp_c": -15,
        "drought_tolerance": "high",
        "wind_tolerance": "medium"
      },
      "exposure": {
        "ideal": [
          "full_sun"
        ],
        "tolerated": []
      },
      "soil": {
        "types": [
          "sandy",
          "loamy",
          "chalky"
        ],
        "drainage": [
          "fast",
          "good"
        ],
        "ph": [
          "neutral",
          "alkaline"
        ],
        "moisture": [
          "dry",
          "normal"
        ]
      },
      "dimensions": {
        "mature_height_m": 0.8,
        "mature_spread_m": 0.9,
        "planting_spacing_m": 0.5,
        "growth_rate": "medium",
        "root_system": "non_invasive"
      },
      "characteristics": {
        "foliage": "evergreen",
        "uses": [
          "ornamental",
          "pollinator",
          "aromatic"
        ],
        "flowering_months": [
          6,
          7,
          8
        ],
        "interest_seasons": [
          "summer"
        ],
        "flower_color": "violet",
        "autumn_color": null
      },
      "care": {
        "maintenance_level": "low",
        "water_need": "very_low",
        "pruning_months": [
          8,
          9
        ],
        "non_toxic_pets": false,
        "has_thorns": false
      },
      "notes": "Redoute les sols lourds détrempés en hiver ; tailler en boule sans entrer dans le vieux bois.",
      "tags": [
        "soleil",
        "sec",
        "pollinisateurs",
        "aromatique"
      ],
      "links": {
        "wikidata": null,
        "wikipedia_fr": null
      },
      "meta": {
        "source": "manual",
        "verified": true,
        "uncertain_fields": [],
        "field_sources": {},
        "updated_at": "2026-11-21"
      }
    },
    {
      "id": "echinacea_purpurea",
      "common_name": "Échinacée pourpre",
      "latin_name": "Echinacea purpurea",
      "cultivar": null,
      "category": "perennial",
      "photo": null,
      "climate": {
        "min_temp_c": -20,
        "drought_tolerance": "medium",
        "wind_tolerance": "medium"
      },
      "exposure": {
        "ideal": [
          "full_sun"
        ],
        "tolerated": [
          "partial_shade"
        ]
      },
      "soil": {
        "types": [
          "loamy",
          "sandy",
          "clay"
        ],
        "drainage": [
          "good",
          "moderate"
        ],
        "ph": [
          "neutral",
          "alkaline",
          "acid"
        ],
        "moisture": [
          "normal",
          "dry"
        ]
      },
      "dimensions": {
        "mature_height_m": 0.9,
        "mature_spread_m": 0.6,
        "planting_spacing_m": 0.45,
        "growth_rate": "medium",
        "root_system": "non_invasive"
      },
      "characteristics": {
        "foliage": "deciduous",
        "uses": [
          "ornamental",
          "pollinator"
        ],
        "flowering_months": [
          7,
          8,
          9
        ],
        "interest_seasons": [
          "summer",
          "autumn"
        ],
        "flower_color": "pink_purple",
        "autumn_color": null
      },
      "care": {
        "maintenance_level": "low",
        "water_need": "low",
        "pruning_months": [
          3,
          4,
          5
        ],
        "non_toxic_pets": true,
        "has_thorns": false
      },
      "notes": "Très attractive pour les abeilles et papillons ; laisser les capitules secs en hiver pour les oiseaux.",
      "tags": [
        "vivace",
        "pollinisateurs",
        "ete"
      ],
      "links": {
        "wikidata": null,
        "wikipedia_fr": null
      },
      "meta": {
        "source": "manual",
        "verified": true,
        "uncertain_fields": [],
        "field_sources": {},
        "updated_at": "2026-11-21"
      }
    },
    {
      "id": "geranium_macrorrhizum",
      "common_name": "Géranium vivace à grosses racines",
      "latin_name": "Geranium macrorrhizum",
      "cultivar": null,
      "category": "groundcover",
      "photo": null,
      "climate": {
        "min_temp_c": -23,
        "drought_tolerance": "high",
        "wind_tolerance": "high"
      },
      "exposure": {
        "ideal": [
          "partial_shade",
          "light_shade"
        ],
        "tolerated": [
          "full_sun",
          "dense_shade"
        ]
      },
      "soil": {
        "types": [
          "loamy",
          "clay",
          "sandy",
          "chalky"
        ],
        "drainage": [
          "fast",
          "good",
          "moderate"
        ],
        "ph": [
          "acid",
          "neutral",
          "alkaline"
        ],
        "moisture": [
          "dry",
          "normal"
        ]
      },
      "dimensions": {
        "mature_height_m": 0.4,
        "mature_spread_m": 0.7,
        "planting_spacing_m": 0.35,
        "growth_rate": "fast",
        "root_system": "spreading"
      },
      "characteristics": {
        "foliage": "semi_evergreen",
        "uses": [
          "groundcover",
          "ornamental",
          "pollinator",
          "aromatic"
        ],
        "flowering_months": [
          5,
          6
        ],
        "interest_seasons": [
          "spring",
          "summer",
          "autumn"
        ],
        "flower_color": "pink_magenta",
        "autumn_color": "red_orange"
      },
      "care": {
        "maintenance_level": "very_low",
        "water_need": "very_low",
        "pruning_months": [
          3
        ],
        "non_toxic_pets": true,
        "has_thorns": false
      },
      "notes": "Couvre-sol très fiable qui limite les adventices et supporte l'ombre et la sécheresse.",
      "tags": [
        "couvre_sol",
        "sans_entretien",
        "ombre"
      ],
      "links": {
        "wikidata": null,
        "wikipedia_fr": null
      },
      "meta": {
        "source": "manual",
        "verified": true,
        "uncertain_fields": [],
        "field_sources": {},
        "updated_at": "2026-11-21"
      }
    },
    {
      "id": "hedera_helix",
      "common_name": "Lierre grimpant",
      "latin_name": "Hedera helix",
      "cultivar": null,
      "category": "climber",
      "photo": null,
      "climate": {
        "min_temp_c": -20,
        "drought_tolerance": "medium",
        "wind_tolerance": "high"
      },
      "exposure": {
        "ideal": [
          "partial_shade",
          "light_shade",
          "dense_shade"
        ],
        "tolerated": [
          "full_sun"
        ]
      },
      "soil": {
        "types": [
          "loamy",
          "clay",
          "chalky",
          "sandy"
        ],
        "drainage": [
          "good",
          "moderate",
          "poor"
        ],
        "ph": [
          "acid",
          "neutral",
          "alkaline"
        ],
        "moisture": [
          "normal",
          "moist",
          "dry"
        ]
      },
      "dimensions": {
        "mature_height_m": 8.0,
        "mature_spread_m": 4.0,
        "planting_spacing_m": null,
        "growth_rate": "fast",
        "root_system": "clinging"
      },
      "characteristics": {
        "foliage": "evergreen",
        "uses": [
          "ornamental",
          "groundcover",
          "pollinator"
        ],
        "flowering_months": [
          9,
          10
        ],
        "interest_seasons": [
          "autumn",
          "winter",
          "spring",
          "summer"
        ],
        "flower_color": "green_yellow",
        "autumn_color": null
      },
      "care": {
        "maintenance_level": "medium",
        "water_need": "very_low",
        "pruning_months": [
          3,
          4,
          5
        ],
        "non_toxic_pets": false,
        "has_thorns": false
      },
      "notes": "À maîtriser par une taille annuelle pour éviter qu'il n'envahisse les façades ou toitures.",
      "tags": [
        "grimpante",
        "persistant",
        "ombre_dense"
      ],
      "links": {
        "wikidata": null,
        "wikipedia_fr": null
      },
      "meta": {
        "source": "manual",
        "verified": true,
        "uncertain_fields": [],
        "field_sources": {},
        "updated_at": "2026-11-21"
      }
    },
    {
      "id": "clematis_montana",
      "common_name": "Clématite des montagnes",
      "latin_name": "Clematis montana",
      "cultivar": null,
      "category": "climber",
      "photo": null,
      "climate": {
        "min_temp_c": -18,
        "drought_tolerance": "low",
        "wind_tolerance": "medium"
      },
      "exposure": {
        "ideal": [
          "full_sun",
          "partial_shade"
        ],
        "tolerated": [
          "light_shade"
        ]
      },
      "soil": {
        "types": [
          "loamy",
          "humus"
        ],
        "drainage": [
          "good",
          "moderate"
        ],
        "ph": [
          "neutral",
          "alkaline"
        ],
        "moisture": [
          "normal",
          "moist"
        ]
      },
      "dimensions": {
        "mature_height_m": 6.0,
        "mature_spread_m": 3.5,
        "planting_spacing_m": null,
        "growth_rate": "fast",
        "root_system": "non_invasive"
      },
      "characteristics": {
        "foliage": "deciduous",
        "uses": [
          "ornamental",
          "pollinator"
        ],
        "flowering_months": [
          5,
          6
        ],
        "interest_seasons": [
          "spring",
          "summer"
        ],
        "flower_color": "pink_white",
        "autumn_color": null
      },
      "care": {
        "maintenance_level": "medium",
        "water_need": "medium",
        "pruning_months": [
          6
        ],
        "non_toxic_pets": false,
        "has_thorns": false
      },
      "notes": "Garder le pied à l'ombre et au frais, tout en laissant la végétation grimper vers la lumière.",
      "tags": [
        "grimpante",
        "floraison_printemps",
        "cloture"
      ],
      "links": {
        "wikidata": null,
        "wikipedia_fr": null
      },
      "meta": {
        "source": "manual",
        "verified": true,
        "uncertain_fields": [],
        "field_sources": {},
        "updated_at": "2026-11-21"
      }
    },
    {
      "id": "jasminum_nudiflorum",
      "common_name": "Jasmin d'hiver",
      "latin_name": "Jasminum nudiflorum",
      "cultivar": null,
      "category": "shrub",
      "photo": null,
      "climate": {
        "min_temp_c": -15,
        "drought_tolerance": "medium",
        "wind_tolerance": "medium"
      },
      "exposure": {
        "ideal": [
          "full_sun",
          "partial_shade"
        ],
        "tolerated": [
          "light_shade"
        ]
      },
      "soil": {
        "types": [
          "loamy",
          "clay",
          "sandy",
          "chalky"
        ],
        "drainage": [
          "good",
          "moderate",
          "fast"
        ],
        "ph": [
          "neutral",
          "alkaline",
          "acid"
        ],
        "moisture": [
          "dry",
          "normal"
        ]
      },
      "dimensions": {
        "mature_height_m": 3.0,
        "mature_spread_m": 2.5,
        "planting_spacing_m": null,
        "growth_rate": "medium",
        "root_system": "non_invasive"
      },
      "characteristics": {
        "foliage": "deciduous",
        "uses": [
          "ornamental",
          "groundcover"
        ],
        "flowering_months": [
          12,
          1,
          2,
          3
        ],
        "interest_seasons": [
          "winter",
          "spring"
        ],
        "flower_color": "yellow",
        "autumn_color": null
      },
      "care": {
        "maintenance_level": "low",
        "water_need": "low",
        "pruning_months": [
          3,
          4,
          5
        ],
        "non_toxic_pets": true,
        "has_thorns": false
      },
      "notes": "Arbuste sarmenteux (ne grimpe pas seul : à palisser). Rameaux verts et fleurs jaunes en plein hiver ; peut aussi s'utiliser en retombant ou couvre-sol.",
      "tags": [
        "hiver",
        "grimpante",
        "facile"
      ],
      "links": {
        "wikidata": null,
        "wikipedia_fr": null
      },
      "meta": {
        "source": "manual",
        "verified": true,
        "uncertain_fields": [],
        "field_sources": {},
        "updated_at": "2026-11-21"
      }
    },
    {
      "id": "salvia_officinalis",
      "common_name": "Sauge officinale",
      "latin_name": "Salvia officinalis",
      "cultivar": null,
      "category": "vegetable_herb",
      "photo": null,
      "climate": {
        "min_temp_c": -15,
        "drought_tolerance": "high",
        "wind_tolerance": "medium"
      },
      "exposure": {
        "ideal": [
          "full_sun"
        ],
        "tolerated": [
          "partial_shade"
        ]
      },
      "soil": {
        "types": [
          "sandy",
          "loamy",
          "chalky"
        ],
        "drainage": [
          "fast",
          "good"
        ],
        "ph": [
          "neutral",
          "alkaline"
        ],
        "moisture": [
          "dry",
          "normal"
        ]
      },
      "dimensions": {
        "mature_height_m": 0.7,
        "mature_spread_m": 0.8,
        "planting_spacing_m": 0.5,
        "growth_rate": "medium",
        "root_system": "non_invasive"
      },
      "characteristics": {
        "foliage": "evergreen",
        "uses": [
          "aromatic",
          "pollinator",
          "ornamental",
          "edible_leaf"
        ],
        "flowering_months": [
          5,
          6,
          7
        ],
        "interest_seasons": [
          "spring",
          "summer"
        ],
        "flower_color": "violet_blue",
        "autumn_color": null
      },
      "care": {
        "maintenance_level": "low",
        "water_need": "very_low",
        "pruning_months": [
          3,
          4,
          5
        ],
        "non_toxic_pets": null,
        "has_thorns": false
      },
      "notes": "Feuillage aromatique persistant ; préfère un sol drainant et peu humide en hiver.",
      "tags": [
        "aromate",
        "sec",
        "pollinisateurs"
      ],
      "links": {
        "wikidata": null,
        "wikipedia_fr": null
      },
      "meta": {
        "source": "manual",
        "verified": false,
        "uncertain_fields": [
          "care.non_toxic_pets"
        ],
        "field_sources": {},
        "updated_at": "2026-11-21"
      }
    },
    {
      "id": "rubus_fruticosus_thornless",
      "common_name": "Mûrier sans épines",
      "latin_name": "Rubus fruticosus",
      "cultivar": null,
      "category": "shrub",
      "photo": null,
      "climate": {
        "min_temp_c": -20,
        "drought_tolerance": "medium",
        "wind_tolerance": "medium"
      },
      "exposure": {
        "ideal": [
          "full_sun",
          "partial_shade"
        ],
        "tolerated": []
      },
      "soil": {
        "types": [
          "loamy",
          "clay",
          "humus"
        ],
        "drainage": [
          "good",
          "moderate"
        ],
        "ph": [
          "acid",
          "neutral"
        ],
        "moisture": [
          "normal",
          "moist"
        ]
      },
      "dimensions": {
        "mature_height_m": 2.2,
        "mature_spread_m": 2.5,
        "planting_spacing_m": null,
        "growth_rate": "fast",
        "root_system": "spreading"
      },
      "characteristics": {
        "foliage": "deciduous",
        "uses": [
          "edible_fruit",
          "pollinator"
        ],
        "flowering_months": [
          6,
          7
        ],
        "interest_seasons": [
          "summer",
          "autumn"
        ],
        "flower_color": "white",
        "autumn_color": null
      },
      "care": {
        "maintenance_level": "medium",
        "water_need": "medium",
        "pruning_months": [
          2,
          3
        ],
        "non_toxic_pets": true,
        "has_thorns": false
      },
      "notes": "Choisir un cultivar sans épines (ex. 'Loch Ness'). À palisser ou contenir pour garder une pousse ordonnée ; récolte estivale généreuse.",
      "tags": [
        "fruitier",
        "ete",
        "sans_epines"
      ],
      "links": {
        "wikidata": null,
        "wikipedia_fr": null
      },
      "meta": {
        "source": "manual",
        "verified": true,
        "uncertain_fields": [],
        "field_sources": {},
        "updated_at": "2026-11-21"
      }
    },
    {
      "id": "betula_pendula",
      "common_name": "Bouleau verruqueux",
      "latin_name": "Betula pendula",
      "cultivar": null,
      "category": "tree",
      "photo": null,
      "climate": {
        "min_temp_c": -30,
        "drought_tolerance": "medium",
        "wind_tolerance": "high"
      },
      "exposure": {
        "ideal": [
          "full_sun"
        ],
        "tolerated": [
          "partial_shade"
        ]
      },
      "soil": {
        "types": [
          "sandy",
          "loamy",
          "humus"
        ],
        "drainage": [
          "good",
          "fast",
          "moderate"
        ],
        "ph": [
          "acid",
          "neutral"
        ],
        "moisture": [
          "normal",
          "dry"
        ]
      },
      "dimensions": {
        "mature_height_m": 15.0,
        "mature_spread_m": 5.0,
        "planting_spacing_m": null,
        "growth_rate": "fast",
        "root_system": "non_invasive"
      },
      "characteristics": {
        "foliage": "deciduous",
        "uses": [
          "ornamental",
          "shade_provider"
        ],
        "flowering_months": [
          4
        ],
        "interest_seasons": [
          "spring",
          "autumn",
          "winter"
        ],
        "flower_color": "yellow_brown",
        "autumn_color": "golden_yellow"
      },
      "care": {
        "maintenance_level": "low",
        "water_need": "very_low",
        "pruning_months": [
          11
        ],
        "non_toxic_pets": true,
        "has_thorns": false
      },
      "notes": "Écorce blanche décorative et ombre légère laissant passer la lumière ; prévoir de la place en hauteur.",
      "tags": [
        "arbre",
        "rustique",
        "ombre_claire",
        "pollen_allergisant"
      ],
      "links": {
        "wikidata": null,
        "wikipedia_fr": null
      },
      "meta": {
        "source": "manual",
        "verified": true,
        "uncertain_fields": [],
        "field_sources": {},
        "updated_at": "2026-11-21"
      }
    }
  ]
}
```
