// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 ChatCarbo

//! Vocabulaire contrôlé (§6.5). Chaque énumération est sérialisée en `snake_case`
//! et porte un libellé français. Le premier variant est la valeur par défaut ;
//! l'ordre de déclaration définit le rang (`rank`).

use std::fmt;

/// Valeur hors vocabulaire rencontrée lors d'une conversion texte → énumération.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownValue {
    pub kind: &'static str,
    pub value: String,
}

impl fmt::Display for UnknownValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "valeur « {} » inconnue pour {}", self.value, self.kind)
    }
}

impl std::error::Error for UnknownValue {}

macro_rules! vocab {
    ($(#[$m:meta])* $name:ident { $($var:ident = $s:literal : $fr:literal),+ $(,)? }) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum $name { $($var),+ }

        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$var),+];
            pub const fn as_str(self) -> &'static str { match self { $(Self::$var => $s),+ } }
            pub const fn label(self) -> &'static str { match self { $(Self::$var => $fr),+ } }
            pub const fn rank(self) -> u8 { self as u8 }
        }

        impl Default for $name {
            fn default() -> Self { Self::ALL[0] }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { f.write_str(self.label()) }
        }

        impl std::str::FromStr for $name {
            type Err = UnknownValue;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($s => Ok(Self::$var),)+
                    _ => Err(UnknownValue { kind: stringify!($name), value: s.to_owned() }),
                }
            }
        }

        #[cfg(feature = "serde")]
        impl serde::Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(self.as_str())
            }
        }

        #[cfg(feature = "serde")]
        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let s = String::deserialize(d)?;
                s.parse().map_err(serde::de::Error::custom)
            }
        }
    };
}

vocab!(Status { ToPlant = "to_plant": "À planter", Existing = "existing": "Existant", ToReplace = "to_replace": "À remplacer" });
vocab!(ZoneKind { Point = "point": "Plante isolée", Area = "area": "Massif", Line = "line": "Haie / bordure" });
vocab!(Sun {
    FullSun = "full_sun": "Plein soleil",
    PartialShade = "partial_shade": "Mi-ombre",
    LightShade = "light_shade": "Ombre légère",
    DenseShade = "dense_shade": "Ombre dense",
});
vocab!(Wind { Sheltered = "sheltered": "Abrité", Moderate = "moderate": "Modéré", Exposed = "exposed": "Exposé" });
vocab!(Moisture { Dry = "dry": "Sec", Normal = "normal": "Normal", Moist = "moist": "Frais", Wet = "wet": "Humide" });
vocab!(SoilType {
    Loamy = "loamy": "Limoneux",
    Clay = "clay": "Argileux",
    Sandy = "sandy": "Sableux",
    Chalky = "chalky": "Calcaire",
    Humus = "humus": "Humifère",
});
vocab!(Drainage { Fast = "fast": "Rapide", Good = "good": "Bon", Moderate = "moderate": "Moyen", Poor = "poor": "Mauvais" });
vocab!(Ph { Acid = "acid": "Acide", Neutral = "neutral": "Neutre", Alkaline = "alkaline": "Alcalin" });
vocab!(
    /// Échelle à trois niveaux : tolérances de la plante, risque de sécheresse, disponibilité d'arrosage.
    Level { Low = "low": "Faible", Medium = "medium": "Moyen", High = "high": "Élevé" }
);
vocab!(WaterNeed { VeryLow = "very_low": "Très faible", Low = "low": "Faible", Medium = "medium": "Moyen", High = "high": "Élevé" });
vocab!(Maintenance { VeryLow = "very_low": "Très faible", Low = "low": "Faible", Medium = "medium": "Moyen", High = "high": "Élevé" });
vocab!(MaxMaintenance {
    Any = "any": "Indifférent",
    VeryLow = "very_low": "Très faible",
    Low = "low": "Faible",
    Medium = "medium": "Moyen",
    High = "high": "Élevé",
});
vocab!(Foliage { Evergreen = "evergreen": "Persistant", SemiEvergreen = "semi_evergreen": "Semi-persistant", Deciduous = "deciduous": "Caduc" });
vocab!(FoliagePref { Any = "any": "Indifférent", Evergreen = "evergreen": "Persistant", Deciduous = "deciduous": "Caduc" });
vocab!(Use {
    Ornamental = "ornamental": "Ornemental",
    EdibleFruit = "edible_fruit": "Fruits comestibles",
    EdibleLeaf = "edible_leaf": "Feuilles comestibles",
    Aromatic = "aromatic": "Aromatique",
    Pollinator = "pollinator": "Mellifère",
    Hedge = "hedge": "Haie",
    Groundcover = "groundcover": "Couvre-sol",
    ShadeProvider = "shade_provider": "Ombrage",
});
vocab!(Category {
    Tree = "tree": "Arbre",
    Shrub = "shrub": "Arbuste",
    Perennial = "perennial": "Vivace",
    Climber = "climber": "Grimpante",
    Groundcover = "groundcover": "Couvre-sol",
    VegetableHerb = "vegetable_herb": "Potager / aromatique",
});
vocab!(GrowthRate { Slow = "slow": "Lente", Medium = "medium": "Moyenne", Fast = "fast": "Rapide" });
vocab!(RootSystem {
    NonInvasive = "non_invasive": "Non envahissant",
    Spreading = "spreading": "Traçant",
    Suckering = "suckering": "Drageonnant",
    Clinging = "clinging": "Crampons",
});
vocab!(Season { Spring = "spring": "Printemps", Summer = "summer": "Été", Autumn = "autumn": "Automne", Winter = "winter": "Hiver" });
vocab!(ObstacleKind {
    House = "house": "Maison",
    Shed = "shed": "Abri",
    Terrace = "terrace": "Terrasse",
    ExistingTree = "existing_tree": "Arbre existant",
    Wall = "wall": "Mur",
    Other = "other": "Autre",
});
vocab!(MapSource { Image = "image": "Image aérienne", Drawing = "drawing": "Plan dessiné" });
vocab!(
    /// Ambiance (style) de jardin, pour orienter les conseils.
    Ambiance {
        Mediterranean = "mediterranean": "Méditerranéen",
        Japanese = "japanese": "Japonais / zen",
        Cottage = "cottage": "Champêtre / cottage",
        Contemporary = "contemporary": "Contemporain / graphique",
        Woodland = "woodland": "Sous-bois / ombre",
        Edible = "edible": "Nourricier / potager",
        Wildlife = "wildlife": "Sauvage / biodiversité",
        Exotic = "exotic": "Exotique",
    }
);
vocab!(DataSource {
    Manual = "manual": "Saisie manuelle",
    Seed = "seed": "Catalogue embarqué",
    Wikidata = "wikidata": "Wikidata",
    WikipediaFr = "wikipedia_fr": "Wikipédia",
    Eive = "eive": "EIVE",
    Mixed = "mixed": "Sources multiples",
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_rank() {
        for s in Sun::ALL {
            assert_eq!(s.as_str().parse::<Sun>().unwrap(), *s);
        }
        assert!("unknown".parse::<Sun>().is_err());
        assert_eq!(WaterNeed::High.rank(), 3);
        assert_eq!(Status::default(), Status::ToPlant);
        assert_eq!(MaxMaintenance::default(), MaxMaintenance::Any);
    }
}
