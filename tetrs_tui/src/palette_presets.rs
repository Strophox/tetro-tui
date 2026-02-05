use std::collections::HashMap;

use crossterm::style::Color;

pub type Palette = HashMap<u8, Color>;

pub fn monochrome_palette() -> Palette {
    HashMap::new()
}

pub fn color16_palette() -> Palette {
    const COLORS_COLOR16: [(u8, Color); 7 + 3] = [
        (1, Color::Yellow),
        (2, Color::DarkCyan),
        (3, Color::Green),
        (4, Color::DarkRed),
        (5, Color::DarkMagenta),
        (6, Color::Red),
        (7, Color::Blue),
        (253, Color::Black),
        (254, Color::DarkGrey),
        (255, Color::White),
    ];
    HashMap::from(COLORS_COLOR16)
}

pub fn oklch_palette() -> Palette {
    #[rustfmt::skip]
    const COLORS_OKLCH_INCRSAT: [(u8, Color); 7 + 3] = [
        (  1, Color::Rgb{r:239,g:175,b: 50}), // #EFAF32
        (  2, Color::Rgb{r:  0,g:199,b:198}), // #00C7C6
        (  3, Color::Rgb{r:108,g:189,b: 70}), // #6CBD46
        (  4, Color::Rgb{r:255,g: 99,b:133}), // #FF6385
        (  5, Color::Rgb{r:164,g:130,b:255}), // #A482FF
        (  6, Color::Rgb{r:245,g:122,b: 62}), // #F57A3E
        (  7, Color::Rgb{r: 49,g:159,b:253}), // #319FFD

        (253, Color::Rgb{r:  0,g:  0,b:  0}), // #000000
        (254, Color::Rgb{r:127,g:127,b:127}), // #7F7F7F
        (255, Color::Rgb{r:255,g:255,b:255}), // #FFFFFF
    ];
    HashMap::from(COLORS_OKLCH_INCRSAT)
}

// pub fn oklch0_palette() -> Palette {
//     #[rustfmt::skip]
//     const COLORS_OKLCH: [(u8, Color); 7 + 3] = [
//         (  1, Color::Rgb{r:234,g:173,b: 55}), // #eaad37
//         (  2, Color::Rgb{r:  0,g:188,b:184}), // #00bcb8
//         (  3, Color::Rgb{r:110,g:183,b: 76}), // #6eb74c
//         (  4, Color::Rgb{r:242,g:113,b:141}), // #e8718d
//         (  5, Color::Rgb{r:168,g:138,b:250}), // #a88afa
//         (  6, Color::Rgb{r:240,g:124,b: 67}), // #f07c43
//         (  7, Color::Rgb{r: 49,g:169,b:253}), // #31a9fd
//
//         (253, Color::Rgb{r:  0,g:  0,b:  0}), // #000000
//         (254, Color::Rgb{r:127,g:127,b:127}), // #7f7f7f
//         (255, Color::Rgb{r:255,g:255,b:255}), // #ffffff
//     ];
//     HashMap::from(COLORS_OKLCH)
// }

pub fn fullcolor_palette() -> Palette {
    #[rustfmt::skip]
    const COLORS_DEFAULT: [(u8, Color); 7 + 3] = [
        (  1, Color::Rgb{r:254,g:203,b:  1}), // #FECB01
        (  2, Color::Rgb{r:  0,g:159,b:219}), // #009FDB
        (  3, Color::Rgb{r:105,g:190,b: 41}), // #69BE29
        (  4, Color::Rgb{r:237,g: 41,b: 58}), // #ED293A
        (  5, Color::Rgb{r:149,g: 45,b:153}), // #952D99
        (  6, Color::Rgb{r:255,g:121,b:  1}), // #FF6901
        (  7, Color::Rgb{r:  0,g:101,b:190}), // #0065BE

        (253, Color::Rgb{r:  0,g:  0,b:  0}), // #000000
        (254, Color::Rgb{r:127,g:127,b:127}), // #7F7F7F
        (255, Color::Rgb{r:255,g:255,b:255}), // #FFFFFF
    ];
    HashMap::from(COLORS_DEFAULT)
}

pub fn gruvbox_palette() -> Palette {
    #[rustfmt::skip]
    const COLORS_GRUVBOX_NORMAL: [(u8, Color); 7 + 3] = [
        (  1, Color::Rgb{r:215,g:153,b: 33}), // #D79921
        (  2, Color::Rgb{r:104,g:157,b:106}), // #689D6A
        (  3, Color::Rgb{r:152,g:151,b: 26}), // #98971A
        (  4, Color::Rgb{r:204,g: 36,b: 29}), // #CC241D
        (  5, Color::Rgb{r:177,g: 98,b:134}), // #B16286
        (  6, Color::Rgb{r:214,g: 93,b: 14}), // #D65D0E
        (  7, Color::Rgb{r: 69,g:133,b:136}), // #458588

        (253, Color::Rgb{r:  0,g:  0,b:  0}), // #000000
        (254, Color::Rgb{r:127,g:127,b:127}), // #7f7f7f
        (255, Color::Rgb{r:255,g:255,b:255}), // #FFFFFF
    ];
    HashMap::from(COLORS_GRUVBOX_NORMAL)
}

pub fn gruvbox_light_palette() -> Palette {
    #[rustfmt::skip]
    const COLORS_GRUVBOX_LIGHT: [(u8, Color); 7 + 3] = [
        (  1, Color::Rgb{r:250,g:189,b: 47}), // #FABD2F
        (  2, Color::Rgb{r:142,g:192,b:124}), // #8EC07C
        (  3, Color::Rgb{r:184,g:187,b: 38}), // #B8BB26
        (  4, Color::Rgb{r:251,g: 73,b: 52}), // #FB4934
        (  5, Color::Rgb{r:211,g:134,b:155}), // #D3869B
        (  6, Color::Rgb{r:254,g:128,b: 25}), // #FE8019
        (  7, Color::Rgb{r:131,g:165,b:152}), // #83A598

        (253, Color::Rgb{r:  0,g:  0,b:  0}), // #000000
        (254, Color::Rgb{r:127,g:127,b:127}), // #7F7F7F
        (255, Color::Rgb{r:255,g:255,b:255}), // #FFFFFF
    ];
    HashMap::from(COLORS_GRUVBOX_LIGHT)
}

pub fn the_matrix_palette() -> Palette {
    #[rustfmt::skip]
    const COLORS_THE_MATRIX: [(u8, Color); 7 + 3] = [
        (  1, Color::Rgb{r:233,g:226,b:  0}), // #FFE200
        (  2, Color::Rgb{r: 80,g:180,b: 90}), // #50B45A
        (  3, Color::Rgb{r:144,g:215,b: 98}), // #90D762
        (  4, Color::Rgb{r: 35,g:117,b: 90}), // #23755A
        (  5, Color::Rgb{r: 64,g:153,b: 49}), // #409931
        (  6, Color::Rgb{r: 47,g:192,b:121}), // #2FC079
        (  7, Color::Rgb{r: 79,g:126,b:126}), // #4F7E7E

        (253, Color::Rgb{r: 15,g: 25,b: 28}), // #0F191C
        (254, Color::Rgb{r:113,g:127,b:115}), // #717F73
        (255, Color::Rgb{r:234,g:255,b:244}), // #EAFFF4
    ];
    HashMap::from(COLORS_THE_MATRIX)
}
