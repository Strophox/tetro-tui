#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum ThemeSong {
    KorobeinikiA,
    KorobeinikiB,
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum SfxPack {
    Classic,
    Arcade,
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
#[serde(default)]
pub struct AudioSettings {
    pub enabled: bool,
    pub theme_enabled: bool,
    pub sfx_enabled: bool,
    pub theme_song: ThemeSong,
    pub sfx_pack: SfxPack,
    pub theme_tempo_percent: u16,
    pub keypress_sfx: bool,
    pub piece_lock_sfx: bool,
    pub line_clear_sfx: bool,
    pub game_over_sfx: bool,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            theme_enabled: true,
            sfx_enabled: true,
            theme_song: ThemeSong::KorobeinikiA,
            sfx_pack: SfxPack::Classic,
            theme_tempo_percent: 100,
            keypress_sfx: true,
            piece_lock_sfx: true,
            line_clear_sfx: true,
            game_over_sfx: true,
        }
    }
}
