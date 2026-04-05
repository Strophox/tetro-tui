mod menus;
mod savefile_load_store;

use std::{
    fmt::Debug,
    io::{self, Write},
    num::{NonZeroU32, NonZeroUsize},
    path::PathBuf,
    time::Duration,
};

use crossterm::{
    cursor::{self, MoveTo},
    event::{KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    terminal::{self, Clear, ClearType},
    ExecutableCommand,
};

use falling_tetromino_engine::{
    Button, DelayParameters, ExtDuration, Game, GameBuilder, GameEndCause, InGameTime, Input,
    Notification, NotificationFeed, NotificationLevel, Stat, Tetromino,
};

use crate::{
    application::{
        menus::{Menu, MenuUpdate},
        savefile_load_store::SavefileGranularity,
    },
    fmt_helpers::arabic_to_roman,
    game_keybinds::*,
    game_modes::{self, game_modifiers, GameMode},
    gameplay_settings::*,
    graphics_settings::*,
    palette::*,
};

/// This struct allows storing 'slots' (elements of some kind), where a certain
/// number of elements is considere as 'unmodifiable' (should not be modified)
/// but can be automatically cloned to a new slot and then modified for ease of use.
#[derive(PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SlotMachine<T> {
    /// The number of slots considered unmodifiable.
    unmodifiable: usize,
    slots: Vec<(String, T)>,
    // The string that is used as base to generate a name for duplicate slots.
    clone_name_template: String,
}

impl<T: Clone> SlotMachine<T> {
    pub fn with_unmodifiable_slots(
        slots: Vec<(String, T)>,
        cloned_slot_name_template: String,
    ) -> Self {
        let num_unmodifiable_slots = slots.len();
        Self {
            slots,
            unmodifiable: num_unmodifiable_slots,
            clone_name_template: cloned_slot_name_template,
        }
    }

    /// Given a valid index, clones and appends to itself of the corresponding slot if it is considered unmodifiable.
    /// Otherwise return `None` and does nothing (i.e. slot is 'modifiable' or index invalid).
    pub fn clone_slot_if_unmodifiable(&mut self, slot_idx: usize) -> Option<usize> {
        slot_idx.lt(&self.unmodifiable).then(|| {
            let cloned_slot_content = self.slots[slot_idx].1.clone();

            let mut n = 1;
            let cloned_slot_name = loop {
                let name = format!("{} {}", self.clone_name_template, arabic_to_roman(n));
                if self.slots.iter().all(|s| s.0 != name) {
                    break name;
                }
                n += 1;
            };

            self.slots.push((cloned_slot_name, cloned_slot_content));

            self.slots.len() - 1
        })
    }
}

/// Raw, uncompressed representation of a partial or complete input history.
///
/// We normally presuppose this is sorted by timestamps.
pub type UncompressedInputHistory = Vec<(InGameTime, Input)>;

/// Compressed verson of an input history.
///
/// Currently done using deltatime and assumption that inputs have millisecond precision at most.
#[derive(
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    Debug,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct CompressedInputHistory {
    inputbuf: Vec<u128>,
}

impl CompressedInputHistory {
    // How many bits it takes to encode a `ButtonChange`:
    // - 1 bit for Press/Release,
    // - At time of writing: 4 bits for the 11 `Button` variants.
    pub const BUTTON_CHANGE_BITSIZE: usize =
        1 + Button::VARIANTS.len().next_power_of_two().ilog2() as usize;

    pub fn new(game_input_history: &UncompressedInputHistory) -> Self {
        let mut inputbuf = Vec::new();

        let mut update_time_0 = InGameTime::ZERO;

        for (update_time_1, button_change) in game_input_history.iter() {
            let time_diff = update_time_1.saturating_sub(update_time_0);
            let i = Self::compress_input((time_diff, *button_change));

            // Add further input.
            inputbuf.push(i);

            update_time_0 = *update_time_1;
        }

        Self { inputbuf }
    }

    pub fn decompress(&self) -> UncompressedInputHistory {
        let mut decompressed_inputs = Vec::new();

        let mut update_time_0 = InGameTime::ZERO;
        for i in self.inputbuf.iter() {
            let (time_diff, button_change) = Self::decompress_input(*i);
            let update_time_1 = update_time_0.saturating_add(time_diff);

            // Add further input.
            decompressed_inputs.push((update_time_1, button_change));

            update_time_0 = update_time_1;
        }

        decompressed_inputs
    }

    // For serialization reasons, we encode a single user input as `u128` instead of
    // `(GameTime, ButtonChange)`, which would have a verbose direct string representation.
    fn compress_input((update_target_time, button_change): (InGameTime, Input)) -> u128 {
        // Encode `GameTime = std::time::Duration` using `std::time::Duration::as_millis`.
        // NOTE: We actually use `millis` not `nanos` as a convention which is upheld by `play_game.rs`!
        let millis: u128 = update_target_time.as_millis();
        // Encode `falling_tetromino_engine::ButtonChange` using `Self::encode_button_change`.
        let bc_bits: u8 = Self::compress_buttonchange(&button_change);
        (millis << Self::BUTTON_CHANGE_BITSIZE) | u128::from(bc_bits)
    }

    fn decompress_input(i: u128) -> (InGameTime, Input) {
        let mask = u128::MAX >> (128 - Self::BUTTON_CHANGE_BITSIZE);
        let bc_bits = u8::try_from(i & mask).unwrap();
        let millis = u64::try_from(i >> Self::BUTTON_CHANGE_BITSIZE).unwrap();
        (
            std::time::Duration::from_millis(millis),
            Self::decompress_buttonchange(bc_bits),
        )
    }

    fn compress_buttonchange(button_change: &Input) -> u8 {
        match button_change {
            Input::Deactivate(button) => (*button as u8) << 1,
            Input::Activate(button) => ((*button as u8) << 1) | 1,
        }
    }

    fn decompress_buttonchange(b: u8) -> Input {
        (if b.is_multiple_of(2) {
            Input::Deactivate
        } else {
            Input::Activate
        })(Button::VARIANTS[usize::from(b >> 1)])
    }
}

/// All the data required to functionally reconstruct gameplay.
#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct GameRestorationData<T> {
    builder: GameBuilder,
    mod_ids_args: Vec<(String, String)>,
    input_history: T,
    forfeit: Option<InGameTime>,
}

impl<T> GameRestorationData<T> {
    fn new(game: &Game, input_history: T, forfeit: Option<InGameTime>) -> GameRestorationData<T> {
        let (builder, mod_ids_args) = game.blueprint();

        GameRestorationData {
            builder,
            mod_ids_args,
            input_history,
            forfeit,
        }
    }

    fn map<U>(self, f: impl Fn(T) -> U) -> GameRestorationData<U> {
        GameRestorationData::<U> {
            builder: self.builder,
            mod_ids_args: self.mod_ids_args,
            input_history: f(self.input_history),
            forfeit: self.forfeit,
        }
    }
}

impl GameRestorationData<UncompressedInputHistory> {
    fn restore(&self, input_index: usize) -> Game {
        // Step 1: Prepare builder.
        let builder = self.builder.clone();
        // Step 2: Build actual game by possibly reconstructing mods to finalize builder with.
        let mut game = if self.mod_ids_args.is_empty() {
            builder.build()
        } else {
            match game_modes::game_modifiers::reconstruct_build_modded(&builder, &self.mod_ids_args)
            {
                Ok((mut modded_game, unrecognized_mod_ids)) => {
                    if !unrecognized_mod_ids.is_empty() {
                        // Add warning messages if certain mods could not be recognized.
                        // This should never happen in our application.
                        let warn_messages = unrecognized_mod_ids
                            .into_iter()
                            .map(|mod_desc| format!("WARNING: idk mod {mod_desc:?}"))
                            .collect();

                        let print_warn_msgs_mod =
                            game_modifiers::PrintMsgs::modifier(warn_messages);

                        modded_game.modifiers.push(print_warn_msgs_mod);
                    }

                    modded_game
                }
                Err(msg) => {
                    let error_messages = vec![format!("ERROR: {msg}")];

                    let print_error_msg_mod = game_modifiers::PrintMsgs::modifier(error_messages);

                    builder.build_modded(vec![print_error_msg_mod])
                }
            }
        };

        // Step 3: Reenact recorded game inputs.
        let restore_notification_level = game.config.notification_level;

        game.config.notification_level = NotificationLevel::Silent;
        for (update_time, button_change) in self.input_history.iter().take(input_index) {
            // FIXME: Handle UpdateGameError? If not, why not?
            let _v = game.update(*update_time, Some(*button_change));
        }

        game.config.notification_level = restore_notification_level;

        game
    }
}

/// Data associated with a Tetro TUI game.
#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct GameMetaData {
    pub datetime: String,
    pub title: String,
    pub comparison_stat: (Stat, bool),
}

// FIXME: Currently an ad-hoc struct to store game saves.
// The exact mechanism to ergonomically store and access several of these is subject to study.
#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct GameSave<T> {
    game_meta_data: GameMetaData,
    game_restoration_data: GameRestorationData<T>,
    inputs_to_load: usize,
}

impl<T> GameSave<T> {
    fn map<U>(self, f: impl Fn(T) -> U) -> GameSave<U> {
        GameSave {
            game_restoration_data: self.game_restoration_data.map(f),
            game_meta_data: self.game_meta_data,
            inputs_to_load: self.inputs_to_load,
        }
    }
}

/// An entry for the scoreboard. Store all the basic, cheap stats required for proper scoreboard entry display and sorting.
#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct ScoreEntry {
    game_meta_data: GameMetaData,
    end_cause: GameEndCause,
    is_win: bool,
    time_elapsed: InGameTime,
    lineclears: u32,
    points_scored: u32,
    pieces_locked: [u32; Tetromino::VARIANTS.len()],
    fall_delay_reached: ExtDuration,
    lock_delay_reached: Option<ExtDuration>,
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum ScoreEntrySorting {
    ModeDependent,
    Chronological,
    GameStat(Stat),
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct Scoreboard {
    entries: Vec<(
        ScoreEntry,
        Option<GameRestorationData<CompressedInputHistory>>,
    )>,
    sorting: ScoreEntrySorting,
}

impl Default for Scoreboard {
    fn default() -> Self {
        Self {
            sorting: ScoreEntrySorting::ModeDependent,
            entries: Vec::new(),
        }
    }
}

impl Scoreboard {
    fn sort(&mut self) {
        match self.sorting {
            ScoreEntrySorting::Chronological => self.sort_chronologically(),
            ScoreEntrySorting::ModeDependent => self.sort_semantically(),
            ScoreEntrySorting::GameStat(stat) => self.sort_by_stat(stat),
        }
    }

    fn sort_chronologically(&mut self) {
        self.entries.sort_by(|(pg1, _), (pg2, _)| {
            pg1.game_meta_data
                .datetime
                .cmp(&pg2.game_meta_data.datetime)
                .reverse()
        });
    }

    #[rustfmt::skip]
    fn sort_semantically(&mut self) {
        self.entries.sort_by(|(pg1, _), (pg2, _)|
            // Sort by gamemode (name).
            pg1.game_meta_data.title.cmp(&pg2.game_meta_data.title).then_with(||
            // Sort by if gamemode was finished successfully.
            pg1.is_win.cmp(&pg2.is_win).reverse().then_with(|| {
                // Sort by comparison stat...
                let o = match pg1.game_meta_data.comparison_stat.0 {
                    Stat::TimeElapsed(_)    => pg1.time_elapsed.cmp(&pg2.time_elapsed),
                    Stat::PiecesLocked(_)   => pg1.pieces_locked.cmp(&pg2.pieces_locked),
                    Stat::LinesCleared(_)   => pg1.lineclears.cmp(&pg2.lineclears),
                    Stat::PointsScored(_)   => pg1.points_scored.cmp(&pg2.points_scored),
                };
                // Comparison stat is used positively/negatively (minimize or maximize) depending on
                // how comparison stat compares to 'most important'(??) (often sole) end condition.
                // This is shady, but the special order we subtly chose and never publicly document
                // makes this make sense...
                if pg1.game_meta_data.comparison_stat.1
                    { o } else { o.reverse() }
            })
            )
        );
    }

    fn sort_by_stat(&mut self, stat: Stat) {
        self.entries.sort_by(|(pg1, _), (pg2, _)| match stat {
            Stat::TimeElapsed(_) => pg1.time_elapsed.cmp(&pg2.time_elapsed),
            Stat::PiecesLocked(_) => pg1.pieces_locked.cmp(&pg2.pieces_locked),
            Stat::LinesCleared(_) => pg1.lineclears.cmp(&pg2.lineclears),
            Stat::PointsScored(_) => pg1.points_scored.cmp(&pg2.points_scored),
        });
    }
}

#[derive(
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Clone,
    Debug,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct Statistics {
    total_new_games: u32,
    total_games_ended: u32,
    total_play_time: Duration,
    total_pieces_locked: u32,
    total_points_scored: u32,
    total_lines_cleared: u32,
    total_mono: u32,
    total_duo: u32,
    total_tri: u32,
    total_tetra: u32,
    total_spin: u32,
    total_perfect_clear: u32,
    total_combo: u32,
}

impl Statistics {
    // This simple blacklist is used to prevent certain game modes from being counted toward stats (e.g. Puzzle's perfect clears).
    const BLACKLIST_TITLE_PREFIXES: &[&str] = &[GameMode::TITLE_PUZZLE, GameMode::TITLE_COMBO];

    fn accumulate_from_feed(&mut self, feed: &NotificationFeed) {
        for (notification, _notif_time) in feed {
            match notification {
                Notification::PieceLocked { .. } => {
                    self.total_pieces_locked += 1;
                }

                Notification::Accolade {
                    points_bonus,
                    lineclears,
                    combo,
                    is_spin,
                    is_perfect_clear,
                    tetromino: _,
                } => {
                    self.total_points_scored += points_bonus;
                    self.total_lines_cleared += lineclears;
                    match lineclears {
                        1 => self.total_mono += 1,
                        2 => self.total_duo += 1,
                        3 => self.total_tri += 1,
                        4 => self.total_tetra += 1,
                        _ => {}
                    }
                    self.total_spin += if *is_spin { 1 } else { 0 };
                    self.total_perfect_clear += if *is_perfect_clear { 1 } else { 0 };
                    self.total_combo += if *combo > 1 { 1 } else { 0 };
                }

                _ => {}
            }
        }
    }

    fn accumulate(&mut self, other: &Statistics) {
        let Statistics {
            total_new_games: total_new_games_started,
            total_games_ended,
            total_play_time,
            total_pieces_locked,
            total_points_scored,
            total_lines_cleared,
            total_mono,
            total_duo,
            total_tri,
            total_tetra,
            total_spin,
            total_perfect_clear,
            total_combo,
        } = self;

        *total_new_games_started += other.total_new_games;
        *total_games_ended += other.total_games_ended;
        *total_play_time += other.total_play_time;
        *total_pieces_locked += other.total_pieces_locked;
        *total_points_scored += other.total_points_scored;
        *total_lines_cleared += other.total_lines_cleared;
        *total_mono += other.total_mono;
        *total_duo += other.total_duo;
        *total_tri += other.total_tri;
        *total_tetra += other.total_tetra;
        *total_spin += other.total_spin;
        *total_perfect_clear += other.total_perfect_clear;
        *total_combo += other.total_combo;
    }
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct NewGameSettings {
    custom_fall_delay_params: DelayParameters,
    custom_win_condition: Option<Stat>,
    custom_seed: Option<u64>,
    custom_encoded_board: Option<String>, // For more compact serialization of NewGameSettings, we store an encoded `Board` (see `encode_board`).

    cheese_tiles_per_line: NonZeroUsize,
    cheese_fall_lock_delays: (ExtDuration, ExtDuration),
    cheese_limit: Option<NonZeroU32>,

    combo_limit: Option<NonZeroU32>,
    /// Custom starting layout when playing Combo mode (4-wide rows), encoded as binary.
    /// Example: '▀▄▄▀' => 0b_1001_0110 = 150
    combo_initial_layout: u16,

    master_mode_unlocked: bool,
    experimental_mode_unlocked: bool,
}

impl Default for NewGameSettings {
    fn default() -> Self {
        Self {
            custom_fall_delay_params: DelayParameters::standard_fall(),
            custom_win_condition: None,
            custom_seed: None,
            custom_encoded_board: None,

            cheese_limit: Some(NonZeroU32::try_from(20).unwrap()),
            cheese_fall_lock_delays: (ExtDuration::Infinite, ExtDuration::Infinite),
            cheese_tiles_per_line: NonZeroUsize::new(Game::WIDTH - 1).unwrap(),

            combo_limit: Some(NonZeroU32::try_from(30).unwrap()),
            combo_initial_layout: game_modifiers::Combo::LAYOUTS[0],

            master_mode_unlocked: false,
            experimental_mode_unlocked: false,
        }
    }
}

// #[serde_with::serde_as]
#[derive(PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    newgame: NewGameSettings,
    graphics_pick: usize,
    keybinds_pick: usize,
    gameplay_pick: usize,

    graphics_slotmachine: SlotMachine<GraphicsSettings>,
    keybinds_slotmachine: SlotMachine<GameKeybinds>,
    gameplay_slotmachine: SlotMachine<GameplaySettings>,
    palette_slotmachine: SlotMachine<Palette>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            newgame: NewGameSettings::default(),
            graphics_pick: 0,
            keybinds_pick: 0,
            gameplay_pick: 0,
            graphics_slotmachine: default_graphics_slots(),
            palette_slotmachine: default_palette_slots(),
            keybinds_slotmachine: default_keybinds_slots(),
            gameplay_slotmachine: default_gameplay_slots(),
        }
    }
}

impl Settings {
    pub fn graphics(&self) -> &GraphicsSettings {
        &self.graphics_slotmachine.slots[self.graphics_pick].1
    }
    pub fn keybinds(&self) -> &GameKeybinds {
        &self.keybinds_slotmachine.slots[self.keybinds_pick].1
    }
    pub fn gameplay(&self) -> &GameplaySettings {
        &self.gameplay_slotmachine.slots[self.gameplay_pick].1
    }
    fn graphics_mut(&mut self) -> &mut GraphicsSettings {
        &mut self.graphics_slotmachine.slots[self.graphics_pick].1
    }
    fn keybinds_mut(&mut self) -> &mut GameKeybinds {
        &mut self.keybinds_slotmachine.slots[self.keybinds_pick].1
    }
    fn gameplay_mut(&mut self) -> &mut GameplaySettings {
        &mut self.gameplay_slotmachine.slots[self.gameplay_pick].1
    }

    pub fn palette(&self) -> &Palette {
        &self.palette_slotmachine.slots[self.graphics().palette_pick].1
    }
    pub fn palette_lockedtiles(&self) -> &Palette {
        &self.palette_slotmachine.slots[self.graphics().lockpalette_pick].1
    }
}

#[derive(Debug)]
pub struct TemporaryAppData {
    pub custom_terminal_state_initialized: bool,
    pub kitty_detected: bool,
    pub kitty_assumed: bool,
    pub blindfold_enabled: bool,
    pub renderernumber: usize,
    pub save_on_exit: SavefileGranularity,
    pub savefile_path: PathBuf, // This should technically be the same for a given compiled binary, but we compute it at runtime.
    pub loadfile_result: io::Result<()>,
}

// FIXME: Move tui application into `main` instead of artifically having it in one module below `tetro-tui::main`?
#[derive(Debug)]
pub struct Application<T: Write> {
    term: T,

    temp_data: TemporaryAppData,

    settings: Settings,

    scores_and_replays: Scoreboard,

    statistics: Statistics,

    // FIXME: Currently one can only access one without resorting to manually editing the savefile.
    game_saves: (usize, Vec<GameSave<UncompressedInputHistory>>),
}

impl<T: Write> Drop for Application<T> {
    fn drop(&mut self) {
        // (Try to) undo terminal setup. Ignore errors cuz atp it's too late to take any flak from Crossterm.
        let _ = self.deinitialize_terminal_state();

        if self.temp_data.save_on_exit != SavefileGranularity::NoSavefile {
            // If the user wants any of their data stored, try to do so.
            if let Err(e) = self.store_to_savefile() {
                eprintln!("{e}");
            }
        } else if self
            .temp_data
            .savefile_path
            .try_exists()
            .is_ok_and(|exists| exists)
        {
            // Otherwise explicitly check for savefile and try to make sure we don't leave it around.
            if let Err(e) = std::fs::remove_file(self.temp_data.savefile_path.clone()) {
                eprintln!("{e}");
            }
        }
    }
}

impl<T: Write> Application<T> {
    // FIXME: What the... Maybe do less hardcoding here?
    // pub const W_MAIN: u16 = 80;
    // pub const H_MAIN: u16 = 24;
    pub const W_MAIN: u16 = 62;
    pub const H_MAIN: u16 = 23;

    pub const TERMINAL_TITLE: &str = "Tetro TUI";

    // FIXME: Could we get any undesirable results from pushing all() enhancement flags?
    pub const GAME_KEYBOARD_ENHANCEMENT_FLAGS: KeyboardEnhancementFlags =
        KeyboardEnhancementFlags::all();

    pub fn fetch_main_xy() -> (u16, u16) {
        let (w_console, h_console) = terminal::size().unwrap_or((0, 0));
        (
            w_console.saturating_sub(Self::W_MAIN) / 2,
            h_console.saturating_sub(Self::H_MAIN) / 2,
        )
    }

    fn initialize_terminal_state(&mut self) -> io::Result<()> {
        if !self.temp_data.custom_terminal_state_initialized {
            self.temp_data.custom_terminal_state_initialized = true;

            // 1. Enter alternate screen. This allows us not to trash the terminal's contents from before the app is run.
            self.term.execute(terminal::EnterAlternateScreen)?;

            // 2a. Enable raw input mode (no enter required to read keyboard input).
            terminal::enable_raw_mode()?;

            // 2b. Hide cursor.
            self.term.execute(cursor::Hide)?;

            // 2c. Set title.
            self.term
                .execute(terminal::SetTitle(Self::TERMINAL_TITLE))?;

            // 2d. For technical reasons we do not want default keyboard enhancement in the TUI's menus.
            // - Default enhancement trigger screen refreshes, discarding text selection and preventing Ctrl+Shift+C (copy, e.g. of savefile path in Advanced Settings menu).
            // - Enhancement-sensitive menus (e.g. game, replay, keybind settings) should set their own custom enhancement flags if applicable, so this should really only affect menus which rely on the "default" terminal enhancement state.
            // FIXME: Explicitly ignore an error when pushing flags. This is so we can still try even if Crossterm minds if we do this on Windows.
            let _v = self.term.execute(PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::empty(),
            ));
        }
        Ok(())
    }

    fn deinitialize_terminal_state(&mut self) -> io::Result<()> {
        if self.temp_data.custom_terminal_state_initialized {
            // (Try to) undo terminal setup.

            // 2d.
            // FIXME: Explicitly ignore an error when pushing flags. This is so we can still try even if Crossterm minds if we do this on Windows.
            let _v = self.term.execute(PopKeyboardEnhancementFlags);

            // 2b.
            self.term.execute(cursor::Show)?;

            // 2a.
            terminal::disable_raw_mode()?;

            // 1.
            self.term.execute(terminal::LeaveAlternateScreen)?;

            self.temp_data.custom_terminal_state_initialized = true;
        }

        Ok(())
    }

    pub fn with_savefile_and_cmdlineoptions(
        term: T,
        savefile_path: PathBuf,
        custom_start_seed: Option<u64>,
        custom_start_board: Option<String>,
    ) -> Self {
        // Now that the settings are loaded, we handle separate flags set for this session.
        let kitty_detected = terminal::supports_keyboard_enhancement().unwrap_or(false);

        let temp_data = TemporaryAppData {
            custom_terminal_state_initialized: false,
            kitty_detected,
            kitty_assumed: kitty_detected,
            blindfold_enabled: false,
            renderernumber: 0,
            save_on_exit: SavefileGranularity::default(),
            savefile_path,
            loadfile_result: Ok(()),
        };

        let mut new = Self {
            term,
            temp_data,
            settings: Settings::default(),
            scores_and_replays: Scoreboard::default(),
            game_saves: (0, Vec::new()),
            statistics: Statistics::default(),
        };

        // Load in actual settings.
        new.temp_data.loadfile_result = new.load_from_savefile();

        // Special: Overwrite specifically requested cmdline flags.

        if custom_start_board.is_some() {
            new.settings.newgame.custom_encoded_board = custom_start_board;
        }

        if custom_start_seed.is_some() {
            new.settings.newgame.custom_seed = custom_start_seed;
        }

        new
    }

    pub fn run(&mut self) -> io::Result<()> {
        // Console prologue: Initialization.
        // FIXME: Handle io::Error? If not, why not?
        let _e = self.initialize_terminal_state();

        let mut menu_stack = vec![Menu::Title];
        loop {
            // Retrieve active menu, stop application if stack is empty.
            let Some(menu) = menu_stack.last_mut() else {
                break;
            };
            // Open new menu screen, then store what it returns.
            let menu_update = match menu {
                Menu::Title => self.run_menu_title(),
                Menu::NewGame => self.run_menu_new_game(),
                Menu::PlayGame {
                    game,
                    game_input_history,
                    game_meta_data,
                    game_renderer,
                } => {
                    self.run_menu_play_game(game, game_input_history, game_meta_data, game_renderer)
                }
                Menu::Pause => self.run_menu_pause(),
                Menu::Settings => self.run_menu_settings(),
                Menu::AdjustGraphics => self.run_menu_adjust_graphics(),
                Menu::AdjustKeybinds => self.run_menu_adjust_keybinds(),
                Menu::AdjustGameplay => self.run_menu_adjust_gameplay(),
                Menu::AdvancedSettings => self.run_menu_advanced_settings(),
                Menu::GameOver { game_scoring } => self.run_menu_game_ended(game_scoring),
                Menu::GameComplete { game_scoring } => self.run_menu_game_ended(game_scoring),
                Menu::ScoresAndReplays {
                    cursor_pos,
                    camera_pos,
                } => self.run_menu_scores_and_replays(cursor_pos, camera_pos),
                Menu::ReplayGame {
                    game_restoration_data,
                    game_meta_data,
                    replay_length,
                    game_renderer,
                } => self.run_menu_replay_game(
                    game_restoration_data,
                    game_meta_data,
                    *replay_length,
                    game_renderer.as_mut(),
                ),
                Menu::Statistics => self.run_menu_statistics(),
                Menu::About => self.run_menu_about(),
                Menu::Quit => break,
            }?;

            // Change screen session depending on what response screen gave.
            match menu_update {
                MenuUpdate::Pop => {
                    if menu_stack.len() > 1 {
                        menu_stack.pop();

                        // FIXME: Unused exit menu transition "DIAG".
                        // let (x_main, y_main) = Self::fetch_main_xy();
                        // /*
                        //  0      /1   /2   /3
                        // 0,0  1,0  2,0  3,0
                        // 0,1  1,1  2,1  3,1/-4
                        // 0,2  1,2  2,2  3,2/-5
                        // 0,3  1,3  2,3  3,3/-6
                        //  */
                        // for xplusy in 0 ..= (Self::W_MAIN/2).saturating_sub(1) + (Self::H_MAIN).saturating_sub(1) {
                        //     for x in xplusy.saturating_sub((Self::H_MAIN).saturating_sub(1)) ..= (Self::W_MAIN/2).saturating_sub(1) {
                        //         let y = xplusy.saturating_sub(x);
                        //         self.term
                        //             .queue(MoveTo(x_main + 2 * x, y_main + y))?
                        //             .queue(Print("  "))?;
                        //     }
                        //     self.term.flush()?;
                        //     std::thread::sleep(Duration::from_secs_f32(1./120.0));
                        // }

                        // FIXME: Unused exit menu transition "SIDE".
                        // let (x_main, y_main) = Self::fetch_main_xy();
                        // for x in 0..Self::W_MAIN/2 {
                        //     for y in 0..Self::H_MAIN {
                        //         self.term
                        //             .queue(MoveTo(x_main + 2 * x, y_main + y))?
                        //             .queue(Print("  "))?;
                        //     }
                        //     self.term.flush()?;
                        //     std::thread::sleep(Duration::from_secs_f32(1./240.0));
                        // }
                    }
                }
                MenuUpdate::Push(menu) => {
                    if matches!(menu, Menu::Quit) {
                        break;
                    }

                    if matches!(
                        menu,
                        Menu::Title
                            | Menu::PlayGame { .. }
                            | Menu::GameOver { .. }
                            | Menu::GameComplete { .. }
                    ) {
                        menu_stack.clear();
                    }

                    if matches!(menu, Menu::GameOver { .. }) {
                        let h_console = terminal::size()?.1;
                        for y in (0..h_console).rev() {
                            self.term
                                .execute(MoveTo(0, y))?
                                .execute(Clear(ClearType::CurrentLine))?;
                            std::thread::sleep(Duration::from_secs_f32(1. / 60.0));
                        }
                    } else if matches!(menu, Menu::GameComplete { .. }) {
                        let h_console = terminal::size()?.1;
                        for y in 0..h_console {
                            self.term
                                .execute(MoveTo(0, y))?
                                .execute(Clear(ClearType::CurrentLine))?;
                            std::thread::sleep(Duration::from_secs_f32(1. / 60.0));
                        }
                    } else {
                        // FIXME: Unused enter menu transition "DIAG".
                        // let (x_main, y_main) = Self::fetch_main_xy();
                        // /*
                        //  0      /1   /2   /3
                        // 0,0  1,0  2,0  3,0
                        // 0,1  1,1  2,1  3,1/-4
                        // 0,2  1,2  2,2  3,2/-5
                        // 0,3  1,3  2,3  3,3/-6
                        //  */
                        // for xplusy in (0 ..= (Self::W_MAIN/2).saturating_sub(1) + (Self::H_MAIN).saturating_sub(1)).rev() {
                        //     for x in xplusy.saturating_sub((Self::H_MAIN).saturating_sub(1)) ..= (Self::W_MAIN/2).saturating_sub(1) {
                        //         let y = xplusy.saturating_sub(x);
                        //         self.term
                        //             .queue(MoveTo(x_main + 2 * x, y_main + y))?
                        //             .queue(Print("  "))?;
                        //     }
                        //     self.term.flush()?;
                        //     std::thread::sleep(Duration::from_secs_f32(1./120.0));
                        // }

                        // FIXME: Unused enter menu transition "SIDE".
                        // let (x_main, y_main) = Self::fetch_main_xy();
                        // for x in (0..Self::W_MAIN/2).rev() {
                        //     for y in 0..Self::H_MAIN {
                        //         self.term
                        //             .queue(MoveTo(x_main + 2 * x, y_main + y))?
                        //             .queue(Print("  "))?;
                        //     }
                        //     self.term.flush()?;
                        //     std::thread::sleep(Duration::from_secs_f32(1./240.0));
                        // }
                    }

                    menu_stack.push(menu);
                }
            }
        }

        // Console epilogue: Deinitialization.
        self.deinitialize_terminal_state()
    }
}
