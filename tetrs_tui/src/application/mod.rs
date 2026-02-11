mod menus;

use std::{
    fmt::Debug,
    fs::File,
    io::{self, Read, Write},
    num::{NonZeroU32, NonZeroUsize},
    path::PathBuf,
    time::{Duration, Instant},
};

use crossterm::{cursor, style, terminal, ExecutableCommand};

use tetrs_engine::{
    Board, Button, ButtonChange, Configuration, DelayParameters, ExtDuration, ExtNonNegF64,
    Feedback, FeedbackVerbosity, Game, GameBuilder, GameOver, GameResult, InGameTime, Modifier,
    RotationSystem, Stat, Tetromino, TetrominoGenerator,
};

use crate::{
    game_mode_presets, game_renderers,
    keybinds_presets::{
        guideline_keybinds, tetrs_default_keybinds, tetrs_finesse_keybinds, vim_keybinds, Keybinds,
    },
    palette_presets::{
        color16_palette, fullcolor_palette, gruvbox_light_palette, gruvbox_palette,
        monochrome_palette, oklch_palette, the_matrix_palette, Palette,
    },
};

pub type Slots<T> = Vec<(String, T)>;

#[derive(
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Clone,
    Hash,
    Debug,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct ButtonInputHistory(Vec<u128>);

impl ButtonInputHistory {
    pub const BUTTON_CHANGE_BITSIZE: usize = 5;

    // For serialization reasons, we encode a single user input as `u128` instead of
    // `(GameTime, ButtonChange)`, which would have a more verbose string representation.
    pub fn encode(update_target_time: InGameTime, button_change: ButtonChange) -> u128 {
        // Encode `GameTime = std::time::Duration` using `std::time::Duration::as_nanos`.
        let nanos: u128 = update_target_time.as_nanos();
        // Encode `tetrs_engine::ButtonChange` using `Self::encode_button_change`.
        let bc_bits: u8 = Self::encode_button_change(&button_change);
        (nanos << Self::BUTTON_CHANGE_BITSIZE) | u128::from(bc_bits)
    }

    pub fn decode(num: u128) -> (InGameTime, ButtonChange) {
        let mask = u128::MAX >> (128 - Self::BUTTON_CHANGE_BITSIZE);
        let bc_bits = u8::try_from(num & mask).unwrap();
        let nanos = u64::try_from(num >> Self::BUTTON_CHANGE_BITSIZE).unwrap();
        (
            std::time::Duration::from_nanos(nanos),
            Self::decode_button_change(bc_bits),
        )
    }

    pub fn encode_button_change(button_change: &ButtonChange) -> u8 {
        match button_change {
            ButtonChange::Release(button) => (*button as u8) << 1,
            ButtonChange::Press(button) => ((*button as u8) << 1) | 1,
        }
    }

    pub fn decode_button_change(bc_bits: u8) -> ButtonChange {
        (if bc_bits.is_multiple_of(2) {
            ButtonChange::Release
        } else {
            ButtonChange::Press
        })(Button::VARIANTS[usize::from(bc_bits >> 1)])
    }
}

#[derive(PartialEq, PartialOrd, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct GameRestorationData {
    builder: GameBuilder,
    mod_descriptors: Vec<String>,
    input_history: ButtonInputHistory,
}

impl GameRestorationData {
    fn new(game: &Game, input_history: &ButtonInputHistory) -> GameRestorationData {
        let (builder, mod_descriptors) = game.blueprint();
        GameRestorationData {
            builder,
            mod_descriptors: mod_descriptors.map(str::to_owned).collect(),
            input_history: input_history.clone(),
        }
    }

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
                Ok(modified_game) => modified_game,
                Err(msg) => {
                    #[rustfmt::skip]
                    let print_error_msg_mod = Modifier {
                        descriptor: "print_error_msg_mod".to_owned(),
                        mod_function: Box::new({ let mut init = false;
                            move |_point, _config, _init_vals, state, _phase, msgs| {
                                if init { return; } init = true;
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
        for bits in self.input_history.0.iter().take(input_index) {
            let (update_time, button_change) = ButtonInputHistory::decode(*bits);
            // FIXME: Error handling?
            let _ = game.update(update_time, Some(button_change));
        }

        game.config.feedback_verbosity = restore_feedback_verbosity;

        game
    }
}

#[derive(
    Eq, PartialEq, Ord, PartialOrd, Clone, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct GameMetaData {
    pub datetime: String,
    pub title: String,
    pub comparison_stat: (Stat, bool),
}

#[derive(
    Eq, PartialEq, Ord, PartialOrd, Clone, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct ScoreboardEntry {
    game_meta_data: GameMetaData,
    result: GameResult,
    time_elapsed: InGameTime,
    lineclears: u32,
    points_scored: u32,
    pieces_locked: [u32; Tetromino::VARIANTS.len()],
    final_fall_delay: ExtDuration,
    final_lock_delay: ExtDuration,
}

impl ScoreboardEntry {
    fn new(game: &Game, game_meta_data: &GameMetaData) -> ScoreboardEntry {
        ScoreboardEntry {
            game_meta_data: game_meta_data.clone(),
            time_elapsed: game.state().time,
            pieces_locked: game.state().pieces_locked,
            lineclears: game.state().lineclears,
            final_fall_delay: game.state().fall_delay,
            final_lock_delay: game.state().lock_delay,
            points_scored: game.state().score,
            result: game.result().unwrap_or(Err(GameOver::Forfeit)),
        }
    }
}

#[derive(
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Clone,
    Copy,
    Hash,
    Debug,
    Default,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum ScoreboardSorting {
    #[default]
    Chronological,
    Semantic,
}

#[derive(PartialEq, PartialOrd, Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Scoreboard {
    sorting: ScoreboardSorting,
    entries: Vec<(ScoreboardEntry, Option<GameRestorationData>)>,
}

#[derive(
    Eq, PartialEq, Ord, PartialOrd, Clone, Hash, Debug, serde::Serialize, serde::Deserialize,
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
            custom_fall_delay_params: DelayParameters::default_fall(),
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
    Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum Glyphset {
    Electronika60,
    #[allow(clippy::upper_case_acronyms)]
    ASCII,
    Unicode,
}

#[derive(PartialEq, PartialOrd, Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct GraphicsSettings {
    palette_active: usize,
    palette_active_lockedtiles: usize,
    pub glyphset: Glyphset,
    pub render_effects: bool,
    pub blindfolded: bool,
    pub show_ghost_piece: bool,
    game_fps: f64,
    show_fps: bool,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self {
            glyphset: Glyphset::Unicode,
            palette_active: 3,
            palette_active_lockedtiles: 3,
            render_effects: true,
            blindfolded: false,
            show_ghost_piece: true,
            game_fps: 30.0,
            show_fps: false,
        }
    }
}

#[derive(PartialEq, PartialOrd, Clone, Debug, serde::Serialize, serde::Deserialize)]
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
    Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub enum SavefileGranularity {
    NoSavefile,
    RememberSettings,
    RememberSettingsScoreboard,
    RememberSettingsScoreboardGamerecords,
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
            ("default".to_owned(), GraphicsSettings::default()),
            (
                "focused".to_owned(),
                GraphicsSettings {
                    palette_active: 2,
                    palette_active_lockedtiles: 0,
                    render_effects: false,
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
            ("Gruvbox (light)".to_owned(), gruvbox_light_palette()),
            ("Gruvbox".to_owned(), gruvbox_palette()),
            ("The Matrix".to_owned(), the_matrix_palette()),
        ];
        let keybinds_slots = vec![
            ("Tetrs default".to_owned(), tetrs_default_keybinds()),
            ("Tetrs finesse".to_owned(), tetrs_finesse_keybinds()),
            ("Vim-like".to_owned(), vim_keybinds()),
            ("TTC default".to_owned(), guideline_keybinds()),
        ];
        let gameplay_slots = vec![
            ("default".to_owned(), GameplaySettings::default()),
            (
                "finesse".to_owned(),
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
    Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct RuntimeData {
    kitty_detected: bool,
    kitty_assumed: bool,
}

#[derive(Debug)]
enum Menu {
    Title,
    NewGame,
    Game {
        game: Box<Game>,
        meta_data: GameMetaData,
        time_started: Instant,
        last_paused: Instant,
        total_pause_duration: Duration,
        button_input_history: ButtonInputHistory,
        game_renderer: Box<game_renderers::diff_print::DiffPrintRenderer>,
    },
    GameOver(Box<ScoreboardEntry>),
    GameComplete(Box<ScoreboardEntry>),
    Pause,
    Settings,
    AdjustKeybinds,
    AdjustGameplay,
    AdjustGraphics,
    Scores,
    About,
    Quit(String),
}

impl std::fmt::Display for Menu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Menu::Title => "Title Screen",
            Menu::NewGame => "New Game",
            Menu::Game { meta_data, .. } => &format!("Game ({})", meta_data.title),
            Menu::GameOver(_) => "Game Over",
            Menu::GameComplete(_) => "Game Completed",
            Menu::Pause => "Pause",
            Menu::Settings => "Settings",
            Menu::AdjustKeybinds => "Adjust Keybinds",
            Menu::AdjustGameplay => "Adjust Gameplay",
            Menu::AdjustGraphics => "Adjust Graphics",
            Menu::Scores => "Scoreboard",
            Menu::About => "About",
            Menu::Quit(_) => "Quit",
        };
        write!(f, "{name}")
    }
}

#[derive(Debug)]
enum MenuUpdate {
    Pop,
    Push(Menu),
}

#[derive(PartialEq, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Application<T: Write> {
    runtime_data: RuntimeData,
    pub term: T,
    save_on_exit: SavefileGranularity,
    settings: Settings,
    scoreboard: Scoreboard,
    game_savepoint: Option<(GameMetaData, GameRestorationData, usize)>,
}

impl<T: Write> Drop for Application<T> {
    fn drop(&mut self) {
        let savefile_path = Self::savefile_path();
        // If the user wants any of their data stored, try to do so.
        if self.save_on_exit != SavefileGranularity::NoSavefile {
            // FIXME: Handle error?
            if let Err(_e) = self.store_savefile(savefile_path) {
                //eprintln!("Could not save settings this time: {e} ");
                //std::thread::sleep(Duration::from_secs(4));
            }
        // Otherwise explicitly check for savefile to make sure it's removed.
        } else if savefile_path.try_exists().is_ok_and(|exists| exists) {
            let _ = std::fs::remove_file(savefile_path);
        }
        // FIXME: Handle error?
        let _ = terminal::disable_raw_mode();
        let _ = self.term.execute(style::ResetColor);
        let _ = self.term.execute(cursor::Show);
        let _ = self.term.execute(terminal::LeaveAlternateScreen);
    }
}

impl<T: Write> Application<T> {
    pub const W_MAIN: u16 = 80;
    pub const H_MAIN: u16 = 24;

    pub const SAVEFILE_NAME: &'static str =
        concat!(".tetrs_tui_", clap::crate_version!(), "_savefile.json");

    pub fn new(
        mut term: T,
        custom_start_seed: Option<u64>,
        custom_start_board: Option<String>,
    ) -> Self {
        // Console prologue: Initialization.
        // FIXME: Handle error?
        let _ = term.execute(terminal::EnterAlternateScreen);
        let _ = term.execute(terminal::SetTitle("tetrs - Terminal User Interface"));
        let _ = term.execute(cursor::Hide);
        let _ = terminal::enable_raw_mode();
        let mut app = Self {
            runtime_data: RuntimeData {
                kitty_detected: false,
                kitty_assumed: false,
            },
            term,
            settings: Settings::default(),
            scoreboard: Scoreboard::default(),
            game_savepoint: None,
            save_on_exit: SavefileGranularity::NoSavefile,
        };

        // Actually load in settings.
        // FIXME: Handle error?
        if app.load_savefile(Self::savefile_path()).is_err() {
            //eprintln!("Could not loading settings: {e}");
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

    pub fn settings(&self) -> &Settings {
        &self.settings
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
        if self.save_on_exit < SavefileGranularity::RememberSettingsScoreboard {
            // Clear scoreboard if no game data is wished to be stored.
            self.scoreboard.entries.clear();
        } else if self.save_on_exit < SavefileGranularity::RememberSettingsScoreboardGamerecords {
            // Clear past game inputs if no game input data is wished to be stored.
            for (_entry, restoration_data) in &mut self.scoreboard.entries {
                restoration_data.take();
            }
        }

        let save_state = (
            &self.save_on_exit,
            &self.settings,
            &self.scoreboard,
            &self.game_savepoint,
        );
        let save_str = serde_json::to_string(&save_state)?;
        let mut file = File::create(path)?;
        // FIXME: Handle error?
        let _ = file.write(save_str.as_bytes())?;
        Ok(())
    }

    fn load_savefile(&mut self, path: PathBuf) -> io::Result<()> {
        let mut file = File::open(path)?;
        let mut save_str = String::new();
        file.read_to_string(&mut save_str)?;
        let save_state = serde_json::from_str(&save_str)?;
        (
            self.save_on_exit,
            self.settings,
            self.scoreboard,
            self.game_savepoint,
        ) = save_state;
        Ok(())
    }

    fn sort_past_games_chronologically(&mut self) {
        self.scoreboard.entries.sort_by(|(pg1, _), (pg2, _)| {
            pg1.game_meta_data
                .datetime
                .cmp(&pg2.game_meta_data.datetime)
                .reverse()
        });
    }

    #[rustfmt::skip]
    fn sort_past_games_semantically(&mut self) {
        self.scoreboard.entries.sort_by(|(pg1, _), (pg2, _)|
            // Sort by gamemode (name).
            pg1.game_meta_data.title.cmp(&pg2.game_meta_data.title).then_with(||
            // Sort by if gamemode was finished successfully.
            pg1.result.is_ok().cmp(&pg2.result.is_ok()).then_with(|| {
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
            .reverse()
        );
    }

    pub fn run(&mut self) -> io::Result<String> {
        let mut menu_stack = vec![Menu::Title];
        let msg = loop {
            // Retrieve active menu, stop application if stack is empty.
            let Some(screen) = menu_stack.last_mut() else {
                break String::from("all menus exited");
            };
            // Open new menu screen, then store what it returns.
            let menu_update = match screen {
                Menu::Title => self.menu_title(),
                Menu::NewGame => self.menu_new_game(),
                Menu::Game {
                    game,
                    meta_data,
                    time_started,
                    total_pause_duration,
                    last_paused,
                    button_input_history,
                    game_renderer,
                } => self.menu_play_game(
                    game,
                    meta_data,
                    time_started,
                    last_paused,
                    total_pause_duration,
                    button_input_history,
                    game_renderer.as_mut(),
                ),
                Menu::Pause => self.menu_pause(),
                Menu::GameOver(past_game) => self.menu_game_ended(past_game),
                Menu::GameComplete(past_game) => self.menu_game_ended(past_game),
                Menu::Scores => self.menu_scoreboard(),
                Menu::About => self.menu_about(),
                Menu::Settings => self.menu_settings(),
                Menu::AdjustKeybinds => self.menu_adjust_keybinds(),
                Menu::AdjustGameplay => self.menu_adjust_gameplay(),
                Menu::AdjustGraphics => self.menu_adjust_graphics(),
                Menu::Quit(string) => break string.clone(),
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
                        Menu::Title | Menu::Game { .. } | Menu::GameOver(_) | Menu::GameComplete(_)
                    ) {
                        menu_stack.clear();
                    }
                    menu_stack.push(menu);
                }
            }
        };
        Ok(msg)
    }
}
