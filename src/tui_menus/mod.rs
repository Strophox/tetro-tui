pub mod about;
pub mod adjust_gameplay;
pub mod adjust_graphics;
pub mod adjust_keybinds;
pub mod advanced_settings;
pub mod game_ended;
pub mod keybinds_help;
pub mod new_game;
pub mod pause;
pub mod play_game;
pub mod replay_game;
pub mod scores_and_replays;
pub mod settings;
pub mod statistics;
pub mod title;

use std::io::{self, Write};

use crossterm::{
    cursor::{MoveDown, MoveTo, MoveToColumn},
    event::{
        self, Event, KeyCode, KeyEvent,
        KeyEventKind::{self, Press, Repeat},
        KeyModifiers,
    },
    style::{Print, PrintStyledContent, Stylize},
    terminal::{Clear, ClearType},
    ExecutableCommand, QueueableCommand,
};
use falling_tetromino_engine::{Game, InGameTime};

use crate::{
    game_renderers::TetroTUIRenderer, tui_menus::replay_game::GameSaveAnchor,
    tui_settings::Settings, Application, GameMetaData, GameRestorationData, RawInputHistory,
    ScoreEntry,
};

#[derive(Debug)]
pub enum MenuUpdate {
    Pop,
    Push(Menu),
}

#[derive(Debug)]
pub enum Menu {
    Title,
    KeybindsOverview {
        client_menu_name: &'static str,
        legend: Vec<(String, Vec<(String, String)>)>,
    },
    NewGame,
    PlayGame {
        game: Box<Game>,
        raw_input_history: RawInputHistory,
        game_meta_data: GameMetaData,
        // game_statistics: Statistics,
        game_renderer: Box<TetroTUIRenderer>,
        selection_id_for_game_retry: Option<usize>,
    },
    Pause,
    Settings,
    AdjustGraphics,
    AdjustKeybinds,
    AdjustGameplay,
    AdvancedSettings,
    GameOver {
        game_scoring: Box<ScoreEntry>,
        // game_statistics: Statistics,
    },
    GameComplete {
        game_scoring: Box<ScoreEntry>,
        // game_statistics: Statistics,
    },
    ScoresAndReplays {
        cursor_pos: usize,
        camera_pos: usize,
    },
    ReplayGame {
        game_restoration_data: Box<GameRestorationData<RawInputHistory>>,
        game_meta_data: GameMetaData,
        replay_length: InGameTime,
        game_renderer: Box<TetroTUIRenderer>,
        cached_game_and_replay_anchors: Box<(Game, Option<Vec<GameSaveAnchor>>)>,
    },
    Statistics,
    About,
    Quit,
}

impl Menu {
    pub fn str_for_list_menu_selection(&self) -> &str {
        match self {
            Menu::Title => "Title Screen",
            Menu::KeybindsOverview { .. } => "Keybinds Overview",
            // Menu::KeybindsOverview { is_help_for_menu_with_name, .. } => return format!("Keybinds Overview ({})", is_help_for_menu_with_name),
            Menu::NewGame => "New Game",
            Menu::PlayGame { .. } => "Live Game",
            // Menu::PlayGame { game_meta_data, .. } => {
            //     return format!("Playing Game ({})", game_meta_data.title)
            // }
            Menu::Pause => "Pause",
            Menu::Settings => "Settings",
            Menu::AdjustGraphics => "Adjust Graphics",
            Menu::AdjustKeybinds => "Adjust Keybinds",
            Menu::AdjustGameplay => "Adjust Gameplay",
            Menu::AdvancedSettings => "Advanced Settings",
            Menu::GameOver { .. } => "Game Over",
            Menu::GameComplete { .. } => "Game Completed",
            Menu::ScoresAndReplays { .. } => "Scores and Replays",
            Menu::ReplayGame { .. } => "Game Replay",
            // Menu::ReplayGame { game_meta_data, .. } => {
            //     return format!("Replaying Game ({})", game_meta_data.title)
            // }
            Menu::Statistics => "All-Time Stats",
            Menu::About => "About",
            Menu::Quit => "Quit",
        }
    }
}

impl<T: Write> Application<T> {
    pub fn run_text_menu(
        &mut self,
        head: &str,
        body: &str,
        center: bool,
        keybinds_help_client_menu_name: &'static str,
    ) -> io::Result<MenuUpdate> {
        let lines: Vec<_> = body.lines().collect();
        let mut cursor_pos = 0;
        const CAMERA_SIZE: usize = 14;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::viewport_offset();

            self.term.queue(Clear(ClearType::All))?;

            let y_selection = (Self::H_MAIN / 5).saturating_sub(2);

            self.term
                .queue(MoveTo(x_main, y_main + y_selection))?
                .queue(PrintStyledContent(format!("{head:^w_main$}").bold()))?;

            self.term
                .queue(MoveTo(x_main, y_main + y_selection + 2))?
                .queue(Print(format!("{:^w_main$}", heading_line(&self.settings))))?;

            self.term.queue(MoveTo(x_main, y_main + y_selection + 4))?;

            for line in lines.iter().skip(cursor_pos).take(CAMERA_SIZE) {
                self.term.queue(MoveToColumn(x_main))?;
                if center {
                    self.term.queue(Print(format!("{line:^w_main$}")))?;
                } else {
                    self.term.queue(Print(line))?;
                }

                self.term.queue(MoveDown(1))?;
            }

            let remaining = lines.len().saturating_sub(cursor_pos + CAMERA_SIZE);
            if remaining > 0 {
                self.term
                    .queue(MoveToColumn(x_main))?
                    .queue(Print(format!(
                        "{:^w_main$}",
                        format!("(... +{remaining})",)
                    )))?
                    .queue(MoveDown(1))?;
            }

            self.term.flush()?;

            // Wait for new input.
            match event::read()? {
                // Quit menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c' | 'C'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: Press | Repeat,
                    state: _,
                }) => break Ok(MenuUpdate::Push(Menu::Quit)),

                // Keybinds help menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('?'),
                    kind: KeyEventKind::Press | KeyEventKind::Repeat,
                    ..
                }) => {
                    if !head.ends_with("Overview' $") {
                        let legend = vec![
                            (
                                "Normal keybinds".to_owned(),
                                [
                                    ("Esc q Backspace", "Exit menu"),
                                    ("↓/↑ j/k", "Navigate down/up"),
                                    ("?", "Open Keybinds overview"),
                                ]
                                .into_iter()
                                .map(|(lhs, rhs)| (lhs.to_owned(), rhs.to_owned()))
                                .collect(),
                            ),
                            (
                                "Special keybinds".to_owned(),
                                [
                                    (
                                        "Ctrl+Alt+L",
                                        "Re-load from savefile (overwrites current data!)",
                                    ),
                                    (
                                        "Ctrl+Alt+S",
                                        "Do savefile storage (respects save preferences)",
                                    ),
                                    ("Ctrl+C", "Exit program (respects save preferences)"),
                                ]
                                .into_iter()
                                .map(|(lhs, rhs)| (lhs.to_owned(), rhs.to_owned()))
                                .collect(),
                            ),
                        ];

                        break Ok(MenuUpdate::Push(Menu::KeybindsOverview {
                            client_menu_name: keybinds_help_client_menu_name,
                            legend,
                        }));
                    } else {
                        self.term
                            .execute(MoveTo(x_main, y_main + y_selection + 4))?
                            .execute(Clear(ClearType::FromCursorDown))?
                            .execute(Print(Self::EGG))?;
                        event::read()?;
                    }
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Esc | KeyCode::Char('q' | 'Q') | KeyCode::Backspace,
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),

                // Move selector up.
                Event::Key(KeyEvent {
                    code: KeyCode::Up | KeyCode::Char('k' | 'K'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    cursor_pos += lines.len() - 1;
                }

                // Move selector down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j' | 'J'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    cursor_pos += 1;
                }

                // Reload from savefile.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('l' | 'L'),
                    modifiers,
                    kind: Press | Repeat,
                    ..
                }) if { modifiers.contains(KeyModifiers::CONTROL.union(KeyModifiers::ALT)) } => {
                    self.temp_data.loadfile_result = self.savefile_load();
                }

                // Store to savefile.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('s' | 'S'),
                    modifiers,
                    kind: Press | Repeat,
                    ..
                }) if { modifiers.contains(KeyModifiers::CONTROL.union(KeyModifiers::ALT)) } => {
                    self.temp_data.storefile_result = self.savefile_store();
                }

                // Other event: don't care.
                _ => {}
            }
            cursor_pos %= lines.len();
        }
    }

    const EGG: &str = r#" ▀█▀ "I am like Solomon because I built God's temple, an operating system. God said 640x480 16 color graphics but the operating system is 64-bit and multi-cored! Go draw a 16 color elephant. Then, draw a 24-bit elephant in MS Paint and be enlightened. Artist stopped photorealism when the camera was invented. A cartoon is actually better than photorealistic. For the next thousand years, first-person shooters are going to get boring. Tetris looks good." - Terry Davis"#;

    /// A transitory menu that only consists of selectable links to other menus.
    pub fn run_liminal_menu(
        &mut self,
        client_menu_name: &'static str,
        head: &str,
        body: Vec<Menu>,
    ) -> io::Result<MenuUpdate> {
        let mut selected = 0usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::viewport_offset();
            let y_selection = Self::H_MAIN / 5;
            if head.is_empty() {
                self.term
                    .queue(Clear(ClearType::All))?
                    .queue(MoveTo(x_main, y_main + y_selection))?
                    .queue(Print(format!("{:^w_main$}", "▀█▀ ██ ▀█▀ █▀▀ ▄█▀")))?
                    .queue(MoveTo(x_main, y_main + y_selection + 1))?
                    .queue(Print(format!("{:^w_main$}", "    █▄▄▄▄▄▄       ")))?;
            } else {
                self.term
                    .queue(Clear(ClearType::All))?
                    .queue(MoveTo(x_main, y_main + y_selection))?
                    .queue(PrintStyledContent(
                        format!("{:^w_main$}", format!("- {} -", head)).bold(),
                    ))?
                    .queue(MoveTo(x_main, y_main + y_selection + 2))?
                    .queue(Print(format!("{:^w_main$}", heading_line(&self.settings))))?;
            }
            let names = body
                .iter()
                .map(|menu| menu.str_for_list_menu_selection())
                .collect::<Vec<_>>();
            let n_names = names.len();
            if n_names == 0 {
                self.term
                    .queue(MoveTo(x_main, y_main + y_selection + 5))?
                    .queue(PrintStyledContent(
                        format!(
                            "{:^w_main$}",
                            "(There isn't anything interesting implemented here yet... )",
                        )
                        .italic(),
                    ))?;
            } else {
                for (i, name) in names.into_iter().enumerate() {
                    self.term
                        .queue(MoveTo(
                            x_main,
                            y_main + y_selection + 4 + u16::try_from(i).unwrap(),
                        ))?
                        .queue(Print(format!(
                            "{:^w_main$}",
                            if i == selected {
                                format!(">> {name} <<")
                            } else {
                                name.to_owned()
                            }
                        )))?;
                }
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 4 + u16::try_from(n_names).unwrap() + 2,
                    ))?
                    .queue(PrintStyledContent(
                        format!(
                            "{:^w_main$}",
                            "[Enter/Esc/Del/←↓↑→] or Vim, [?] to view keybinds anywhere",
                        )
                        .italic(),
                    ))?;
            }
            self.term.flush()?;
            // Wait for new input.
            match event::read()? {
                // Quit menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c' | 'C'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: Press | Repeat,
                    state: _,
                }) => break Ok(MenuUpdate::Push(Menu::Quit)),

                // Keybinds help menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('?'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    let legend = vec![
                        (
                            "Normal keybinds".to_owned(),
                            [
                                ("Enter e", "Select"),
                                ("Esc q Backspace", "Exit menu"),
                                ("↓/↑ j/k", "Navigate down/up"),
                                ("?", "Open Keybinds overview"),
                            ]
                            .into_iter()
                            .map(|(lhs, rhs)| (lhs.to_owned(), rhs.to_owned()))
                            .collect(),
                        ),
                        (
                            "Special keybinds".to_owned(),
                            [
                                (
                                    "Ctrl+Alt+L",
                                    "Re-load from savefile (overwrites current data!)",
                                ),
                                (
                                    "Ctrl+Alt+S",
                                    "Do savefile storage (respects save preferences)",
                                ),
                                ("Ctrl+C", "Exit program (respects save preferences)"),
                            ]
                            .into_iter()
                            .map(|(lhs, rhs)| (lhs.to_owned(), rhs.to_owned()))
                            .collect(),
                        ),
                    ];

                    break Ok(MenuUpdate::Push(Menu::KeybindsOverview {
                        client_menu_name,
                        legend,
                    }));
                }

                Event::Key(KeyEvent {
                    code: KeyCode::Esc | KeyCode::Char('q' | 'Q') | KeyCode::Backspace,
                    kind: Press,
                    ..
                }) => break Ok(MenuUpdate::Pop),

                // Select next menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Enter | KeyCode::Char('e' | 'E'),
                    kind: Press,
                    ..
                }) => {
                    if !body.is_empty() {
                        let menu = body.into_iter().nth(selected).unwrap();
                        break Ok(MenuUpdate::Push(menu));
                    }
                }

                // Move selector up.
                Event::Key(KeyEvent {
                    code: KeyCode::Up | KeyCode::Char('k' | 'K'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if !body.is_empty() {
                        selected += body.len() - 1;
                    }
                }

                // Move selector down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j' | 'J'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if !body.is_empty() {
                        selected += 1;
                    }
                }

                // Reload from savefile.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('l' | 'L'),
                    modifiers,
                    kind: Press | Repeat,
                    ..
                }) if { modifiers.contains(KeyModifiers::CONTROL.union(KeyModifiers::ALT)) } => {
                    self.temp_data.loadfile_result = self.savefile_load();
                }

                // Store to savefile.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('s' | 'S'),
                    modifiers,
                    kind: Press | Repeat,
                    ..
                }) if { modifiers.contains(KeyModifiers::CONTROL.union(KeyModifiers::ALT)) } => {
                    self.temp_data.storefile_result = self.savefile_store();
                }

                // Other event: don't care.
                _ => {}
            }
            if !body.is_empty() {
                selected = selected.rem_euclid(body.len());
            }
        }
    }
}

pub fn heading_line(settings: &Settings) -> String {
    settings.tui_symbols().headingline[0].to_string().repeat(32)
}
