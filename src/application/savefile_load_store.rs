use std::{
    fs::File,
    io::{self, Read, Write},
};

use crate::application::{Application, CompressedInputHistory, GameSave};

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

impl<T: Write> Application<T> {
    pub fn load_from_savefile(&mut self) -> io::Result<()> {
        let mut file = File::open(self.temp_data.savefile_path.clone())?;
        let mut save_str = String::new();
        file.read_to_string(&mut save_str)?;

        // Make sure no field is forgotten by explicitly unpacking.
        let Application {
            term: _,
            temp_data,
            settings,
            scores_and_replays,
            statistics,
            game_saves,
        } = self;
        let compressed_game_saves: (usize, Vec<GameSave<CompressedInputHistory>>);

        (
            temp_data.save_on_exit,
            *settings,
            *scores_and_replays,
            *statistics,
            compressed_game_saves,
        ) = serde_json::from_str(&save_str)?;

        *game_saves = (
            compressed_game_saves.0,
            compressed_game_saves
                .1
                .into_iter()
                .map(|save| save.map(|input_history| input_history.decompress()))
                .collect::<Vec<_>>(),
        );

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

        let compressed_game_saves = (
            self.game_saves.0,
            self.game_saves
                .1
                .iter()
                .cloned()
                .map(|save| save.map(|input_history| CompressedInputHistory::new(&input_history)))
                .collect::<Vec<_>>(),
        );

        let save_str = serde_json::to_string(&(
            self.temp_data.save_on_exit,
            &self.settings,
            &self.scores_and_replays,
            &self.statistics,
            compressed_game_saves,
        ))?;

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
