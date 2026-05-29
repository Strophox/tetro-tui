use std::process::{Command, Stdio};

#[derive(
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    Copy,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    Default,
)]
pub enum ThemeSong {
    #[default]
    #[serde(alias = "KorobeinikiA", alias = "KorobeinikiB")]
    Korobeiniki,
}

#[derive(
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    Copy,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    Default,
)]
pub enum AudioBackend {
    #[default]
    Auto,
    PcSpeakerBeep,
    SoundCardMidi,
    SoundCardSox,
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
    pub backend: AudioBackend,
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
            enabled: any_audio_backend_is_available(),
            theme_enabled: true,
            sfx_enabled: true,
            theme_song: ThemeSong::Korobeiniki,
            backend: AudioBackend::Auto,
            sfx_pack: SfxPack::Classic,
            theme_tempo_percent: 100,
            keypress_sfx: true,
            piece_lock_sfx: true,
            line_clear_sfx: true,
            game_over_sfx: true,
        }
    }
}

fn any_audio_backend_is_available() -> bool {
    audio_backend_is_available(AudioBackend::PcSpeakerBeep)
        || audio_backend_is_available(AudioBackend::SoundCardMidi)
        || audio_backend_is_available(AudioBackend::SoundCardSox)
}

pub fn audio_backend_is_available(backend: AudioBackend) -> bool {
    match backend {
        AudioBackend::Auto => any_audio_backend_is_available(),
        AudioBackend::PcSpeakerBeep => command_is_available("beep", "-h"),
        AudioBackend::SoundCardMidi => command_is_available("timidity", "--help"),
        AudioBackend::SoundCardSox => command_is_available("sox", "--help"),
    }
}

fn command_is_available(command: &str, probe_arg: &str) -> bool {
    Command::new(command)
        .arg(probe_arg)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}
