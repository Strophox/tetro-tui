mod menus;

use std::{
    fmt::Debug,
    fs::File,
    io::{self, Read, Write},
    num::{NonZeroU32, NonZeroUsize},
    path::PathBuf,
    time::Duration,
};

use crossterm::{cursor, event::KeyboardEnhancementFlags, style, terminal, ExecutableCommand};

use falling_tetromino_engine::{
    Board, Button, ButtonChange, Configuration, DelayParameters, ExtDuration, ExtNonNegF64,
    Feedback, FeedbackVerbosity, Game, GameBuilder, GameOver, GameResult, InGameTime, Modifier,
    RotationSystem, Stat, Tetromino, TetrominoGenerator,
};

use crate::{game_mode_presets, game_renderers, keybinds_presets::*, palette_presets::*};

pub type Slots<T> = Vec<(String, T)>;

pub type UncompressedInputHistory = Vec<(InGameTime, ButtonChange)>;

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
pub struct CompressedInputHistory(Vec<u128>);

impl CompressedInputHistory {
    // How many bits it takes to encode a `ButtonChange`:
    // - 1 bit for Press/Release,
    // - At time of writing: 4 bits for the 11 `Button` variants.
    pub const BUTTON_CHANGE_BITSIZE: usize =
        1 + Button::VARIANTS.len().next_power_of_two().ilog2() as usize;

    pub fn new(game_input_history: &UncompressedInputHistory) -> Self {
        let mut compressed_inputs = Vec::new();

        if let Some((mut update_time_0, button_change)) = game_input_history.first() {
            let i = Self::compress_input((update_time_0, *button_change));

            // Add first input.
            compressed_inputs.push(i);

            for (update_time_1, button_change) in game_input_history.iter().skip(1) {
                let time_diff = update_time_1.saturating_sub(update_time_0);
                let i = Self::compress_input((time_diff, *button_change));

                // Add further input.
                compressed_inputs.push(i);

                update_time_0 = *update_time_1;
            }
        };

        Self(compressed_inputs)
    }

    pub fn decompress(&self) -> UncompressedInputHistory {
        let mut decompressed_inputs = Vec::new();

        if let Some(i) = self.0.first() {
            let (mut update_time_0, button_change) = Self::decompress_input(*i);

            // Add first input.
            decompressed_inputs.push((update_time_0, button_change));

            for i in self.0.iter().skip(1) {
                let (time_diff, button_change) = Self::decompress_input(*i);
                let update_time_1 = update_time_0.saturating_add(time_diff);

                // Add further input.
                decompressed_inputs.push((update_time_1, button_change));

                update_time_0 = update_time_1;
            }
        }

        decompressed_inputs
    }

    // For serialization reasons, we encode a single user input as `u128` instead of
    // `(GameTime, ButtonChange)`, which would have a verbose direct string representation.
    fn compress_input((update_target_time, button_change): (InGameTime, ButtonChange)) -> u128 {
        // Encode `GameTime = std::time::Duration` using `std::time::Duration::as_millis`.
        // NOTE: We actually use `millis` not `nanos` as a convention which is upheld by `play_game.rs`!
        let millis: u128 = update_target_time.as_millis();
        // Encode `falling_tetromino_engine::ButtonChange` using `Self::encode_button_change`.
        let bc_bits: u8 = Self::compress_buttonchange(&button_change);
        (millis << Self::BUTTON_CHANGE_BITSIZE) | u128::from(bc_bits)
    }

    fn decompress_input(i: u128) -> (InGameTime, ButtonChange) {
        let mask = u128::MAX >> (128 - Self::BUTTON_CHANGE_BITSIZE);
        let bc_bits = u8::try_from(i & mask).unwrap();
        let millis = u64::try_from(i >> Self::BUTTON_CHANGE_BITSIZE).unwrap();
        (
            std::time::Duration::from_millis(millis),
            Self::decompress_buttonchange(bc_bits),
        )
    }

    fn compress_buttonchange(button_change: &ButtonChange) -> u8 {
        match button_change {
            ButtonChange::Release(button) => (*button as u8) << 1,
            ButtonChange::Press(button) => ((*button as u8) << 1) | 1,
        }
    }

    fn decompress_buttonchange(b: u8) -> ButtonChange {
        (if b.is_multiple_of(2) {
            ButtonChange::Release
        } else {
            ButtonChange::Press
        })(Button::VARIANTS[usize::from(b >> 1)])
    }
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct GameRestorationData<T> {
    builder: GameBuilder,
    mod_descriptors: Vec<String>,
    input_history: T,
    forfeit: Option<InGameTime>,
}

impl<T> GameRestorationData<T> {
    fn new(game: &Game, input_history: T, forfeit: Option<InGameTime>) -> GameRestorationData<T> {
        let (builder, mod_descriptors) = game.blueprint();
        GameRestorationData {
            builder,
            mod_descriptors: mod_descriptors.map(str::to_owned).collect(),
            input_history,
            forfeit,
        }
    }

    fn map<U>(self, f: impl Fn(T) -> U) -> GameRestorationData<U> {
        GameRestorationData::<U> {
            builder: self.builder,
            mod_descriptors: self.mod_descriptors,
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
        let mut game = if self.mod_descriptors.is_empty() {
            builder.build()
        } else {
            match game_mode_presets::game_modifiers::reconstruct_build_modded(
                &builder,
                self.mod_descriptors.iter().map(String::as_str),
            ) {
                Ok((mut modded_game, unrecognized_mod_descriptors)) => {
                    #[rustfmt::skip]
                    let print_warn_msgs_mod = Modifier {
                        descriptor: "print_warn_msgs".to_owned(),
                        mod_function: Box::new({
                            let mut init = false;
                            move |_point, _config, _init_vals, state, _phase, msgs| {
                                if init { return; } else { init = true; }
                                for umd in unrecognized_mod_descriptors.iter() { msgs.push((state.time,Feedback::Text(format!("WARNING: Idk {umd:?}")))); }
                            }
                        }),
                    };

                    modded_game.modifiers.push(print_warn_msgs_mod);

                    modded_game
                }
                Err(msg) => {
                    #[rustfmt::skip]
                    let print_error_msg_mod = Modifier {
                        descriptor: "print_error_msg".to_owned(),
                        mod_function: Box::new({ let mut init = false;
                            move |_point, _config, _init_vals, state, _phase, msgs| {
                                if init { return; } else { init = true; }
                                msgs.push((state.time, Feedback::Text(format!("ERROR: {msg:?}"))));
                            }
                        }),
                    };

                    builder.build_modded([print_error_msg_mod])
                }
            }
        };

        // Step 3: Reenact recorded game inputs.
        let restore_feedback_verbosity = game.config.feedback_verbosity;

        game.config.feedback_verbosity = FeedbackVerbosity::Silent;
        for (update_time, button_change) in self.input_history.iter().take(input_index) {
            // FIXME: Handle UpdateGameError? If not, why not?
            let _v = game.update(*update_time, Some(*button_change));
        }

        game.config.feedback_verbosity = restore_feedback_verbosity;

        game
    }
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct GameMetaData {
    pub datetime: String,
    pub title: String,
    pub comparison_stat: (Stat, bool),
}

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

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct ScoresEntry {
    game_meta_data: GameMetaData,
    result: GameResult,
    time_elapsed: InGameTime,
    lineclears: u32,
    points_scored: u32,
    pieces_locked: [u32; Tetromino::VARIANTS.len()],
    fall_delay_reached: ExtDuration,
    lock_delay_reached: Option<ExtDuration>,
}

impl ScoresEntry {
    fn new(game: &Game, game_meta_data: &GameMetaData) -> ScoresEntry {
        ScoresEntry {
            game_meta_data: game_meta_data.clone(),
            time_elapsed: game.state().time,
            pieces_locked: game.state().pieces_locked,
            lineclears: game.state().lineclears,
            fall_delay_reached: game.state().fall_delay,
            lock_delay_reached: (game
                .state()
                .fall_delay_lowerbound_hit_at_n_lineclears
                .is_some()
                && !game.config.lock_delay_params.is_constant())
            .then_some(game.state().lock_delay),
            points_scored: game.state().score,
            result: game.result().unwrap_or(Err(GameOver::Forfeit)),
        }
    }
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
pub enum ScoresSorting {
    #[default]
    Chronological,
    Semantic,
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
pub struct ScoresAndReplays {
    sorting: ScoresSorting,
    entries: Vec<(
        ScoresEntry,
        Option<GameRestorationData<CompressedInputHistory>>,
    )>,
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct NewGameSettings {
    custom_fall_delay_params: DelayParameters,
    custom_win_condition: Option<Stat>,
    custom_seed: Option<u64>,
    custom_board: Option<String>, // For more compact serialization of NewGameSettings, we store an encoded `Board` (see `encode_board`).

    cheese_linelimit: Option<NonZeroU32>,
    cheese_fall_delay: ExtDuration,
    cheese_tiles_per_line: NonZeroUsize,

    combo_linelimit: Option<NonZeroU32>,
    /// Custom starting layout when playing Combo mode (4-wide rows), encoded as binary.
    /// Example: '▀▄▄▀' => 0b_1001_0110 = 150
    combo_startlayout: u16,

    experimental_mode_unlocked: bool,
}

impl Default for NewGameSettings {
    fn default() -> Self {
        Self {
            custom_fall_delay_params: DelayParameters::standard_fall(),
            custom_win_condition: None,
            custom_seed: None,
            custom_board: None,

            cheese_linelimit: Some(NonZeroU32::try_from(20).unwrap()),
            cheese_fall_delay: ExtDuration::Infinite,
            cheese_tiles_per_line: NonZeroUsize::new(Game::WIDTH - 1).unwrap(),

            combo_linelimit: Some(NonZeroU32::try_from(30).unwrap()),
            combo_startlayout: game_mode_presets::game_modifiers::combo_board::LAYOUTS[0],

            experimental_mode_unlocked: false,
        }
    }
}

impl NewGameSettings {
    #[allow(dead_code)]
    pub fn encode_board(board: &Board) -> String {
        board
            .iter()
            .map(|line| {
                line.iter()
                    .map(|tile| if tile.is_some() { 'X' } else { ' ' })
                    .collect::<String>()
            })
            .collect::<String>()
            .trim_end()
            .to_owned()
    }

    pub fn decode_board(board_str: &str) -> Board {
        let grey_tile = Some(std::num::NonZeroU8::try_from(254).unwrap());

        let mut new_board = Board::default();

        let mut chars = board_str.chars();

        for line in &mut new_board {
            for tile in line {
                for char in chars.by_ref() {
                    if char == ' ' {
                        // Space = empty tile.
                        *tile = None;
                        break;
                    } else if char == '\n' {
                        // Newline = ignore, stay at tile but move on to next char.
                        continue;
                    } else {
                        // Otherwise = filled tile.
                        *tile = grey_tile;
                        break;
                    }
                }
            }
        }

        new_board
    }
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
pub enum Glyphset {
    Electronika60,
    #[allow(clippy::upper_case_acronyms)]
    ASCII,
    #[default]
    Unicode,
}

#[derive(PartialEq, PartialOrd, Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct GraphicsSettings {
    palette_active: usize,
    palette_active_lockedtiles: usize,
    pub glyphset: Glyphset,
    pub show_effects: bool,
    pub blindfolded: bool,
    pub show_shadow_piece: bool,
    pub show_button_state: bool,
    game_fps: f64,
    show_fps: bool,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            glyphset: Glyphset::default(),
            palette_active: 3,
            palette_active_lockedtiles: 3,
            show_effects: true,
            blindfolded: false,
            show_shadow_piece: true,
            show_button_state: false,
            game_fps: 30.0,
            show_fps: false,
        }
    }
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct GameplaySettings {
    rotation_system: RotationSystem,
    tetromino_generator: TetrominoGenerator,
    piece_preview_count: usize,
    delayed_auto_shift: Duration,
    auto_repeat_rate: Duration,
    soft_drop_factor: ExtNonNegF64,
    line_clear_duration: Duration,
    spawn_delay: Duration,
    allow_prespawn_actions: bool,
}

impl Default for GameplaySettings {
    fn default() -> Self {
        let c = Configuration::default();
        Self {
            rotation_system: c.rotation_system,
            tetromino_generator: TetrominoGenerator::default(),
            piece_preview_count: c.piece_preview_count,
            delayed_auto_shift: c.delayed_auto_shift,
            auto_repeat_rate: c.auto_repeat_rate,
            soft_drop_factor: c.soft_drop_divisor,
            line_clear_duration: c.line_clear_duration,
            spawn_delay: c.spawn_delay,
            allow_prespawn_actions: c.allow_prespawn_actions,
        }
    }
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

#[serde_with::serde_as]
#[derive(PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Settings {
    graphics_slot_active: usize,
    keybinds_slot_active: usize,
    gameplay_slot_active: usize,
    graphics_slots_that_should_not_be_changed: usize,
    palette_slots_that_should_not_be_changed: usize,
    keybinds_slots_that_should_not_be_changed: usize,
    gameplay_slots_that_should_not_be_changed: usize,
    graphics_slots: Slots<GraphicsSettings>,
    palette_slots: Slots<Palette>,
    gameplay_slots: Slots<GameplaySettings>,
    // NOTE: Reconsider #[serde_as(as = "Vec<(_, std::collections::HashMap<serde_with::json::JsonString, _>)>")]
    #[serde_as(as = "Vec<(_, Vec<(_, _)>)>")]
    keybinds_slots: Slots<Keybinds>,
    new_game: NewGameSettings,
}

impl Default for Settings {
    fn default() -> Self {
        let graphics_slots = vec![
            ("Default".to_owned(), GraphicsSettings::default()),
            (
                "Extra Focused".to_owned(),
                GraphicsSettings {
                    palette_active: 2,
                    palette_active_lockedtiles: 0,
                    show_effects: false,
                    game_fps: 60.0,
                    ..GraphicsSettings::default()
                },
            ),
        ];
        let palette_slots = vec![
            ("Monochrome".to_owned(), monochrome_palette()), // NOTE: The slot at index 0 is the special 'monochrome'/no palette slot.
            ("16-color".to_owned(), color16_palette()),
            ("Fullcolor".to_owned(), fullcolor_palette()),
            ("Okpalette".to_owned(), oklch_palette()),
            ("Gruvbox".to_owned(), gruvbox_palette()),
            ("Solarized".to_owned(), solarized_palette()),
            ("Terafox".to_owned(), terafox_palette()),
            ("Fahrenheit".to_owned(), fahrenheit_palette()),
            ("The Matrix".to_owned(), the_matrix_palette()),
            ("Sequoia".to_owned(), sequoia_palette()),
        ];
        let keybinds_slots = vec![
            ("Default".to_owned(), tetro_default_keybinds()),
            ("Extra Finesse".to_owned(), tetro_finesse_keybinds()),
            ("Vim".to_owned(), vim_keybinds()),
            ("Guideline".to_owned(), guideline_keybinds()),
        ];
        let gameplay_slots = vec![
            ("Default".to_owned(), GameplaySettings::default()),
            (
                "Extra Finesse".to_owned(),
                GameplaySettings {
                    delayed_auto_shift: Duration::from_millis(110),
                    auto_repeat_rate: Duration::from_millis(0),
                    piece_preview_count: 9,
                    ..GameplaySettings::default()
                },
            ),
        ];
        Self {
            graphics_slot_active: 0,
            keybinds_slot_active: 0,
            gameplay_slot_active: 0,
            graphics_slots_that_should_not_be_changed: graphics_slots.len(),
            palette_slots_that_should_not_be_changed: palette_slots.len(),
            keybinds_slots_that_should_not_be_changed: keybinds_slots.len(),
            gameplay_slots_that_should_not_be_changed: gameplay_slots.len(),
            graphics_slots,
            palette_slots,
            keybinds_slots,
            gameplay_slots,
            new_game: NewGameSettings::default(),
        }
    }
}

impl Settings {
    pub fn graphics(&self) -> &GraphicsSettings {
        &self.graphics_slots[self.graphics_slot_active].1
    }
    pub fn keybinds(&self) -> &Keybinds {
        &self.keybinds_slots[self.keybinds_slot_active].1
    }
    pub fn gameplay(&self) -> &GameplaySettings {
        &self.gameplay_slots[self.gameplay_slot_active].1
    }
    fn graphics_mut(&mut self) -> &mut GraphicsSettings {
        &mut self.graphics_slots[self.graphics_slot_active].1
    }
    fn keybinds_mut(&mut self) -> &mut Keybinds {
        &mut self.keybinds_slots[self.keybinds_slot_active].1
    }
    fn gameplay_mut(&mut self) -> &mut GameplaySettings {
        &mut self.gameplay_slots[self.gameplay_slot_active].1
    }

    pub fn palette(&self) -> &Palette {
        &self.palette_slots[self.graphics().palette_active].1
    }
    pub fn palette_lockedtiles(&self) -> &Palette {
        &self.palette_slots[self.graphics().palette_active_lockedtiles].1
    }
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct RuntimeData {
    kitty_detected: bool,
    kitty_assumed: bool,
}

#[derive(Debug)]
enum Menu {
    Title,
    NewGame,
    PlayGame {
        game: Box<Game>,
        game_input_history: UncompressedInputHistory,
        game_meta_data: GameMetaData,
        game_renderer: Box<game_renderers::diff_print::DiffPrintRenderer>,
    },
    Pause,
    Settings,
    AdjustGraphics,
    AdjustKeybinds,
    AdjustGameplay,
    GameOver(Box<ScoresEntry>),
    GameComplete(Box<ScoresEntry>),
    ScoresAndReplays,
    ReplayGame {
        game_restoration_data: Box<GameRestorationData<UncompressedInputHistory>>,
        game_meta_data: GameMetaData,
        game_renderer: Box<game_renderers::diff_print::DiffPrintRenderer>,
    },
    About,
    Quit,
}

impl std::fmt::Display for Menu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Menu::Title => "Title Screen",
            Menu::NewGame => "New Game",
            Menu::PlayGame { game_meta_data, .. } => {
                &format!("Playing Game ({})", game_meta_data.title)
            }
            Menu::Pause => "Pause",
            Menu::Settings => "Settings",
            Menu::AdjustGraphics => "Adjust Graphics",
            Menu::AdjustKeybinds => "Adjust Keybinds",
            Menu::AdjustGameplay => "Adjust Gameplay",
            Menu::GameOver(_) => "Game Over",
            Menu::GameComplete(_) => "Game Completed",
            Menu::ScoresAndReplays => "Scores and Replays",
            Menu::ReplayGame { game_meta_data, .. } => {
                &format!("Replaying Game ({})", game_meta_data.title)
            }
            Menu::About => "About",
            Menu::Quit => "Quit",
        };
        write!(f, "{name}")
    }
}

#[derive(Debug)]
enum MenuUpdate {
    Pop,
    Push(Menu),
}

// FIXME: Move tui application into `main` instead of artifically having it in one module below `tetro-tui::main`.
#[derive(PartialEq, Clone, Debug)]
pub struct Application<T: Write> {
    runtime_data: RuntimeData,
    term: T,
    save_on_exit: SavefileGranularity,
    settings: Settings,
    scores_and_replays: ScoresAndReplays,
    game_saves: (usize, Vec<GameSave<UncompressedInputHistory>>),
}

impl<T: Write> Drop for Application<T> {
    fn drop(&mut self) {
        // (Try to) undo terminal setup.
        let _ = terminal::disable_raw_mode();
        let _ = self.term.execute(style::ResetColor);
        let _ = self.term.execute(cursor::Show);
        let _ = self.term.execute(terminal::LeaveAlternateScreen);

        // Save settings using file system.
        let savefile_path = Self::savefile_path();

        if self.save_on_exit != SavefileGranularity::NoSavefile {
            // If the user wants any of their data stored, try to do so.
            if let Err(e) = self.store_savefile(savefile_path) {
                eprintln!("{e}");
            }
        } else if savefile_path.try_exists().is_ok_and(|exists| exists) {
            // Otherwise explicitly check for savefile and try to make sure we don't leave it around.
            if let Err(e) = std::fs::remove_file(savefile_path) {
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

    pub const SAVEFILE_NAME: &'static str =
        concat!(".tetro-tui_", clap::crate_version!(), "_savefile.json");

    // FIXME: Could we ever get any undesirable results from pushing *all* enhancement flags?
    pub const KEYBOARD_ENHANCEMENT_FLAGS: KeyboardEnhancementFlags =
        KeyboardEnhancementFlags::all();

    pub fn new(
        mut term: T,
        custom_start_seed: Option<u64>,
        custom_start_board: Option<String>,
    ) -> Self {
        // Console prologue: Initialization.
        // FIXME: Handle io::Error? If not, why not?
        let _v = term.execute(terminal::EnterAlternateScreen);
        let _v = term.execute(terminal::SetTitle("Tetro Terminal User Interface"));
        let _v = term.execute(cursor::Hide);
        let _v = terminal::enable_raw_mode();
        let mut app = Self {
            runtime_data: RuntimeData {
                kitty_detected: false,
                kitty_assumed: false,
            },
            term,
            settings: Settings::default(),
            scores_and_replays: ScoresAndReplays::default(),
            game_saves: (0, Vec::new()),
            save_on_exit: SavefileGranularity::NoSavefile,
        };

        // Actually load in settings.
        // FIXME: Handle io::Error? If not, why not?
        if app.load_savefile(Self::savefile_path()).is_err() {
            //eprintln!("Could not load settings: {e}");
            //std::thread::sleep(Duration::from_secs(5));
        }

        // Now that the settings are loaded, we handle separate flags set for this session.
        let kitty_detected = terminal::supports_keyboard_enhancement().unwrap_or(false);
        app.runtime_data = RuntimeData {
            kitty_detected,
            kitty_assumed: kitty_detected,
        };
        if custom_start_board.is_some() {
            app.settings.new_game.custom_board = custom_start_board;
        }
        if custom_start_seed.is_some() {
            app.settings.new_game.custom_seed = custom_start_seed;
        }
        app
    }

    pub(crate) fn fetch_main_xy() -> (u16, u16) {
        let (w_console, h_console) = terminal::size().unwrap_or((0, 0));
        (
            w_console.saturating_sub(Self::W_MAIN) / 2,
            h_console.saturating_sub(Self::H_MAIN) / 2,
        )
    }

    fn savefile_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(Self::SAVEFILE_NAME)
    }

    fn store_savefile(&mut self, path: PathBuf) -> io::Result<()> {
        if self.save_on_exit < SavefileGranularity::RememberSettingsScores {
            // Clear scoreboard if no game data is wished to be stored.
            self.scores_and_replays.entries.clear();
        } else if self.save_on_exit < SavefileGranularity::RememberSettingsScoresReplays {
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

        let save_state = (
            &self.save_on_exit,
            &self.settings,
            &self.scores_and_replays,
            compressed_game_saves,
        );
        let save_str = serde_json::to_string(&save_state)?;
        let mut file = File::create(path)?;

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

    fn load_savefile(&mut self, path: PathBuf) -> io::Result<()> {
        let mut file = File::open(path)?;
        let mut save_str = String::new();
        file.read_to_string(&mut save_str)?;
        let save_state = serde_json::from_str(&save_str)?;

        let compressed_game_saves: (usize, Vec<GameSave<CompressedInputHistory>>);

        (
            self.save_on_exit,
            self.settings,
            self.scores_and_replays,
            compressed_game_saves,
        ) = save_state;

        self.game_saves = (
            compressed_game_saves.0,
            compressed_game_saves
                .1
                .into_iter()
                .map(|save| save.map(|input_history| input_history.decompress()))
                .collect::<Vec<_>>(),
        );

        Ok(())
    }

    fn sort_past_games_chronologically(&mut self) {
        self.scores_and_replays
            .entries
            .sort_by(|(pg1, _), (pg2, _)| {
                pg1.game_meta_data
                    .datetime
                    .cmp(&pg2.game_meta_data.datetime)
                    .reverse()
            });
    }

    #[rustfmt::skip]
    fn sort_past_games_semantically(&mut self) {
        self.scores_and_replays.entries.sort_by(|(pg1, _), (pg2, _)|
            // Sort by gamemode (name).
            pg1.game_meta_data.title.cmp(&pg2.game_meta_data.title).then_with(||
            // Sort by if gamemode was finished successfully.
            pg1.result.is_ok().cmp(&pg2.result.is_ok()).reverse().then_with(|| {
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

    pub fn run(&mut self) -> io::Result<()> {
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
                } => self.run_menu_play_game(
                    game,
                    game_input_history,
                    game_meta_data,
                    game_renderer.as_mut(),
                ),
                Menu::Pause => self.run_menu_pause(),
                Menu::Settings => self.run_menu_settings(),
                Menu::AdjustGraphics => self.run_menu_adjust_graphics(),
                Menu::AdjustKeybinds => self.run_menu_adjust_keybinds(),
                Menu::AdjustGameplay => self.run_menu_adjust_gameplay(),
                Menu::GameOver(past_game) => self.run_menu_game_ended(past_game),
                Menu::GameComplete(past_game) => self.run_menu_game_ended(past_game),
                Menu::ScoresAndReplays => self.run_menu_scores_and_replays(),
                Menu::ReplayGame {
                    game_restoration_data,
                    game_meta_data,
                    game_renderer,
                } => self.run_menu_replay_game(
                    game_restoration_data,
                    game_meta_data,
                    game_renderer.as_mut(),
                ),
                Menu::About => self.run_menu_about(),
                Menu::Quit => break,
            }?;

            // Change screen session depending on what response screen gave.
            match menu_update {
                MenuUpdate::Pop => {
                    if menu_stack.len() > 1 {
                        menu_stack.pop();
                    }
                }
                MenuUpdate::Push(menu) => {
                    if matches!(
                        menu,
                        Menu::Title
                            | Menu::PlayGame { .. }
                            | Menu::GameOver(_)
                            | Menu::GameComplete(_)
                    ) {
                        menu_stack.clear();
                    }
                    menu_stack.push(menu);
                }
            }
        }

        Ok(())
    }
}
