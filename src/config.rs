use std::fmt;

use hashbrown::HashMap;
use owo_colors::{colored::Color as OwoColor, AnsiColors, FgDynColorDisplay, OwoColorize};
use serde::{Deserialize, Serialize};

use crate::Formula;

// We want to be able to upgrade configuration from at least one version ago, so we'll do
// initial deserialization as UpgradeConfig and convert from Upgrade into whatever the most
// current config happens to be at the time.

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum UpgradeConfig {
    Current(Config),
    Old(HashMap<String, Formula>),
}

impl UpgradeConfig {
    pub fn is_legacy(&self) -> bool {
        match self {
            UpgradeConfig::Current(_) => false,
            UpgradeConfig::Old(_) => true,
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    colors: Option<Colors>,
    formulas: HashMap<String, Formula>,
}

impl Config {
    #[inline]
    pub fn get_alias(&self, name: &str) -> Option<&Formula> {
        self.formulas.get(name)
    }

    #[inline]
    pub fn set_alias(&mut self, name: String, formula: Formula) -> Option<Formula> {
        self.formulas.insert(name, formula)
    }

    #[inline]
    pub fn remove(&mut self, name: &str) -> Option<Formula> {
        self.formulas.remove(name)
    }

    pub fn aliases(&self) -> impl Iterator<Item = (&String, &Formula)> {
        self.formulas.iter()
    }

    pub fn colors(&self) -> Colors {
        self.colors.unwrap_or_default()
    }
}

impl From<UpgradeConfig> for Config {
    fn from(config: UpgradeConfig) -> Self {
        match config {
            UpgradeConfig::Current(current) => current,
            UpgradeConfig::Old(formulas) => Config {
                formulas,
                ..Default::default()
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
pub struct Colors {
    #[serde(skip_serializing_if = "Option::is_none")]
    low: Option<Color>,
    #[serde(skip_serializing_if = "Option::is_none")]
    high: Option<Color>,
}

impl Colors {
    pub fn high<'a, T: fmt::Display>(&self, item: &'a T) -> FgDynColorDisplay<'a, AnsiColors, T> {
        match self.high {
            None => item.color(AnsiColors::BrightGreen),
            Some(color) => item.color(color.0),
        }
    }

    pub fn low<'a, T: fmt::Display>(&self, item: &'a T) -> FgDynColorDisplay<'a, AnsiColors, T> {
        match self.low {
            None => item.color(AnsiColors::BrightRed),
            Some(color) => item.color(color.0),
        }
    }
}

// Rather than waste a lot of time trying to convert my type into their type for choosing colors,
// I'm just going to wrap a newtype around their type and implement deserialize on that. The only
// reason this is needed is in order to support case-insensitive deserialization (since the
// serde-aux crate recommended for that purpose doesn't support enums).

// Serde may already treat enums as case insensitve. I don't know. If it does, don't tell me.

// Anyway, the easiest way to do this is to deserialize to a string slice, convert to lower case,
// and call it good.

#[derive(Clone, Copy, Debug)]
pub struct Color(OwoColor);

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data: &str = Deserialize::deserialize(deserializer)?;
        let data = data.to_ascii_uppercase();

        let result = match &*data {
            "BLACK" => OwoColor::Black,
            "RED" => OwoColor::Red,
            "GREEN" => OwoColor::Green,
            "YELLOW" => OwoColor::Yellow,
            "BLUE" => OwoColor::Blue,
            "MAGENTA" => OwoColor::Magenta,
            "CYAN" => OwoColor::Cyan,
            "WHITE" => OwoColor::White,
            "DEFAULT" => OwoColor::Default,
            "BRIGHTBLACK" => OwoColor::BrightBlack,
            "BRIGHTRED" => OwoColor::BrightRed,
            "BRIGHTGREEN" => OwoColor::BrightGreen,
            "BRIGHTYELLOW" => OwoColor::BrightYellow,
            "BRIGHTBLUE" => OwoColor::BrightBlue,
            "BRIGHTMAGENTA" => OwoColor::BrightMagenta,
            "BRIGHTCYAN" => OwoColor::BrightCyan,
            "BRIGHTWHITE" => OwoColor::BrightWhite,

            _ => return Err(serde::de::Error::custom("unknown color")),
        };

        Ok(Color(result))
    }
}

// Since I implemented a custom DE-serializer for Color, I also need a custom serializer.
// Luckily, for this kind of type, serialization is really about as easy as deserialization.
//
//      "Easiest money you'll ever make."
//                      - Peoples

// I'm a fucking comment artiste. The E is silent.

impl Serialize for Color {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let color = match self.0 {
            OwoColor::Black => "Black",
            OwoColor::Red => "Red",
            OwoColor::Green => "Green",
            OwoColor::Yellow => "Yellow",
            OwoColor::Blue => "Blue",
            OwoColor::Magenta => "Magenta",
            OwoColor::Cyan => "Cyan",
            OwoColor::White => "White",
            OwoColor::Default => "Default",
            OwoColor::BrightBlack => "BrightBlack",
            OwoColor::BrightRed => "BrightRed",
            OwoColor::BrightGreen => "BrightGreen",
            OwoColor::BrightYellow => "BrightYellow",
            OwoColor::BrightBlue => "BrightBlue",
            OwoColor::BrightMagenta => "BrightMagenta",
            OwoColor::BrightCyan => "BrightCyan",
            OwoColor::BrightWhite => "BrightWhite",
        };

        color.serialize(serializer)
    }
}
