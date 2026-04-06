use std::{
    borrow::Cow,
    fs::File,
    io::{self, Read, Write},
    path::PathBuf,
};

use crate::{
    game_restoration::RawInputHistory, settings::Settings, Application, CompressedInputHistory,
    GameSave, GameSaves, Scoreboard, Statistics,
};

pub fn savefile_name() -> String {
    format!(".tetro-tui_v{}_savefile.json", crate::VERSION_MAJOR_MINOR)
}

pub fn savefile_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(savefile_name())
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
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum SavefileGranularity {
    #[default]
    NoSavefile,
    RememberSettings,
    RememberSettingsScores,
    RememberSettingsScoresReplays,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct SavefileContents<'a> {
    save_on_exit: SavefileGranularity,
    statistics: Cow<'a, Statistics>,
    game_saves: GameSaves<CompressedInputHistory>,
    settings: Cow<'a, Settings>,
    scores_and_replays: Cow<'a, Scoreboard>,
}

impl<T: Write> Application<T> {
    pub fn load_from_savefile(&mut self) -> io::Result<()> {
        let mut file = File::open(self.temp_data.savefile_path.clone())?;
        let mut save_str = String::new();
        file.read_to_string(&mut save_str)?;

        let save_loaded: SavefileContents = serde_json::from_str(&save_str)?;

        // Make sure no field is forgotten by explicitly unpacking.
        let Application {
            term: _,
            temp_data,
            settings,
            scores_and_replays,
            statistics,
            game_saves,
        } = self;

        temp_data.save_on_exit = save_loaded.save_on_exit;
        *settings = save_loaded.settings.into_owned();
        *scores_and_replays = save_loaded.scores_and_replays.into_owned();
        *statistics = save_loaded.statistics.into_owned();
        game_saves.slots = save_loaded
            .game_saves
            .slots
            .into_iter()
            .filter_map(|save| save.decompress())
            .collect::<Vec<GameSave<RawInputHistory>>>();
        game_saves.pick = save_loaded
            .game_saves
            .pick
            .min(game_saves.slots.len().saturating_sub(1));

        Ok(())
    }

    pub fn store_to_savefile(&mut self) -> io::Result<()> {
        if self.temp_data.save_on_exit < SavefileGranularity::RememberSettingsScores {
            // Clear scoreboard if no game data is wished to be stored.
            self.scores_and_replays.entries.clear();
        } else if self.temp_data.save_on_exit < SavefileGranularity::RememberSettingsScoresReplays {
            // Clear past game inputs if no game input data is wished to be stored.
            for (_entry, restoration_data) in &mut self.scores_and_replays.entries {
                restoration_data.take();
            }
        }

        let compressed_game_saves = GameSaves {
            pick: self.game_saves.pick,
            slots: self
                .game_saves
                .slots
                .iter()
                .cloned()
                .map(|save| save.compress())
                .collect::<Vec<_>>(),
        };

        let savefile_contents = SavefileContents {
            save_on_exit: self.temp_data.save_on_exit,
            settings: Cow::Borrowed(&self.settings),
            scores_and_replays: Cow::Borrowed(&self.scores_and_replays),
            statistics: Cow::Borrowed(&self.statistics),
            game_saves: compressed_game_saves,
        };

        let save_str = serde_json::to_string(&savefile_contents)?;

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
