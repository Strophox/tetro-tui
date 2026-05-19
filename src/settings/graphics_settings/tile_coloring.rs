use crate::settings::graphics_settings::ColorSerializationType;

use crossterm::style::Color;

use crate::{
    core_game_engine::{Tetromino, TileType},
    settings::SlotMachine,
};

// pub type ColorLUTIdx = u8;
//
// #[derive(PartialEq, Eq, Clone, Debug, serde::Serialize, serde::Deserialize)]
// #[serde(transparent)]
// pub struct ColorLUT {
//     #[serde(serialize_with = "ordered_colormap")]
//     pub map: HashMap<ColorLUTIdx, Color>,
// }

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum NamedColor {
    Black = 0,
    Gray,
    White,
}

impl<T> std::ops::Index<NamedColor> for [T; 3] {
    type Output = T;
    fn index(&self, index: NamedColor) -> &Self::Output {
        &self[index as usize]
    }
}

#[serde_with::serde_as] // Do **NOT** place this after #[derive(..)] !!
#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct SimpleTileColoring {
    /// An accessible base palette, currently representing [Black, Gray, White].
    #[serde_as(as = "[ColorSerializationType; _]")]
    named_colors: [Color; 3],
    #[serde_as(as = "[ColorSerializationType; _]")]
    tiles_fg: [Color; TileType::VARIANTS.len()],
    #[serde_as(as = "Option<[ColorSerializationType; _]>")]
    tiles_bg: Option<[Color; TileType::VARIANTS.len()]>,
    #[serde_as(as = "(ColorSerializationType, Option<ColorSerializationType>)")]
    uniform_tile: (Color, Option<Color>),
    simplified_tet_col_from_bg_not_fg: bool,
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum TileColoring {
    Simple(SimpleTileColoring),
    Variable(Vec<SimpleTileColoring>),
    HardcodedNES,
}

pub fn tile_coloring_presets() -> SlotMachine<TileColoring> {
    // NOTE: The slot at index 0 is the special 'monochrome'/no color slot.
    let slots = vec![
        (
            "Terminal Default".to_owned(),
            TileColoring::terminal_default(),
        ),
        ("Just white".to_owned(), TileColoring::white()),
        ("ANSI".to_owned(), TileColoring::ansi()),
        ("Tetro Pastel".to_owned(), TileColoring::tetro_pastel()),
        ("Guideline".to_owned(), TileColoring::guideline()),
        ("Gruvbox".to_owned(), TileColoring::gruvbox()),
        ("Solarized".to_owned(), TileColoring::solarized()),
        ("Terafox".to_owned(), TileColoring::terafox()),
        ("Fahrenheit".to_owned(), TileColoring::fahrenheit()),
        ("Matrix".to_owned(), TileColoring::matrix()),
        ("Sequoia".to_owned(), TileColoring::sequoia()),
        ("Just amber".to_owned(), TileColoring::amber()),
        ("NES levels".to_owned(), TileColoring::HardcodedNES),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "Tile Coloring".to_owned())
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum ColorID {
    N(NamedColor),
    T(TileType),
}
// FIXME: Currently hackier than we'd like it to be.
// The issue stems from conflicting interests, odd tradeoffs and suboptimal design:
// - We want colors to be simple to edit, i.e. it should be obvious how to change the colors of Tetromino pieces. This is the reason we directly store `TileType -> Color` translations (instead of having some sort of indirection).
// - On the other hand, we want the flexibility of referring to colors by names, e.g. Red/Green/White/... in visual effects that want to rely on it (e.g. 'burning' line clear requires 'red'/'orange'/...). For this we'd like a uniform access to a palette, perhaps using color names.
//
// We have considered but not yet implemented a solution where there exists a complete `ColorLUT`. It would likely use an `enum NamedColor { Black, Yellow, ... }` and tetrominos would preferably be colored using that?
// The problem with that is that tetrominos, at their *very* core, are not associated with specific named colors (e.g. all of them are a different shade of green in the 'Matrix' theme).
// A more abstract ColorLUT (indexed by `u8`) might have to be used, then?
// FIXME: One last idea might be to store the tetromino colors and then the 'actual' plain/named colors separately. That way effects can explicitly rely on the 'actual' colors though it is unclear whether the 'Matrix' theme would then actually provide e.g. a strong Red. This could work, though slightly increases complexity/savefile size (probably reasonably).
//
// What our current solution does: We store everything based on just tetromino colors and a handful plain colors
impl ColorID {
    pub const YELLOW: Self = ColorID::T(TileType::Tet(Tetromino::O));
    pub const CYAN: Self = ColorID::T(TileType::Tet(Tetromino::I));
    pub const GREEN: Self = ColorID::T(TileType::Tet(Tetromino::S));
    pub const RED: Self = ColorID::T(TileType::Tet(Tetromino::Z));
    pub const PURPLE: Self = ColorID::T(TileType::Tet(Tetromino::T));
    pub const ORANGE: Self = ColorID::T(TileType::Tet(Tetromino::L));
    pub const BLUE: Self = ColorID::T(TileType::Tet(Tetromino::J));
    pub const BLACK: Self = ColorID::N(NamedColor::Black);
    #[allow(unused)]
    pub const GRAY: Self = ColorID::N(NamedColor::Gray);
    pub const WHITE: Self = ColorID::N(NamedColor::White);
}

impl TileColoring {
    pub fn lookup_col_id(&self, id: ColorID, level: usize) -> Color {
        match id {
            ColorID::N(named_color) => self.named_colors(level)[named_color],
            ColorID::T(tile_type) => self.simplified_tile_col(tile_type, level),
        }
    }

    pub fn uniform_tile(&self, level: usize) -> (Color, Option<Color>) {
        match self {
            TileColoring::Simple(clrng) => clrng.uniform_tile,
            TileColoring::Variable(clrngs) => clrngs[level % clrngs.len()].uniform_tile,
            TileColoring::HardcodedNES => (NES_WHITE, Some(NES_PALETTE[0x10])),
        }
    }

    pub fn named_colors(&self, level: usize) -> [Color; 3] {
        match self {
            TileColoring::Simple(clrng) => clrng.named_colors,
            TileColoring::Variable(clrngs) => clrngs[level % clrngs.len()].named_colors,
            TileColoring::HardcodedNES => [NES_BLACK, NES_GRAY, NES_WHITE],
        }
    }

    /// Produce a single color to represent the tile for (even more) constrainted environments (e.g. small piece preview).
    pub fn simplified_tile_col(&self, tile: TileType, level: usize) -> Color {
        let (fg, opt_bg) = self.tile_col(tile, level);
        let try_get_from_bg = match self {
            TileColoring::Simple(clrng) => clrng.simplified_tet_col_from_bg_not_fg,
            TileColoring::Variable(clrngs) => {
                clrngs[level % clrngs.len()].simplified_tet_col_from_bg_not_fg
            }
            TileColoring::HardcodedNES => true,
        };
        if try_get_from_bg {
            opt_bg.unwrap_or(fg)
        } else {
            fg
        }
    }

    pub fn tile_col(&self, tile: TileType, level: usize) -> (Color, Option<Color>) {
        match self {
            TileColoring::Simple(clrng) => (
                clrng.tiles_fg[usize::from(tile)],
                clrng.tiles_bg.map(|a| a[usize::from(tile)]),
            ),
            TileColoring::Variable(clrngs) => {
                let clrng = &clrngs[level % clrngs.len()];
                (
                    clrng.tiles_fg[usize::from(tile)],
                    clrng.tiles_bg.map(|a| a[usize::from(tile)]),
                )
            }
            TileColoring::HardcodedNES => {
                let tet = match tile {
                    TileType::Generic => return (NES_WHITE, Some(NES_GRAY)),
                    TileType::Tet(tet) => tet,
                };
                // Handle actual tetromino colors
                const COLOR_RUN_BASE: [(usize, usize, usize); 10] = [
                    (0x30, 0x21, 0x12),
                    (0x30, 0x29, 0x1a),
                    (0x30, 0x24, 0x14),
                    (0x30, 0x2a, 0x12),
                    (0x30, 0x2b, 0x15),
                    (0x30, 0x22, 0x2b),
                    (0x30, 0x00, 0x16),
                    (0x30, 0x05, 0x13),
                    (0x30, 0x16, 0x12),
                    (0x30, 0x27, 0x16),
                ];
                // TODO
                const COLOR_RUN_GLITCHED: [(usize, usize, usize); 54] = [(0, 0, 0); 54];
                // E.g. O/I/T are 'mainly' white, Z/L are red, S/J are blue (at level 8).
                let (oit, zl, sj) = match level % 256 {
                    0..=137 => COLOR_RUN_BASE[level % 10],
                    138..=191 => COLOR_RUN_GLITCHED[level - 138],
                    192..=201 => COLOR_RUN_BASE[level - 192],
                    202..=255 => COLOR_RUN_GLITCHED[level - 202],
                    _ => unreachable!(),
                };
                let (col_oit, col_zl, col_sj) =
                    (NES_PALETTE[oit], NES_PALETTE[zl], NES_PALETTE[sj]);
                match tet {
                    Tetromino::O | Tetromino::I | Tetromino::T => (col_sj, Some(col_oit)),
                    Tetromino::Z | Tetromino::L => (col_oit, Some(col_zl)),
                    Tetromino::S | Tetromino::J => (col_oit, Some(col_sj)),
                }
            }
        }
    }

    // FIXME: Done quick and dirty, not meant for performant contexts yet.
    pub fn tetromino_rainbow(&self) -> [Color; Tetromino::VARIANTS.len()] {
        use Tetromino::*;
        [I, J, T, Z, L, O, S].map(|tet| self.simplified_tile_col(tet.into(), 0))
    }
}

pub const NES_PALETTE: [Color; 64] = const {
    let src = [
        0x7c7c7c, 0x0000fc, 0x0000bc, 0x4428bc, 0x940084, 0xa80020, 0xa81000, 0x881400, 0x503000,
        0x007800, 0x006800, 0x005800, 0x004058, 0x000000, 0x000000, 0x000000, 0xbcbcbc, 0x0078f8,
        0x0058f8, 0x6844fc, 0xd800cc, 0xe40058, 0xf83800, 0xe45c10, 0xac7c00, 0x00b800, 0x00a800,
        0x00a844, 0x008888, 0x000000, 0x000000, 0x000000, 0xf8f8f8, 0x3cbdfc, 0x6888fc, 0x9878f8,
        0xf878f8, 0xf85898, 0xf87858, 0xfca044, 0xf8b800, 0xb8f818, 0x58d854, 0x58f898, 0x00e8d8,
        0x787878, 0x000000, 0x000000, 0xfcfcfc, 0xa4e4fc, 0xb8b8f8, 0xd8b8f8, 0xf8b8f8, 0xf8a4c0,
        0xf0d0b0, 0xfce0a8, 0xf8d878, 0xd8f878, 0xb8f8b8, 0xb8f8d8, 0x00fcfc, 0xf8d8f8, 0x000000,
        0x000000,
    ];
    // FIXME: Once `array::map` becomes constified use it instead.
    let mut dst = [Color::Reset; 64];
    let mut idx = 0;
    while idx < 64 {
        let int = src[idx];
        dst[idx] = Color::Rgb {
            r: ((int >> 16) & 0xFF) as u8,
            g: ((int >> 8) & 0xFF) as u8,
            b: (int & 0xFF) as u8,
        };
        idx += 1;
    }
    dst
};
pub const NES_BLACK: Color = NES_PALETTE[0x3F];
pub const NES_GRAY: Color = NES_PALETTE[0x2D]; //NES_PALETTE[0];
pub const NES_WHITE: Color = NES_PALETTE[0x30];

fn read_rgb(rgb: &str) -> Color {
    let int = u32::from_str_radix(rgb.trim_start_matches('#'), 16).unwrap();
    Color::Rgb {
        r: ((int >> 16) & 0xFF) as u8,
        g: ((int >> 8) & 0xFF) as u8,
        b: (int & 0xFF) as u8,
    }
}

fn new_simple_tile_coloring(
    tile_colors: [&str; TileType::VARIANTS.len()],
    plain_colors: [&str; 3],
) -> TileColoring {
    let plain_colors = plain_colors.map(read_rgb);
    let tiles_fg = tile_colors.map(read_rgb);
    TileColoring::Simple(SimpleTileColoring {
        named_colors: plain_colors,
        tiles_fg,
        tiles_bg: None,
        uniform_tile: (plain_colors[NamedColor::White], None),
        simplified_tet_col_from_bg_not_fg: false,
    })
}

impl TileColoring {
    pub fn terminal_default() -> Self {
        TileColoring::Simple(SimpleTileColoring {
            tiles_fg: [Color::Reset; 8],
            tiles_bg: None,
            uniform_tile: (Color::Reset, None),
            named_colors: [Color::Reset; 3],
            simplified_tet_col_from_bg_not_fg: false,
        })
    }

    pub fn white() -> Self {
        let white = "ffffff";
        new_simple_tile_coloring([white; 8], [white; 3])
    }

    pub fn ansi() -> Self {
        let tiles_fg = [
            Color::Yellow,
            Color::DarkCyan,
            Color::Green,
            Color::DarkRed,
            Color::DarkMagenta,
            Color::Red,
            Color::Blue,
            Color::DarkGrey,
        ];
        let plain_colors = [Color::Black, Color::DarkGrey, Color::White];
        TileColoring::Simple(SimpleTileColoring {
            tiles_fg,
            tiles_bg: None,
            uniform_tile: (Color::Reset, None),
            named_colors: plain_colors,
            simplified_tet_col_from_bg_not_fg: false,
        })
    }

    // pub fn oklch0_palette() -> Self {
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

    pub fn tetro_pastel() -> Self {
        new_simple_tile_coloring(
            [
                "#EFAF32", // Color::Rgb{r:239,g:175,b: 50}),
                "#00C7C6", // Color::Rgb{r:  0,g:199,b:198}),
                "#6CBD46", // Color::Rgb{r:108,g:189,b: 70}),
                "#FF577E", // Color::Rgb{r:255,g: 87,b:126}),
                "#A482FF", // Color::Rgb{r:164,g:130,b:255}),
                "#F57A3E", // Color::Rgb{r:245,g:122,b: 62}),
                "#319FFD", // Color::Rgb{r: 49,g:159,b:253}),
                "#8F8F8F", // Color::Rgb{r:143,g:143,b:143}),
            ],
            [
                "#000000", // Color::Rgb{r:  0,g:  0,b:  0}),
                "#8F8F8F", // Color::Rgb{r:143,g:143,b:143}),
                "#FFFFFF", // Color::Rgb{r:255,g:255,b:255}),
            ],
        )
    }

    pub fn guideline() -> Self {
        new_simple_tile_coloring(
            [
                "#FECB01", // Color::Rgb{r:254,g:203,b:  1}),
                "#009FDB", // Color::Rgb{r:  0,g:159,b:219}),
                "#69BE29", // Color::Rgb{r:105,g:190,b: 41}),
                "#ED293A", // Color::Rgb{r:237,g: 41,b: 58}),
                "#952D99", // Color::Rgb{r:149,g: 45,b:153}),
                "#FF6901", // Color::Rgb{r:255,g:121,b:  1}),
                "#0065BE", // Color::Rgb{r:  0,g:101,b:190}),
                "#7F7F7F", // Color::Rgb{r:127,g:127,b:127}),
            ],
            [
                "#000000", // Color::Rgb{r:  0,g:  0,b:  0}),
                "#7F7F7F", // Color::Rgb{r:127,g:127,b:127}),
                "#FFFFFF", // Color::Rgb{r:255,g:255,b:255}),
            ],
        )
    }

    pub fn fahrenheit() -> Self {
        new_simple_tile_coloring(
            [
                "#FD9F4D", // Color::Rgb{r:253,g:159,b: 77}),
                "#979796", // Color::Rgb{r:151,g:151,b:150}),
                "#FECEA0", // Color::Rgb{r:254,g:206,b:160}),
                "#CC734D", // Color::Rgb{r:204,g:115,b: 77}),
                "#734C4D", // Color::Rgb{r:115,g: 76,b: 77}),
                "#CB4A05", // Color::Rgb{r:203,g: 73,b:  5}),
                "#CDA074", // Color::Rgb{r:205,g:160,b:116}),
                "#7F7F7F", // Color::Rgb{r:127,g:127,b:127}),
            ],
            [
                "#000000", // Color::Rgb{r:  0,g:  0,b:  0}),
                "#7F7F7F", // Color::Rgb{r:127,g:127,b:127}),
                "#FFFFCE", // Color::Rgb{r:255,g:255,b:206}),
            ],
        )
    }

    /*pub fn gruvbox_palette() -> Self {
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
    pub fn gruvbox() -> Self {
        new_simple_tile_coloring(
            [
                "#FABD2F", // Color::Rgb{r:250,g:189,b: 47}),
                "#8EC07C", // Color::Rgb{r:142,g:192,b:124}),
                "#B8BB26", // Color::Rgb{r:184,g:187,b: 38}),
                "#FB4934", // Color::Rgb{r:251,g: 73,b: 52}),
                "#D3869B", // Color::Rgb{r:211,g:134,b:155}),
                "#FE8019", // Color::Rgb{r:254,g:128,b: 25}),
                "#83A598", // Color::Rgb{r:131,g:165,b:152}),
                "#7F7F7F", // Color::Rgb{r:127,g:127,b:127}),
            ],
            [
                "#000000", // Color::Rgb{r:  0,g:  0,b:  0}),
                "#7F7F7F", // Color::Rgb{r:127,g:127,b:127}),
                "#FFFFFF", // Color::Rgb{r:255,g:255,b:255}),
            ],
        )
    }

    /*pub fn lavendel() -> Self {
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

    /*pub fn nature_suede() -> Self {
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

    /*pub fn papercolor() -> Self {
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

    pub fn solarized() -> Self {
        new_simple_tile_coloring(
            [
                "#b58900", // Color::Rgb{r:181,g:137,b:  0}),
                "#2aa198", // Color::Rgb{r: 42,g:161,b:152}),
                "#859900", // Color::Rgb{r:133,g:153,b:  0}),
                "#d33682", // Color::Rgb{r:211,g: 54,b:130}),
                "#6c71c4", // Color::Rgb{r:108,g:113,b:196}),
                "#cb4b16", // Color::Rgb{r:203,g: 75,b: 22}),
                "#268bd2", // Color::Rgb{r: 38,g:139,b:210}),
                "#657b83", // Color::Rgb{r:101,g:123,b:131}),
            ],
            [
                "#002b36", // Color::Rgb{r:  0,g: 43,b: 54}),
                "#657b83", // Color::Rgb{r:101,g:123,b:131}),
                "#fdf6e3", // Color::Rgb{r:253,g:246,b:227}),
            ],
        )
    }

    pub fn terafox() -> Self {
        new_simple_tile_coloring(
            [
                "#FDB292", // Color::Rgb{r:253,g:178,b:146}),
                "#A1CDD8", // Color::Rgb{r:161,g:205,b:216}),
                "#8EB2AF", // Color::Rgb{r:142,g:178,b:175}),
                "#E85C51", // Color::Rgb{r:232,g: 92,b: 81}),
                "#AD5C7C", // Color::Rgb{r:173,g: 92,b:124}),
                "#ED7A6D", // Color::Rgb{r:237,g:122,b:109}),
                "#73A3B7", // Color::Rgb{r:115,g:163,b:183}),
                "#4E5157", // Color::Rgb{r: 78,g: 81,b: 87}),
            ],
            [
                "#1d1f23", // Color::Rgb{r: 19,g: 31,b: 35}),
                "#4E5157", // Color::Rgb{r: 78,g: 81,b: 87}),
                "#DEE4E6", // Color::Rgb{r:222,g:228,b:230}),
            ],
        )
    }

    pub fn matrix() -> Self {
        new_simple_tile_coloring(
            [
                "#E9E200", // Color::Rgb{r:233,g:226,b:  0}),
                "#2FC079", // Color::Rgb{r: 47,g:192,b:121}),
                "#409931", // Color::Rgb{r: 64,g:153,b: 49}),
                "#90D762", // Color::Rgb{r:144,g:215,b: 98}),
                "#23755A", // Color::Rgb{r: 35,g:117,b: 90}),
                "#50B45A", // Color::Rgb{r: 80,g:180,b: 90}),
                "#4F7E7E", // Color::Rgb{r: 79,g:126,b:126}),
                "#717F73", // Color::Rgb{r:113,g:127,b:115}),
            ],
            [
                "#0F191C", // Color::Rgb{r: 15,g: 25,b: 28}),
                "#717F73", // Color::Rgb{r:113,g:127,b:115}),
                "#EAFFF4", // Color::Rgb{r:234,g:255,b:244}),
            ],
        )
    }

    pub fn sequoia() -> Self {
        new_simple_tile_coloring(
            [
                "#E2E4ED", // Color::Rgb{r:226,g:228,b:237}),
                "#9498A9", // Color::Rgb{r:148,g:152,b:169}),
                "#D3D5DE", // Color::Rgb{r:211,g:213,b:222}),
                "#999EB2", // Color::Rgb{r:153,g:158,b:178}),
                "#7C829D", // Color::Rgb{r:124,g:130,b:157}),
                "#B6BAC8", // Color::Rgb{r:182,g:186,b:200}),
                "#626983", // Color::Rgb{r: 98,g:105,b:131}),
                "#868690", // Color::Rgb{r:134,g:134,b:144}),
            ],
            [
                "#131317", // Color::Rgb{r: 19,g: 19,b: 23}),
                "#868690", // Color::Rgb{r:134,g:134,b:144}),
                "#E8EAF2", // Color::Rgb{r:232,g:234,b:242}),
            ],
        )
    }

    pub fn amber() -> Self {
        let amber = "ff9400";
        new_simple_tile_coloring([amber; 8], [amber; 3])
    }
}
