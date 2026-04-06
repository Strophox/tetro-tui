pub mod about;
pub mod adjust_gameplay;
pub mod adjust_graphics;
pub mod adjust_keybinds;
pub mod advanced_settings;
pub mod game_ended;
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
    cursor::MoveTo,
    event::{
        self, Event, KeyCode, KeyEvent,
        KeyEventKind::{Press, Repeat},
        KeyModifiers,
    },
    style::{Print, PrintStyledContent, Stylize},
    terminal::{Clear, ClearType},
    QueueableCommand,
};
use falling_tetromino_engine::{Game, InGameTime};

use crate::{
    application::{
        Application, GameMetaData, GameRestorationData, ScoreEntry, UncompressedInputHistory,
    },
    game_renderers::TetroTUIRenderer,
};

#[derive(Debug)]
pub enum MenuUpdate {
    Pop,
    Push(Menu),
}

#[derive(Debug)]
pub enum Menu {
    Title,
    NewGame,
    PlayGame {
        game: Box<Game>,
        game_input_history: UncompressedInputHistory,
        game_meta_data: GameMetaData,
        // game_statistics: Statistics,
        game_renderer: Box<TetroTUIRenderer>,
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
        game_restoration_data: Box<GameRestorationData<UncompressedInputHistory>>,
        game_meta_data: GameMetaData,
        replay_length: InGameTime,
        game_renderer: Box<TetroTUIRenderer>,
    },
    Statistics,
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
            Menu::AdvancedSettings => "Advanced Settings",
            Menu::GameOver { .. } => "Game Over",
            Menu::GameComplete { .. } => "Game Completed",
            Menu::ScoresAndReplays { .. } => "Scores and Replays",
            Menu::ReplayGame { game_meta_data, .. } => {
                &format!("Replaying Game ({})", game_meta_data.title)
            }
            Menu::Statistics => "Statistics",
            Menu::About => "About",
            Menu::Quit => "Quit",
        };
        write!(f, "{name}")
    }
}

impl<T: Write> Application<T> {
    pub(in crate::application) fn generic_menu(
        &mut self,
        current_menu_name: &str,
        selection: Vec<Menu>,
    ) -> io::Result<MenuUpdate> {
        let mut easteregg = 0isize;
        let mut selected = 0usize;
        loop {
            let w_main = Self::W_MAIN.into();
            let (x_main, y_main) = Self::fetch_main_xy();
            let y_selection = Self::H_MAIN / 5;
            if current_menu_name.is_empty() {
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
                        format!("{:^w_main$}", format!("- {} -", current_menu_name)).bold(),
                    ))?
                    .queue(MoveTo(x_main, y_main + y_selection + 2))?
                    .queue(Print(format!("{:^w_main$}", "──────────────────────────")))?;
            }
            let names = selection
                .iter()
                .map(|menu| menu.to_string())
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
                                name
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
                            "(Controls: [←|↓|↑|→] [Esc|Enter|Del] / hjklqed)",
                        )
                        .italic(),
                    ))?;
            }
            if easteregg.abs() == 42 {
                self.term
                    .queue(Clear(ClearType::All))?
                    .queue(MoveTo(0, y_main))?
                    .queue(PrintStyledContent(DAVIS.italic()))?;
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
                    if !selection.is_empty() {
                        let menu = selection.into_iter().nth(selected).unwrap();
                        break Ok(MenuUpdate::Push(menu));
                    }
                }
                // Move selector up.
                Event::Key(KeyEvent {
                    code: KeyCode::Up | KeyCode::Char('k' | 'K'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if !selection.is_empty() {
                        selected += selection.len() - 1;
                    }
                    easteregg -= 1;
                }
                // Move selector down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j' | 'J'),
                    kind: Press | Repeat,
                    ..
                }) => {
                    if !selection.is_empty() {
                        selected += 1;
                    }
                    easteregg += 1;
                }
                // Other event: don't care.
                _ => {}
            }
            if !selection.is_empty() {
                selected = selected.rem_euclid(selection.len());
            }
        }
    }
}

const DAVIS: &str = r#" ▀█▀ "I am like Solomon because I built God's temple, an operating system. God said 640x480 16 color graphics but the operating system is 64-bit and multi-cored! Go draw a 16 color elephant. Then, draw a 24-bit elephant in MS Paint and be enlightened. Artist stopped photorealism when the camera was invented. A cartoon is actually better than photorealistic. For the next thousand years, first-person shooters are going to get boring. Tetris looks good." - In memory of Terry A. Davis"#;
