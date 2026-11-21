# Licences

## Code source

PlotFlora — Copyright © 2026 ChatCarbo

Ce programme est un logiciel libre : vous pouvez le redistribuer et/ou le modifier selon les termes
de la **GNU General Public License** publiée par la Free Software Foundation, soit la version 3 de
la licence, soit (à votre choix) toute version ultérieure (`GPL-3.0-or-later`).

Ce programme est distribué dans l'espoir qu'il sera utile, mais **sans aucune garantie** ; sans même
la garantie implicite de qualité marchande ou d'adéquation à un usage particulier. Voir le texte
complet de la licence dans le fichier `LICENSE`.

Le logo et l'icône (`assets/icon.*`, `assets/logo*`) sont distribués sous la même licence.

## Les données gardent leurs propres licences

La licence GPL s'applique au code de PlotFlora. **Elle ne s'applique pas aux données de tiers**
qu'il utilise ou télécharge, qui restent soumises à leurs propres licences :

| Données | Licence | Obligation |
| :--- | :--- | :--- |
| Orthophotos IGN – BD ORTHO (importées par l'utilisateur) | Licence Ouverte Etalab 2.0 | Citer « © IGN – BD ORTHO » (champ Attribution du projet) |
| Wikidata (identité des taxons, noms communs) | CC0 1.0 | Aucune |
| `assets/eive_index.json`, dérivé de Tichý L. et al. (2023), *Ellenberg-type indicator values for European vascular plant species*, Journal of Vegetation Science 34 : e13168, doi:10.5281/zenodo.7427088 | CC BY 4.0 | Citer la source (fait dans « À propos & licences ») |
| Photos Wikimedia Commons (téléchargées à la demande) | Licence propre à chaque fichier | Auteur et licence conservés dans la fiche et affichés dans « À propos & licences » |
| Wikipédia FR | CC BY-SA 4.0 | Seules des valeurs factuelles sont extraites ; aucun texte n'est stocké ni redistribué |
| Images, photos et projets créés par l'utilisateur | Celle choisie par l'utilisateur | — |

Les fiches du catalogue de démarrage saisies à la main (`assets/plants_catalog.seed.json`) sont
distribuées avec le code, sous GPL-3.0-or-later.

## Dépendances

Les bibliothèques Rust utilisées (eframe/egui, serde, ureq, rfd, image, calamine, winresource…)
sont publiées sous licences MIT et/ou Apache-2.0, compatibles avec la GPL version 3.
