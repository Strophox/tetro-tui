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
pub fn oklch_palette() -> Palette {
    #[rustfmt::skip]
    const COLORS_OKLCH_INCRSAT: [(u8, Color); 7 + 3] = [
        (  1, Color::Rgb{r:239,g:175,b: 50}), // #EFAF32
        (  2, Color::Rgb{r:  0,g:199,b:198}), // #00C7C6
        (  3, Color::Rgb{r:108,g:189,b: 70}), // #6CBD46
        (  4, Color::Rgb{r:255,g: 87,b:126}), // #FF577E
        (  5, Color::Rgb{r:164,g:130,b:255}), // #A482FF
        (  6, Color::Rgb{r:245,g:122,b: 62}), // #F57A3E
        (  7, Color::Rgb{r: 49,g:159,b:253}), // #319FFD

        (253, Color::Rgb{r:  0,g:  0,b:  0}), // #000000
        (254, Color::Rgb{r:127,g:127,b:127}), // #7F7F7F
        (255, Color::Rgb{r:255,g:255,b:255}), // #FFFFFF
    ];
    HashMap::from(COLORS_OKLCH_INCRSAT)
}

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

pub fn fahrenheit_palette() -> Palette {
    #[rustfmt::skip]
    const COLORS_FAHRENHEIT: [(u8, Color); 7 + 3] = [
        (  1, Color::Rgb{r:253,g:159,b: 77}), // #FD9F4D
        (  2, Color::Rgb{r:151,g:151,b:150}), // #979796
        (  3, Color::Rgb{r:254,g:206,b:160}), // #FECEA0
        (  4, Color::Rgb{r:204,g:115,b: 77}), // #CC734D
        (  5, Color::Rgb{r:115,g: 76,b: 77}), // #734C4D
        (  6, Color::Rgb{r:203,g: 73,b:  5}), // #CB4A05
        (  7, Color::Rgb{r:205,g:160,b:116}), // #CDA074

        (253, Color::Rgb{r:  0,g:  0,b:  0}), // #000000
        (254, Color::Rgb{r:127,g:127,b:127}), // #7F7F7F
        (255, Color::Rgb{r:255,g:255,b:206}), // #FFFFCE
    ];
    HashMap::from(COLORS_FAHRENHEIT)
}

/*pub fn gruvbox_palette() -> Palette {
    #[rustfmt::skip]
    const COLORS_GRUVBOX0: [(u8, Color); 7 + 3] = [
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
    HashMap::from(COLORS_GRUVBOX0)
}*/
pub fn gruvbox_palette() -> Palette {
    #[rustfmt::skip]
    const COLORS_GRUVBOX1: [(u8, Color); 7 + 3] = [
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
    HashMap::from(COLORS_GRUVBOX1)
}

/*pub fn lavendel_palette() -> Palette {
    #[rustfmt::skip]
    const COLORS_LAVENDEL: [(u8, Color); 7 + 3] = [
        (  1, Color::Rgb{r:196,g:145,b:222}), // #C491DE
        (  2, Color::Rgb{r:158,g:113,b:200}), // #9E71C8
        (  3, Color::Rgb{r: 59,g: 63,b:130}), // #3B3F82
        (  4, Color::Rgb{r:119,g: 96,b:178}), // #7760B2
        (  5, Color::Rgb{r:216,g:184,b:237}), // #D8B8ED
        (  6, Color::Rgb{r:138,g:115,b:201}), // #8A73C9
        (  7, Color::Rgb{r: 80,g: 79,b:156}), // #504F9C

        (253, Color::Rgb{r: 19,g: 19,b: 23}), // #131317
        (254, Color::Rgb{r:134,g:134,b:144}), // #868690
        (255, Color::Rgb{r:225,g:227,b:237}), // #E1E3ED
    ];
    HashMap::from(COLORS_LAVENDEL)
}*/

/*pub fn nature_suede_palette() -> Palette {
    #[rustfmt::skip]
    const COLORS_NATURE_SUEDE: [(u8, Color); 7 + 3] = [
        (  1, Color::Rgb{r:200,g:157,b: 91}), // #C89D5B
        (  2, Color::Rgb{r:123,g:161,b:108}), // #7BA16C
        (  3, Color::Rgb{r:195,g:164,b: 61}), // #C3A43D
        (  4, Color::Rgb{r:152,g: 98,b: 76}), // #98624C
        (  5, Color::Rgb{r:107,g: 78,b: 68}), // #6B4E44
        (  6, Color::Rgb{r:175,g: 73,b: 47}), // #AF492F
        (  7, Color::Rgb{r: 92,g: 75,b: 66}), // #5C4B42

        (253, Color::Rgb{r: 23,g: 13,b: 13}), // #170D0D
        (254, Color::Rgb{r: 92,g: 81,b: 66}), // #5C5142
        (255, Color::Rgb{r:228,g:201,b:140}), // #E4C98C
    ];
    HashMap::from(COLORS_NATURE_SUEDE)
}*/

/*pub fn papercolor_palette() -> Palette {
    #[rustfmt::skip]
    const COLORS_PAPERCOLOR: [(u8, Color); 7 + 3] = [
        (  1, Color::Rgb{r:255,g:175,b:  0}), // #FFAF00
        (  2, Color::Rgb{r:  0,g:175,b:175}), // #00AFAF
        (  3, Color::Rgb{r:175,g:215,b:  0}), // #AFD700
        (  4, Color::Rgb{r: 88,g: 88,b: 88}), // #585858
        (  5, Color::Rgb{r:175,g:135,b:215}), // #AF87D7
        (  6, Color::Rgb{r:255,g: 95,b:175}), // #FF5FAF
        (  7, Color::Rgb{r: 89,g: 89,b: 89}), // #595959

        (253, Color::Rgb{r: 28,g: 28,b: 28}), // #1C1C1C
        (254, Color::Rgb{r:128,g:128,b:128}), // #808080
        (255, Color::Rgb{r:208,g:208,b:208}), // #D0D0D0
    ];
    HashMap::from(COLORS_PAPERCOLOR)
}*/

pub fn sequoia_palette() -> Palette {
    #[rustfmt::skip]
    const COLORS_SEQUOIA: [(u8, Color); 7 + 3] = [
        (  1, Color::Rgb{r:226,g:228,b:237}), // #E2E4ED
        (  2, Color::Rgb{r:148,g:152,b:169}), // #9498A9
        (  3, Color::Rgb{r:211,g:213,b:222}), // #D3D5DE
        (  4, Color::Rgb{r:153,g:158,b:178}), // #999EB2
        (  5, Color::Rgb{r:124,g:130,b:157}), // #7C829D
        (  6, Color::Rgb{r:182,g:186,b:200}), // #B6BAC8
        (  7, Color::Rgb{r: 98,g:105,b:131}), // #626983

        (253, Color::Rgb{r: 19,g: 19,b: 23}), // #131317
        (254, Color::Rgb{r:134,g:134,b:144}), // #868690
        (255, Color::Rgb{r:232,g:234,b:242}), // #E8EAF2
    ];
    HashMap::from(COLORS_SEQUOIA)
}

pub fn solarized_palette() -> Palette {
    #[rustfmt::skip]
    const COLORS_SOLARIZED: [(u8, Color); 7 + 3] = [
        (  1, Color::Rgb{r:181,g:137,b:  0}), // #b58900
        (  2, Color::Rgb{r: 42,g:161,b:152}), // #2aa198
        (  3, Color::Rgb{r:133,g:153,b:  0}), // #859900
        (  4, Color::Rgb{r:211,g: 54,b:130}), // #d33682
        (  5, Color::Rgb{r:108,g:113,b:196}), // #6c71c4
        (  6, Color::Rgb{r:203,g: 75,b: 22}), // #cb4b16
        (  7, Color::Rgb{r: 38,g:139,b:210}), // #268bd2

        (253, Color::Rgb{r:  0,g: 43,b: 54}), // #002b36
        (254, Color::Rgb{r:101,g:123,b:131}), // #657b83
        (255, Color::Rgb{r:253,g:246,b:227}), // #fdf6e3
    ];
    HashMap::from(COLORS_SOLARIZED)
}

pub fn terafox_palette() -> Palette {
    #[rustfmt::skip]
    const COLORS_TERAFOX: [(u8, Color); 7 + 3] = [
        (  1, Color::Rgb{r:253,g:178,b:146}), // #FDB292
        (  2, Color::Rgb{r:161,g:205,b:216}), // #A1CDD8
        (  3, Color::Rgb{r:142,g:178,b:175}), // #8EB2AF
        (  4, Color::Rgb{r:232,g: 92,b: 81}), // #E85C51
        (  5, Color::Rgb{r:173,g: 92,b:124}), // #AD5C7C
        (  6, Color::Rgb{r:237,g:122,b:109}), // #ED7A6D
        (  7, Color::Rgb{r:115,g:163,b:183}), // #73A3B7

        (253, Color::Rgb{r: 19,g: 31,b: 35}), // #1d1f23
        (254, Color::Rgb{r: 78,g: 81,b: 87}), // #4E5157
        (255, Color::Rgb{r:222,g:228,b:230}), // #DEE4E6
    ];
    HashMap::from(COLORS_TERAFOX)
}

pub fn the_matrix_palette() -> Palette {
    #[rustfmt::skip]
    const COLORS_THE_MATRIX: [(u8, Color); 7 + 3] = [
        (  1, Color::Rgb{r:233,g:226,b:  0}), // #E9E200
        (  2, Color::Rgb{r: 47,g:192,b:121}), // #2FC079
        (  3, Color::Rgb{r: 64,g:153,b: 49}), // #409931
        (  4, Color::Rgb{r:144,g:215,b: 98}), // #90D762
        (  5, Color::Rgb{r: 35,g:117,b: 90}), // #23755A
        (  6, Color::Rgb{r: 80,g:180,b: 90}), // #50B45A
        (  7, Color::Rgb{r: 79,g:126,b:126}), // #4F7E7E

        (253, Color::Rgb{r: 15,g: 25,b: 28}), // #0F191C
        (254, Color::Rgb{r:113,g:127,b:115}), // #717F73
        (255, Color::Rgb{r:234,g:255,b:244}), // #EAFFF4
    ];
    HashMap::from(COLORS_THE_MATRIX)
}
