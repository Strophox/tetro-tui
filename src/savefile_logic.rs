use std::{
    borrow::Cow,
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

use crate::{
    tui_settings::Settings, Application, GameSave, GameSaves, RawInputHistory, Scoreboard,
    Statistics,
};

pub type SavefileResult<T> = Result<T, Box<dyn std::error::Error>>;

const SAVEFILE_EXTENSION: &str = "rson";

// Custom string data format used for savefile and crate as a whole.
pub fn to_savefile_string<T: serde::Serialize>(value: &T) -> SavefileResult<String> {
    // let string = serde_json::to_string(value)?; // Cannot de-/serialize f64::inf. WARNING: Requires reactivation of usage of serde_with::json::JsonString in serialized HashMaps/BTreeMaps!
    // let string = json5::to_string(value)?; // Verbose, meant for configuration files, not compact storage.
    // let string = toml::to_string(value)?; // "unsupported None value"...
    // let string = serde_yaml::to_string(value)?; // https://ruudvanasseldonk.com/2023/01/11/the-yaml-document-from-hell
    let string = ron::to_string(value)?;
    Ok(string)
}

// Custom string data format used for savefile and crate as a whole.
pub fn from_savefile_str<'de, T: serde::Deserialize<'de>>(input: &'de str) -> SavefileResult<T> {
    // let value = serde_json::from_str(input)?;
    // let value = json5::from_str(input)?;
    // let value = toml::from_str(input)?;
    // let value = serde_yaml::from_str(input)?;
    let value = ron::from_str(input)?;
    Ok(value)
}

pub fn savefile_name() -> String {
    format!(
        ".tetro-tui_v{}_savefile.{SAVEFILE_EXTENSION}",
        crate::VERSION_MAJOR_MINOR
    )
}

pub fn savefile_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(savefile_name())
}

// NOTE: We could consider using bitflags.
// But right now there are some dependencies, e.g., cannot store replay without scoreboard.
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
    #[serde(rename = "SAVEFILE_ON_EXIT")]
    save_on_exit: SavefileGranularity,

    #[serde(rename = "SETTINGS")]
    settings: Cow<'a, Settings>,

    #[serde(rename = "ALL_TIME_STATISTICS")]
    statistics: Cow<'a, Statistics>,

    #[serde(rename = "GAME_SAVE_SLOTS")]
    compressed_game_saves: GameSaves<
        crate::game_restoration::deltas_bitencoded_base64::DeltasBitencodedBase64InputHistory,
    >,

    #[serde(rename = "SCORES_AND_REPLAYS")]
    scores_and_replays: Cow<'a, Scoreboard>,
}

impl<T: Write> Application<T> {
    pub fn savefile_store(&mut self) -> SavefileResult<()> {
        match self.temp_data.save_on_exit {
            // Explicitly check for savefile and try to make sure we don't leave it around.
            SavefileGranularity::NoSavefile => {
                if self
                    .temp_data
                    .savefile_path
                    .try_exists()
                    .is_ok_and(|exists| exists)
                {
                    std::fs::remove_file(self.temp_data.savefile_path.clone())?;
                    return Ok(());
                }
            }

            // Clear scoreboard if no data other than settings is wished to be stored.
            SavefileGranularity::StoreSettings => {
                self.scores_and_replays.entries.clear();
            }

            // Clear past game restoration data if no game replay data is wished to be stored.
            SavefileGranularity::StoreSettingsScores => {
                for (_entry, restoration_data) in &mut self.scores_and_replays.entries {
                    restoration_data.take();
                }
            }

            // Everything to be stored. Fall through.
            SavefileGranularity::StoreSettingsScoresReplays => {}
        }

        let save_contents = SaveContents {
            save_on_exit: self.temp_data.save_on_exit,
            settings: Cow::Borrowed(&self.settings),
            scores_and_replays: Cow::Borrowed(&self.scores_and_replays),
            statistics: Cow::Borrowed(&self.statistics),
            compressed_game_saves: GameSaves {
                selected: self.game_saves.selected,
                slots: self
                    .game_saves
                    .slots
                    .iter()
                    .cloned()
                    .map(|save| save.compress())
                    .collect::<Vec<_>>(),
            },
        };

        let save_str = to_savefile_string(&save_contents)?;

        let mut file = File::create(self.temp_data.savefile_path.clone())?;
        let n_written = file.write(save_str.as_bytes())?;
        // Attempt at additionally handling the case when save_str could not be written entirely.
        if n_written < save_str.len() {
            Err(Box::new(std::io::Error::other(
                "attempt to write to file consumed `n < save_str.len()` bytes",
            )))
        } else {
            Ok(())
        }
    }

    pub fn savefile_load(&mut self) -> SavefileResult<()> {
        let mut file = File::open(self.temp_data.savefile_path.clone())?;
        let mut save_str = String::new();
        file.read_to_string(&mut save_str)?;

        let save_contents: SaveContents = from_savefile_str(&save_str)?;

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
        // FIXME: Improve error handling by actually displaying the game save decompression errors somewhere instead of gobbling them with `.ok()`.
        game_saves.slots = save_contents
            .compressed_game_saves
            .slots
            .into_iter()
            .filter_map(|save| save.try_decode().ok())
            .collect::<Vec<GameSave<RawInputHistory>>>();
        game_saves.selected = save_contents
            .compressed_game_saves
            .selected
            .min(game_saves.slots.len().saturating_sub(1));

        Ok(())
    }
}
