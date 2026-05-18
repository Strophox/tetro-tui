use std::collections::{BTreeMap, HashMap};

use crossterm::style::Color;

use crate::{
    core_game_engine::{Tetromino, TileType},
    settings::SlotMachine,
};

pub type PaletteIdx = u8;

#[derive(PartialEq, Eq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TileColoring {
    pub tile_palette_idx: TilePaletteIndexing,
    pub palette: Palette,
}

pub fn tile_coloring_presets() -> SlotMachine<TileColoring> {
    // NOTE: The slot at index 0 is the special 'monochrome'/no color slot.
    let slots = vec![
        (
            "No color".to_owned(),
            TileColoring::simple(Palette::no_color()),
        ),
        ("ANSI".to_owned(), TileColoring::simple(Palette::ansi())),
        (
            "Tetro Pastel".to_owned(),
            TileColoring::simple(Palette::tetro_pastel()),
        ),
        (
            "Guideline".to_owned(),
            TileColoring::simple(Palette::guideline()),
        ),
        (
            "Gruvbox".to_owned(),
            TileColoring::simple(Palette::gruvbox()),
        ),
        (
            "Solarized".to_owned(),
            TileColoring::simple(Palette::solarized()),
        ),
        (
            "Terafox".to_owned(),
            TileColoring::simple(Palette::terafox()),
        ),
        (
            "Fahrenheit".to_owned(),
            TileColoring::simple(Palette::fahrenheit()),
        ),
        ("Matrix".to_owned(), TileColoring::simple(Palette::matrix())),
        (
            "Sequoia".to_owned(),
            TileColoring::simple(Palette::sequoia()),
        ),
        (
            "NES levels".to_owned(),
            TileColoring {
                tile_palette_idx: TilePaletteIndexing::HardcodedNES,
                palette: Palette::nes(),
            },
        ),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "Tile Coloring".to_owned())
}

impl TileColoring {
    pub fn simple(palette: Palette) -> TileColoring {
        TileColoring {
            tile_palette_idx: TilePaletteIndexing::FgSimple,
            palette,
        }
    }

    pub fn get(&self, tile: TileType, level: usize) -> (Color, Color) {
        let colormap = &self.palette.map;
        match &self.tile_palette_idx {
            TilePaletteIndexing::FgSimple => (
                *colormap.get(&u8::from(tile)).unwrap_or(&Color::Reset),
                Color::Reset,
            ),
            TilePaletteIndexing::FgBgSimple => (
                *colormap.get(&u8::from(tile)).unwrap_or(&Color::Reset),
                *colormap
                    .get(&(u8::from(tile) + TileType::VARIANTS.len() as u8))
                    .unwrap_or(&Color::Reset),
            ),
            TilePaletteIndexing::FgBgVariable(level_luts) => {
                let (fg_lut, bg_lut) = level_luts[level];
                (
                    *colormap
                        .get(&(fg_lut[usize::from(tile)]))
                        .unwrap_or(&Color::Reset),
                    *colormap
                        .get(&(bg_lut[usize::from(tile)]))
                        .unwrap_or(&Color::Reset),
                )
            }
            TilePaletteIndexing::HardcodedNES => todo!(),
        }
    }

    pub fn tetromino_rainbow(&self) -> [Color; Tetromino::VARIANTS.len()] {
        "1643502"
            .chars()
            .map(|ch| {
                self.get(
                    crate::core_game_engine::Tetromino::VARIANTS
                        [ch.to_string().parse::<usize>().unwrap()]
                    .into(),
                    0,
                )
                .0
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }
}

pub type TilePaletteLUT = [u8; TileType::VARIANTS.len()];

// See also https://en.wikipedia.org/wiki/Indexed_color
#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum TilePaletteIndexing {
    /// This maps Tetromino FG -> 0..=6, Generic FG -> 7.
    FgSimple,
    /// This maps Tetromino FG -> 0..=6, BG -> 8..=14, Generic FG -> 7, BG -> 15,
    FgBgSimple,
    /// Most general TileType-to-palette mapping, changes according to a game's `Configuration::update_delays_every_n_lineclears` field.
    FgBgVariable(Vec<(TilePaletteLUT, TilePaletteLUT)>),
    /// Hardcoded variant that acts like a `ChangingFGBG` but without cluttering the savefile with thousands of entries.
    HardcodedNES,
}

fn col_from_raw_rgb(rgb: u32) -> Color {
    Color::Rgb {
        r: ((rgb >> 16) & 0xFF) as u8,
        g: ((rgb >> 8) & 0xFF) as u8,
        b: (rgb & 0xFF) as u8,
    }
}

fn construct_simple_palette(raw_map: [(PaletteIdx, u32); 10]) -> Palette {
    Palette {
        map: HashMap::from_iter(
            raw_map
                .into_iter()
                .map(|(i, rgb)| (i, col_from_raw_rgb(rgb))),
        ),
    }
}

#[derive(PartialEq, Eq, Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Palette {
    #[serde(serialize_with = "ordered_colormap")]
    pub map: HashMap<u8, Color>,
    // TODO: Add special fields for white/black to FIX nes b/w color indices? (but all the other ones are the same again...)
}

impl Palette {
    pub const BLACK: u8 = 248;
    pub const WHITE: u8 = 255;

    pub fn no_color() -> Palette {
        Palette {
            map: Default::default(),
        }
    }

    pub fn ansi() -> Palette {
        Palette {
            map: [
                (0, Color::Yellow),
                (1, Color::DarkCyan),
                (2, Color::Green),
                (3, Color::DarkRed),
                (4, Color::DarkMagenta),
                (5, Color::Red),
                (6, Color::Blue),
                (7, Color::DarkGrey),
                (248, Color::Black),
                (255, Color::White),
            ]
            .into(),
        }
    }

    // pub fn oklch0_palette() -> Palette {
    //     #[rustfmt::skip]
    //     const COLORS_OKLCH: [(u8, Color); 7 + 3] = [
    //         (  0, Color::Rgb{r:234,g:173,b: 55}), // #eaad37
    //         (  1, Color::Rgb{r:  0,g:188,b:184}), // #00bcb8
    //         (  2, Color::Rgb{r:110,g:183,b: 76}), // #6eb74c
    //         (  3, Color::Rgb{r:242,g:113,b:141}), // #e8718d
    //         (  4, Color::Rgb{r:168,g:138,b:250}), // #a88afa
    //         (  5, Color::Rgb{r:240,g:124,b: 67}), // #f07c43
    //         (  6, Color::Rgb{r: 49,g:169,b:253}), // #31a9fd
    //         (  7, Color::Rgb{r:127,g:127,b:127}), // #7f7f7f
    //         (248, Color::Rgb{r:  0,g:  0,b:  0}), // #000000
    //         (255, Color::Rgb{r:255,g:255,b:255}), // #ffffff
    //     ];
    //     HashMap::from(COLORS_OKLCH)
    // }
    pub fn tetro_pastel() -> Palette {
        construct_simple_palette([
            (0, 0xEFAF32),   // Color::Rgb{r:239,g:175,b: 50}),
            (1, 0x00C7C6),   // Color::Rgb{r:  0,g:199,b:198}),
            (2, 0x6CBD46),   // Color::Rgb{r:108,g:189,b: 70}),
            (3, 0xFF577E),   // Color::Rgb{r:255,g: 87,b:126}),
            (4, 0xA482FF),   // Color::Rgb{r:164,g:130,b:255}),
            (5, 0xF57A3E),   // Color::Rgb{r:245,g:122,b: 62}),
            (6, 0x319FFD),   // Color::Rgb{r: 49,g:159,b:253}),
            (7, 0x8F8F8F),   // Color::Rgb{r:143,g:143,b:143}),
            (248, 0x000000), // Color::Rgb{r:  0,g:  0,b:  0}),
            (255, 0xFFFFFF), // Color::Rgb{r:255,g:255,b:255}),
        ])
    }

    pub fn guideline() -> Palette {
        construct_simple_palette([
            (0, 0xFECB01),   // Color::Rgb{r:254,g:203,b:  1}),
            (1, 0x009FDB),   // Color::Rgb{r:  0,g:159,b:219}),
            (2, 0x69BE29),   // Color::Rgb{r:105,g:190,b: 41}),
            (3, 0xED293A),   // Color::Rgb{r:237,g: 41,b: 58}),
            (4, 0x952D99),   // Color::Rgb{r:149,g: 45,b:153}),
            (5, 0xFF6901),   // Color::Rgb{r:255,g:121,b:  1}),
            (6, 0x0065BE),   // Color::Rgb{r:  0,g:101,b:190}),
            (7, 0x7F7F7F),   // Color::Rgb{r:127,g:127,b:127}),
            (248, 0x000000), // Color::Rgb{r:  0,g:  0,b:  0}),
            (255, 0xFFFFFF), // Color::Rgb{r:255,g:255,b:255}),
        ])
    }

    pub fn fahrenheit() -> Palette {
        construct_simple_palette([
            (0, 0xFD9F4D),   // Color::Rgb{r:253,g:159,b: 77}),
            (1, 0x979796),   // Color::Rgb{r:151,g:151,b:150}),
            (2, 0xFECEA0),   // Color::Rgb{r:254,g:206,b:160}),
            (3, 0xCC734D),   // Color::Rgb{r:204,g:115,b: 77}),
            (4, 0x734C4D),   // Color::Rgb{r:115,g: 76,b: 77}),
            (5, 0xCB4A05),   // Color::Rgb{r:203,g: 73,b:  5}),
            (6, 0xCDA074),   // Color::Rgb{r:205,g:160,b:116}),
            (7, 0x7F7F7F),   // Color::Rgb{r:127,g:127,b:127}),
            (248, 0x000000), // Color::Rgb{r:  0,g:  0,b:  0}),
            (255, 0xFFFFCE), // Color::Rgb{r:255,g:255,b:206}),
        ])
    }

    /*pub fn gruvbox_palette() -> Palette {
        #[rustfmt::skip]
        const COLORS_GRUVBOX0: [(u8, Color); 7 + 3] = [
            (  0, Color::Rgb{r:215,g:153,b: 33}), // #D79921
            (  1, Color::Rgb{r:104,g:157,b:106}), // #689D6A
            (  2, Color::Rgb{r:152,g:151,b: 26}), // #98971A
            (  3, Color::Rgb{r:204,g: 36,b: 29}), // #CC241D
            (  4, Color::Rgb{r:177,g: 98,b:134}), // #B16286
            (  5, Color::Rgb{r:214,g: 93,b: 14}), // #D65D0E
            (  6, Color::Rgb{r: 69,g:133,b:136}), // #458588
            (  7, Color::Rgb{r:127,g:127,b:127}), // #7f7f7f
            (248, Color::Rgb{r:  0,g:  0,b:  0}), // #000000
            (255, Color::Rgb{r:255,g:255,b:255}), // #FFFFFF
        ];
        HashMap::from(COLORS_GRUVBOX0)
    }*/
    pub fn gruvbox() -> Palette {
        construct_simple_palette([
            (0, 0xFABD2F),   // Color::Rgb{r:250,g:189,b: 47}),
            (1, 0x8EC07C),   // Color::Rgb{r:142,g:192,b:124}),
            (2, 0xB8BB26),   // Color::Rgb{r:184,g:187,b: 38}),
            (3, 0xFB4934),   // Color::Rgb{r:251,g: 73,b: 52}),
            (4, 0xD3869B),   // Color::Rgb{r:211,g:134,b:155}),
            (5, 0xFE8019),   // Color::Rgb{r:254,g:128,b: 25}),
            (6, 0x83A598),   // Color::Rgb{r:131,g:165,b:152}),
            (7, 0x7F7F7F),   // Color::Rgb{r:127,g:127,b:127}),
            (248, 0x000000), // Color::Rgb{r:  0,g:  0,b:  0}),
            (255, 0xFFFFFF), // Color::Rgb{r:255,g:255,b:255}),
        ])
    }

    /*pub fn lavendel() -> Palette {
        #[rustfmt::skip]
        const COLORS_LAVENDEL: [(u8, Color); 7 + 3] = [
            (  0, Color::Rgb{r:196,g:145,b:222}), // #C491DE
            (  1, Color::Rgb{r:158,g:113,b:200}), // #9E71C8
            (  2, Color::Rgb{r: 59,g: 63,b:130}), // #3B3F82
            (  3, Color::Rgb{r:119,g: 96,b:178}), // #7760B2
            (  4, Color::Rgb{r:216,g:184,b:237}), // #D8B8ED
            (  5, Color::Rgb{r:138,g:115,b:201}), // #8A73C9
            (  6, Color::Rgb{r: 80,g: 79,b:156}), // #504F9C
            (  7, Color::Rgb{r:134,g:134,b:144}), // #868690
            (248, Color::Rgb{r: 19,g: 19,b: 23}), // #131317
            (255, Color::Rgb{r:225,g:227,b:237}), // #E1E3ED
        ];
        HashMap::from(COLORS_LAVENDEL)
    }*/

    /*pub fn nature_suede() -> Palette {
        #[rustfmt::skip]
        const COLORS_NATURE_SUEDE: [(u8, Color); 7 + 3] = [
            (  0, Color::Rgb{r:200,g:157,b: 91}), // #C89D5B
            (  1, Color::Rgb{r:123,g:161,b:108}), // #7BA16C
            (  2, Color::Rgb{r:195,g:164,b: 61}), // #C3A43D
            (  3, Color::Rgb{r:152,g: 98,b: 76}), // #98624C
            (  4, Color::Rgb{r:107,g: 78,b: 68}), // #6B4E44
            (  5, Color::Rgb{r:175,g: 73,b: 47}), // #AF492F
            (  6, Color::Rgb{r: 92,g: 75,b: 66}), // #5C4B42
            (  7, Color::Rgb{r: 92,g: 81,b: 66}), // #5C5142
            (248, Color::Rgb{r: 23,g: 13,b: 13}), // #170D0D
            (255, Color::Rgb{r:228,g:201,b:140}), // #E4C98C
        ];
        HashMap::from(COLORS_NATURE_SUEDE)
    }*/

    /*pub fn papercolor() -> Palette {
        #[rustfmt::skip]
        const COLORS_PAPERCOLOR: [(u8, Color); 7 + 3] = [
            (  0, Color::Rgb{r:255,g:175,b:  0}), // #FFAF00
            (  1, Color::Rgb{r:  0,g:175,b:175}), // #00AFAF
            (  2, Color::Rgb{r:175,g:215,b:  0}), // #AFD700
            (  3, Color::Rgb{r: 88,g: 88,b: 88}), // #585858
            (  4, Color::Rgb{r:175,g:135,b:215}), // #AF87D7
            (  5, Color::Rgb{r:255,g: 95,b:175}), // #FF5FAF
            (  6, Color::Rgb{r: 89,g: 89,b: 89}), // #595959
            (  7, Color::Rgb{r:128,g:128,b:128}), // #808080
            (248, Color::Rgb{r: 28,g: 28,b: 28}), // #1C1C1C
            (255, Color::Rgb{r:208,g:208,b:208}), // #D0D0D0
        ];
        HashMap::from(COLORS_PAPERCOLOR)
    }*/

    pub fn sequoia() -> Palette {
        construct_simple_palette([
            (0, 0xE2E4ED),   // Color::Rgb{r:226,g:228,b:237}),
            (1, 0x9498A9),   // Color::Rgb{r:148,g:152,b:169}),
            (2, 0xD3D5DE),   // Color::Rgb{r:211,g:213,b:222}),
            (3, 0x999EB2),   // Color::Rgb{r:153,g:158,b:178}),
            (4, 0x7C829D),   // Color::Rgb{r:124,g:130,b:157}),
            (5, 0xB6BAC8),   // Color::Rgb{r:182,g:186,b:200}),
            (6, 0x626983),   // Color::Rgb{r: 98,g:105,b:131}),
            (7, 0x868690),   // Color::Rgb{r:134,g:134,b:144}),
            (248, 0x131317), // Color::Rgb{r: 19,g: 19,b: 23}),
            (255, 0xE8EAF2), // Color::Rgb{r:232,g:234,b:242}),
        ])
    }

    pub fn solarized() -> Palette {
        construct_simple_palette([
            (0, 0xb58900),   // Color::Rgb{r:181,g:137,b:  0}),
            (1, 0x2aa198),   // Color::Rgb{r: 42,g:161,b:152}),
            (2, 0x859900),   // Color::Rgb{r:133,g:153,b:  0}),
            (3, 0xd33682),   // Color::Rgb{r:211,g: 54,b:130}),
            (4, 0x6c71c4),   // Color::Rgb{r:108,g:113,b:196}),
            (5, 0xcb4b16),   // Color::Rgb{r:203,g: 75,b: 22}),
            (6, 0x268bd2),   // Color::Rgb{r: 38,g:139,b:210}),
            (7, 0x657b83),   // Color::Rgb{r:101,g:123,b:131}),
            (248, 0x002b36), // Color::Rgb{r:  0,g: 43,b: 54}),
            (255, 0xfdf6e3), // Color::Rgb{r:253,g:246,b:227}),
        ])
    }

    pub fn terafox() -> Palette {
        construct_simple_palette([
            (0, 0xFDB292),   // Color::Rgb{r:253,g:178,b:146}),
            (1, 0xA1CDD8),   // Color::Rgb{r:161,g:205,b:216}),
            (2, 0x8EB2AF),   // Color::Rgb{r:142,g:178,b:175}),
            (3, 0xE85C51),   // Color::Rgb{r:232,g: 92,b: 81}),
            (4, 0xAD5C7C),   // Color::Rgb{r:173,g: 92,b:124}),
            (5, 0xED7A6D),   // Color::Rgb{r:237,g:122,b:109}),
            (6, 0x73A3B7),   // Color::Rgb{r:115,g:163,b:183}),
            (7, 0x4E5157),   // Color::Rgb{r: 78,g: 81,b: 87}),
            (248, 0x1d1f23), // Color::Rgb{r: 19,g: 31,b: 35}),
            (255, 0xDEE4E6), // Color::Rgb{r:222,g:228,b:230}),
        ])
    }

    pub fn matrix() -> Palette {
        construct_simple_palette([
            (0, 0xE9E200),   // Color::Rgb{r:233,g:226,b:  0}),
            (1, 0x2FC079),   // Color::Rgb{r: 47,g:192,b:121}),
            (2, 0x409931),   // Color::Rgb{r: 64,g:153,b: 49}),
            (3, 0x90D762),   // Color::Rgb{r:144,g:215,b: 98}),
            (4, 0x23755A),   // Color::Rgb{r: 35,g:117,b: 90}),
            (5, 0x50B45A),   // Color::Rgb{r: 80,g:180,b: 90}),
            (6, 0x4F7E7E),   // Color::Rgb{r: 79,g:126,b:126}),
            (7, 0x717F73),   // Color::Rgb{r:113,g:127,b:115}),
            (248, 0x0F191C), // Color::Rgb{r: 15,g: 25,b: 28}),
            (255, 0xEAFFF4), // Color::Rgb{r:234,g:255,b:244}),
        ])
    }

    pub fn nes() -> Palette {
        #[rustfmt::skip]
        let raw_map: [u32; 64] = [
            0x7c7c7c,
            0x0000fc,
            0x0000bc,
            0x4428bc,
            0x940084,
            0xa80020,
            0xa81000,
            0x881400,
            0x503000,
            0x007800,
            0x006800,
            0x005800,
            0x004058,
            0x000000,
            0x000000,
            0x000000,

            0xbcbcbc,
            0x0078f8,
            0x0058f8,
            0x6844fc,
            0xd800cc,
            0xe40058,
            0xf83800,
            0xe45c10,
            0xac7c00,
            0x00b800,
            0x00a800,
            0x00a844,
            0x008888,
            0x000000,
            0x000000,
            0x000000,

            0xf8f8f8,
            0x3cbdfc,
            0x6888fc,
            0x9878f8,
            0xf878f8,
            0xf85898,
            0xf87858,
            0xfca044,
            0xf8b800,
            0xb8f818,
            0x58d854,
            0x58f898,
            0x00e8d8,
            0x787878,
            0x000000,
            0x000000,

            0xfcfcfc,
            0xa4e4fc,
            0xb8b8f8,
            0xd8b8f8,
            0xf8b8f8,
            0xf8a4c0,
            0xf0d0b0,
            0xfce0a8,
            0xf8d878,
            0xd8f878,
            0xb8f8b8,
            0xb8f8d8,
            0x00fcfc,
            0xf8d8f8,
            0x000000,
            0x000000,
        ];

        Palette {
            map: HashMap::from_iter(
                raw_map
                    .into_iter()
                    .enumerate()
                    .map(|(i, rgb)| (i as u8, col_from_raw_rgb(rgb))),
            ),
        }
    }
}

// -- ♪ Symphony of Serialization Boilerplate ♫ --

// FIXME: Refactor using  #[serde(try_from = "FromType")]  and  #[serde(into = "IntoType")]  ? (See <https://serde.rs/container-attrs.html>)

// From <https://stackoverflow.com/questions/42723065/how-to-sort-hashmap-keys-when-serializing-with-serde>
//
// For use with serde's [serialize_with] attribute
fn ordered_colormap<S: serde::Serializer>(
    value: &HashMap<u8, Color>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    #[serde_with::serde_as] // Do **NOT** place this after #[derive(..)] !!
    #[derive(serde::Serialize, serde::Deserialize)]
    #[serde(transparent)]
    struct Wrapper(
        // #[serde_as(as = "Vec<(_, ColorDummyType)>")]
        // #[serde_as(
        //     as = "std::collections::BTreeMap<serde_with::json::JsonString, ColorDummyType>"
        // )]
        #[serde_as(as = "std::collections::BTreeMap<_, ColorDummyType>")] BTreeMap<u8, Color>,
    );

    serde::Serialize::serialize(&Wrapper(value.clone().into_iter().collect()), serializer)
}

// FIXME: The following boilerplate is adapted from how Crossterm serializes its `Color`
// (<https://github.com/crossterm-rs/crossterm/blob/master/src/style/types/color.rs#L260>)
// and should maybe be changed or accredited better.
struct ColorDummyType;
#[rustfmt::skip]
impl serde_with::SerializeAs<Color> for ColorDummyType {
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
impl<'de> serde_with::DeserializeAs<'de, Color> for ColorDummyType {
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
