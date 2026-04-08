use std::{
    borrow::Cow,
    fs::File,
    io::{self, Read, Write},
    path::PathBuf,
};

use crate::{
    settings::Settings, Application, EncodedInputHistory, GameSave, GameSaves, RawInputHistory,
    Scoreboard, Statistics,
};

pub fn savefile_name() -> String {
    format!(".tetro-tui_v{}_savefile.json", crate::VERSION_MAJOR_MINOR)
}

pub fn savefile_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(savefile_name())
}

// FIXME: Use biflags instead? C.f. https://docs.rs/crossterm/0.29.0/src/crossterm/event.rs.html#832-849
#[derive(
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    Copy,
    Debug,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum SavefileGranularity {
    #[default]
    NoSavefile,
    StoreSettings,
    StoreSettingsScores,
    StoreSettingsScoresReplays,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SaveContents<'a> {
    #[serde(rename = "save_on_exit")]
    save_on_exit: SavefileGranularity,

    #[serde(rename = "SETTINGS")]
    settings: Cow<'a, Settings>,

    #[serde(rename = "STATISTICS")]
    statistics: Cow<'a, Statistics>,

    #[serde(rename = "GAME_SAVE_SLOTS")]
    compressed_game_saves: GameSaves<EncodedInputHistory>,

    #[serde(rename = "SCORES_AND_REPLAYS")]
    scores_and_replays: Cow<'a, Scoreboard>,
}

impl<T: Write> Application<T> {
    pub fn load_from_savefile(&mut self) -> io::Result<()> {
        let mut file = File::open(self.temp_data.savefile_path.clone())?;
        let mut save_str = String::new();
        file.read_to_string(&mut save_str)?;

        let save_contents: SaveContents = serde_json::from_str(&save_str)?;

        // Make sure no field is forgotten by explicitly unpacking.
        let Application {
            term: _,
            temp_data,
            settings,
            scores_and_replays,
            statistics,
            game_saves,
        } = self;

        temp_data.save_on_exit = save_contents.save_on_exit;
        *settings = save_contents.settings.into_owned();
        *scores_and_replays = save_contents.scores_and_replays.into_owned();
        *statistics = save_contents.statistics.into_owned();
        // FIXME: Improve error handling by actually displaying the decompression errors somewhere instead of gobbling them with `.ok()`.
        game_saves.slots = save_contents
            .compressed_game_saves
            .slots
            .into_iter()
            .filter_map(|save| save.try_decode().ok())
            .collect::<Vec<GameSave<RawInputHistory>>>();
        game_saves.picked = save_contents
            .compressed_game_saves
            .picked
            .min(game_saves.slots.len().saturating_sub(1));

        Ok(())
    }

    pub fn store_to_savefile(&mut self) -> io::Result<()> {
        if self.temp_data.save_on_exit < SavefileGranularity::StoreSettingsScores {
            // Clear scoreboard if no game data is wished to be stored.
            self.scores_and_replays.entries.clear();
        } else if self.temp_data.save_on_exit < SavefileGranularity::StoreSettingsScoresReplays {
            // Clear past game restoration data if no game replay data is wished to be stored.
            for (_entry, restoration_data) in &mut self.scores_and_replays.entries {
                restoration_data.take();
            }
        }

        let save_contents = SaveContents {
            save_on_exit: self.temp_data.save_on_exit,
            settings: Cow::Borrowed(&self.settings),
            scores_and_replays: Cow::Borrowed(&self.scores_and_replays),
            statistics: Cow::Borrowed(&self.statistics),
            compressed_game_saves: GameSaves {
                picked: self.game_saves.picked,
                slots: self
                    .game_saves
                    .slots
                    .iter()
                    .cloned()
                    .map(|save| save.compress())
                    .collect::<Vec<_>>(),
            },
        };

        let save_str = serde_json::to_string(&save_contents)?;

        let mut file = File::create(self.temp_data.savefile_path.clone())?;
        let n_written = file.write(save_str.as_bytes())?;
        // Attempt at additionally handling the case when save_str could not be written entirely.
        if n_written < save_str.len() {
            Err(std::io::Error::other(
                "attempt to write to file consumed `n < save_str.len()` bytes",
            ))
        } else {
            Ok(())
        }
    }
}
