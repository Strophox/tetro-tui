use std::collections::{BTreeMap, HashMap};
use std::num::NonZeroUsize;

use crossterm::style::Color;

use crate::core_game_engine::ExtNonNegF64;

use crate::settings::SlotMachine;

pub mod hard_drop_effect;
pub mod line_clear_effect;
pub mod lock_effect;
pub mod mini_tetromino_symbols;
pub mod small_tetromino_symbols;
pub mod tile_coloring;
pub mod tile_symbols;
pub mod tui_coloring;
pub mod tui_symbols;

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct GraphicsSettings {
    #[serde(rename = "tilecol_sel")]
    pub tile_coloring_selected: usize,
    #[serde(rename = "tuisymb_sel")]
    pub tui_symbols_selected: usize,
    #[serde(rename = "tuicol_sel")]
    pub tui_coloring_selected: usize,
    #[serde(rename = "tilesymb_sel")]
    pub tile_symbols_selected: usize,
    #[serde(rename = "harddrop_sel")]
    pub hard_drop_selected: usize,
    #[serde(rename = "lock_sel")]
    pub lock_effect_selected: usize,
    #[serde(rename = "lineclear_sel")]
    pub line_clear_selected: usize,
    #[serde(rename = "minitetsymb_sel")]
    pub mini_tetromino_symbols_selected: usize,
    #[serde(rename = "smalltetsymb_sel")]
    pub small_tetromino_symbols_selected: usize,
    #[serde(rename = "normsize_prev_limit")]
    pub normalsize_preview_limit: Option<NonZeroUsize>,
    #[serde(rename = "fps")]
    pub fps: ExtNonNegF64,
    #[serde(rename = "uniform_locked_tiles")]
    pub uniform_locked_tiles: bool,
    #[serde(rename = "s_hud")]
    pub show_main_hud: bool,
    #[serde(rename = "s_keybinds")]
    pub show_keybinds: bool,
    #[serde(rename = "s_buttons")]
    pub show_buttons: bool,
    #[serde(rename = "s_lockdelay")]
    pub show_lockdelay: bool,
    #[serde(rename = "s_shadow")]
    pub show_shadow: bool,
    #[serde(rename = "s_spawn")]
    pub show_spawn: bool,
    #[serde(rename = "s_grid")]
    pub show_grid: bool,
    #[serde(rename = "s_fps")]
    pub show_fps: bool,
}

pub fn graphics_settings_presets() -> SlotMachine<GraphicsSettings> {
    let slots = vec![
        ("Default".to_owned(), GraphicsSettings::default()),
        ("Guideline".to_owned(), GraphicsSettings::guideline()),
        ("Focus+".to_owned(), GraphicsSettings::extra_focus()),
        (
            "Terminal compatibility".to_owned(),
            GraphicsSettings::compatibility(),
        ),
        (
            "Elektronika 60".to_owned(),
            GraphicsSettings::elektronika_60(),
        ),
        ("I⠐⢷⠗ Braille".to_owned(), GraphicsSettings::braille()), // ⠺⡾⠂
        ("Minimal".to_owned(), GraphicsSettings::minimal()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "Graphics".to_owned())
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        GraphicsSettings {
            tui_coloring_selected: 0,            // No color
            tile_coloring_selected: 2,           // Okpalette
            tui_symbols_selected: 1,             // Unicode
            tile_symbols_selected: 1,            // Unicode
            hard_drop_selected: 1,               // ASCII particles
            lock_effect_selected: 3,             // Unicode pulse
            line_clear_selected: 11,             // Stardust
            mini_tetromino_symbols_selected: 1,  // Braille
            small_tetromino_symbols_selected: 1, // Blocks
            normalsize_preview_limit: Some(NonZeroUsize::MIN),
            fps: ExtNonNegF64::from(60),
            uniform_locked_tiles: false,
            show_main_hud: true,
            show_lockdelay: false,
            show_keybinds: true,
            show_buttons: false,
            show_shadow: true,
            show_spawn: true,
            show_grid: false,
            show_fps: false,
        }
    }
}

impl GraphicsSettings {
    pub fn extra_focus() -> Self {
        GraphicsSettings {
            tui_coloring_selected: 0,            // No color
            tile_coloring_selected: 3,           // Standard
            tui_symbols_selected: 4,             // Unicode
            tile_symbols_selected: 1,            // Unicode
            hard_drop_selected: 0,               // None
            lock_effect_selected: 0,             // None
            line_clear_selected: 0,              // None (retain)
            mini_tetromino_symbols_selected: 1,  // Braille
            small_tetromino_symbols_selected: 1, // Blocks
            normalsize_preview_limit: None,
            fps: ExtNonNegF64::from(60),
            uniform_locked_tiles: true,
            show_main_hud: true,
            show_lockdelay: true,
            show_keybinds: false,
            show_buttons: true,
            show_shadow: true,
            show_spawn: true,
            show_grid: false,
            show_fps: false,
        }
    }

    pub fn guideline() -> Self {
        GraphicsSettings {
            tui_coloring_selected: 0,            // No color
            tile_coloring_selected: 3,           // Standard
            tui_symbols_selected: 2,             // Rounded Unicode
            tile_symbols_selected: 1,            // Unicode
            hard_drop_selected: 4,               // Solid beam Unicode
            lock_effect_selected: 3,             // Pulse Unicode
            line_clear_selected: 6,              // Clear inward
            mini_tetromino_symbols_selected: 1,  // Braille
            small_tetromino_symbols_selected: 1, // Blocks
            normalsize_preview_limit: Some(NonZeroUsize::new(4).unwrap()),
            fps: ExtNonNegF64::from(60),
            uniform_locked_tiles: false,
            show_main_hud: true,
            show_lockdelay: false,
            show_keybinds: true,
            show_buttons: false,
            show_shadow: true,
            show_spawn: true,
            show_grid: false,
            show_fps: false,
        }
    }

    pub fn braille() -> Self {
        GraphicsSettings {
            tui_coloring_selected: 0,            // No color
            tile_coloring_selected: 3,           // Standard
            tui_symbols_selected: 5,             // Braille
            tile_symbols_selected: 2,            // Braille
            hard_drop_selected: 6,               // Braille helix
            lock_effect_selected: 4,             // Spiral Braille
            line_clear_selected: 14,             // Sparks Braille
            mini_tetromino_symbols_selected: 1,  // Braille
            small_tetromino_symbols_selected: 2, // Braille
            normalsize_preview_limit: Some(NonZeroUsize::new(4).unwrap()),
            fps: ExtNonNegF64::from(60),
            uniform_locked_tiles: false,
            show_main_hud: true,
            show_lockdelay: false,
            show_keybinds: true,
            show_buttons: false,
            show_shadow: true,
            show_spawn: true,
            show_grid: false,
            show_fps: false,
        }
    }

    pub fn compatibility() -> Self {
        GraphicsSettings {
            tui_coloring_selected: 0,            // No color
            tile_coloring_selected: 1,           // ANSI
            tui_symbols_selected: 0,             // ASCII
            tile_symbols_selected: 0,            // ASCII
            hard_drop_selected: 1,               // ASCII particles
            lock_effect_selected: 2,             // ASCII transform
            line_clear_selected: 15,             // Sparks ASCII
            mini_tetromino_symbols_selected: 0,  // Letters
            small_tetromino_symbols_selected: 0, // ASCII
            normalsize_preview_limit: None,
            fps: ExtNonNegF64::from(30),
            uniform_locked_tiles: false,
            show_main_hud: true,
            show_lockdelay: false,
            show_keybinds: true,
            show_buttons: false,
            show_shadow: true,
            show_spawn: true,
            show_grid: false,
            show_fps: false,
        }
    }

    pub fn elektronika_60() -> Self {
        GraphicsSettings {
            tui_coloring_selected: 0,            // No color
            tile_coloring_selected: 0,           // Monochrome
            tui_symbols_selected: 6,             // Elektronika 60
            tile_symbols_selected: 4,            // Elektronika 60
            hard_drop_selected: 0,               // None
            lock_effect_selected: 0,             // None
            line_clear_selected: 5,              // Clear left-to-right
            mini_tetromino_symbols_selected: 0,  // Letters
            small_tetromino_symbols_selected: 0, // ASCII
            normalsize_preview_limit: None,
            fps: ExtNonNegF64::from(60),
            uniform_locked_tiles: false,
            show_main_hud: true,
            show_lockdelay: false,
            show_keybinds: true,
            show_buttons: false,
            show_shadow: false,
            show_spawn: false,
            show_grid: true,
            show_fps: false,
        }
    }

    pub fn minimal() -> Self {
        GraphicsSettings {
            tui_coloring_selected: 0,            // No color
            tile_coloring_selected: 0,           // Monochrome
            tui_symbols_selected: 3,             // Borderless-Next/Hold Unicode
            tile_symbols_selected: 1,            // Unicode
            hard_drop_selected: 0,               // None
            lock_effect_selected: 0,             // None
            line_clear_selected: 0,              // None
            mini_tetromino_symbols_selected: 1,  // Braille
            small_tetromino_symbols_selected: 1, // Blocks
            normalsize_preview_limit: None,
            fps: ExtNonNegF64::from(60),
            uniform_locked_tiles: false,
            show_main_hud: false,
            show_lockdelay: false,
            show_keybinds: false,
            show_buttons: false,
            show_shadow: false,
            show_spawn: false,
            show_grid: true, // We make an exception here, because without anything it's hard to play but with grid this arguably already becomes a viable preset.
            show_fps: false,
        }
    }
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
#[serde(into = "String", try_from = "String")]
pub struct TileTexture(pub [char; 2]);

impl TileTexture {
    pub const SPACE: Self = TileTexture([' '; 2]);
}

impl TryFrom<String> for TileTexture {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let tile = value
            .chars()
            .collect::<Vec<char>>()
            .try_into()
            .map_err(|x| format!("Error: {x:?}"))?;
        Ok(TileTexture(tile))
    }
}

// -- Serialization boilerplate --

pub trait UnwrapTileFromStr {
    fn tile(&self) -> TileTexture;
}

impl UnwrapTileFromStr for str {
    fn tile(&self) -> TileTexture {
        let tile = self.chars().collect::<Vec<char>>().try_into().unwrap();
        TileTexture(tile)
    }
}

impl From<TileTexture> for String {
    fn from(value: TileTexture) -> Self {
        value.0.iter().collect()
    }
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum MaybeOverride<T> {
    Keep,
    Override(T),
}

impl<T> MaybeOverride<T> {
    pub fn unwrap_or(self, keep: T) -> T {
        match self {
            MaybeOverride::Keep => keep,
            MaybeOverride::Override(r#override) => r#override,
        }
    }
}

// -- ♪ Symphony of Serialization Boilerplate ♫ --

// FIXME: Refactor using  #[serde(try_from = "FromType")]  and  #[serde(into = "IntoType")]  ? (See <https://serde.rs/container-attrs.html>)

// From <https://stackoverflow.com/questions/42723065/how-to-sort-hashmap-keys-when-serializing-with-serde>
//
// For use with serde's [serialize_with] attribute
#[allow(unused)]
fn ordered_colormap<S: serde::Serializer>(
    value: &HashMap<u8, Color>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    #[serde_with::serde_as] // Do **NOT** place this after #[derive(..)] !!
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(transparent)]
    struct Wrapper(
        // #[serde_as(as = "Vec<(_, ColorSerializationType)>")]
        // #[serde_as(
        //     as = "std::collections::BTreeMap<serde_with::json::JsonString, ColorSerializationType>"
        // )]
        #[serde_as(as = "std::collections::BTreeMap<_, ColorSerializationType>")]
        BTreeMap<u8, Color>,
    );

    serde::Serialize::serialize(&Wrapper(value.clone().into_iter().collect()), serializer)
}

// FIXME: The following boilerplate is adapted from how Crossterm serializes its `Color`
// (<https://github.com/crossterm-rs/crossterm/blob/master/src/style/types/color.rs#L260>)
// and should maybe be changed or accredited better.
struct ColorSerializationType;
#[rustfmt::skip]
impl serde_with::SerializeAs<Color> for ColorSerializationType {
    fn serialize_as<S: serde::ser::Serializer>(c: &Color, s: S) -> Result<S::Ok, S::Error> {
        use Color as C;
        match *c {
            C::AnsiValue(value) => s.serialize_str(&format!("ansi_({})", value)),
            C::Rgb { r, g, b } => {
                s.serialize_str(&format!("#{r:02x}{g:02x}{b:02x}"))
            }
            c => {
                s.serialize_str(match c {
                    C::Reset => "reset",
                    C::Black => "black",
                    C::DarkGrey => "dark_grey",
                    C::Red => "red",
                    C::DarkRed => "dark_red",
                    C::Green => "green",
                    C::DarkGreen => "dark_green",
                    C::Yellow => "yellow",
                    C::DarkYellow => "dark_yellow",
                    C::Blue => "blue",
                    C::DarkBlue => "dark_blue",
                    C::Magenta => "magenta",
                    C::DarkMagenta => "dark_magenta",
                    C::Cyan => "cyan",
                    C::DarkCyan => "dark_cyan",
                    C::White => "white",
                    C::Grey => "grey",
                    _ => unreachable!(),
                })
            }
        }
    }
}

#[rustfmt::skip]
impl<'de> serde_with::DeserializeAs<'de, Color> for ColorSerializationType {
    fn deserialize_as<D: serde::de::Deserializer<'de>>(d: D) -> Result<Color, D::Error> {
        struct ColorVisitor;
        impl serde::de::Visitor<'_> for ColorVisitor {
            type Value = Color;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(
                    "`reset`, `black`, `blue`, `dark_blue`, `cyan`, `dark_cyan`, `green`, `dark_green`, `grey`, `dark_grey`, `magenta`, `dark_magenta`, `red`, `dark_red`, `white`, `yellow`, `dark_yellow`, `ansi_(value)`, or `#rgbhex`",
                )
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Color, E> {
                if let Ok(c) = Color::try_from(value) {
                    Ok(c)
                } else {
                    if value.contains("ansi") {
                        // strip away `ansi_(..)' and get the inner value between parenthesis.
                        let results = value.replace("ansi_(", "").replace(")", "");

                        let ansi_val = results.parse::<u8>();

                        if let Ok(ansi) = ansi_val {
                            return Ok(Color::AnsiValue(ansi));
                        }
                    } else if value.contains("rgb") {
                        // strip away `rgb_(..)' and get the inner values between parenthesis.
                        let results = value
                            .replace("rgb_(", "")
                            .replace(")", "")
                            .split(',')
                            .map(|x| x.to_string())
                            .collect::<Vec<String>>();

                        if results.len() == 3 {
                            let r = results[0].parse::<u8>();
                            let g = results[1].parse::<u8>();
                            let b = results[2].parse::<u8>();

                            if let (Ok(r), Ok(g), Ok(b)) = (r, g, b) {
                                return Ok(Color::Rgb { r, g, b });
                            }
                        }
                    } else if let Some(hex) = value.strip_prefix('#') && hex.is_ascii() && hex.len() == 6 {
                        let r = u8::from_str_radix(&hex[0..2], 16);
                        let g = u8::from_str_radix(&hex[2..4], 16);
                        let b = u8::from_str_radix(&hex[4..6], 16);

                        if let (Ok(r), Ok(g), Ok(b)) = (r, g, b) {
                            return Ok(Color::Rgb { r, g, b });
                        }
                    }

                    Err(E::invalid_value(serde::de::Unexpected::Str(value), &self))
                }
            }
        }

        d.deserialize_str(ColorVisitor)
    }
}
