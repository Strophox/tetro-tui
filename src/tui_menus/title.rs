use std::io::{self, Write};

use crossterm::{
    QueueableCommand,
    cursor::MoveTo,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    style::{Color, Print, PrintStyledContent, Stylize},
    terminal::{Clear, ClearType},
};

use crate::{
    Application,
    tui_menus::{Menu, MenuUpdate},
};

impl<T: Write> Application<T> {
    pub fn run_menu_title(&mut self) -> io::Result<MenuUpdate> {
        let selection = vec![
            Menu::NewGame,
            Menu::Settings,
            Menu::ScoresAndReplays {
                cursor_pos: 0,
                camera_pos: 0,
            },
            Menu::Statistics,
            Menu::About,
            Menu::Quit,
        ];
        let mut selected = 0usize;
        let mut dynamic_title_style = 1isize;
        let mut dynamic_color_offset = 0isize;
        loop {
            let w_main: usize = Self::W_MAIN.into();
            let (x_main, y_main) = Self::viewport_offset();
            let y_selection = (Self::H_MAIN / 5).saturating_sub(1);

            self.term.queue(Clear(ClearType::All))?;

            let dx_title = w_main.saturating_sub(36) / 2;

            if self.settings.tui_symbols().blocky_title_logo {
                let title_colors = [
                    "1111555  1111 1111555  5666    1111 ",
                    "   35   666      35   35526  33   33",
                    "  33   6661111  33   33  22  311113 ",
                ];
                let title_color_offsets = [
                    "0000111  3333 4444555  0111    3333 ",
                    "   01   222      45   60011  22   44",
                    "  00   2223333  44   66  11  233334 ",
                ];
                let title_unicode = [
                    "▄▄▄▄▄▄▄  ▄▄▄▄ ▄▄▄▄▄▄▄  ▄▄▄▄    ▄▄▄▄ ",
                    "   ▄▀   █▄▄      ▄▀   ▄█▄▄▀  ▄█   ▄█",
                    "  █▀   █▄▄▄▄▄▄  █▀   █▀  ▀█  ▀▄▄▄▄▀ ",
                ];
                let color_tetromino_rainbow = "1643502"
                    .chars()
                    .map(|ch| {
                        self.settings
                            .palette()
                            .get(
                                &falling_tetromino_engine::Tetromino::VARIANTS
                                    [ch.to_string().parse::<usize>().unwrap()]
                                .tile_id(),
                            )
                            .unwrap_or(&Color::Reset)
                    })
                    .copied()
                    .collect::<Vec<_>>();

                for (dy, ((t_line, c_line), co_line)) in title_unicode
                    .iter()
                    .zip(title_colors)
                    .zip(title_color_offsets)
                    .enumerate()
                {
                    for (dx, ((t_char, c_char), co_char)) in t_line
                        .chars()
                        .zip(c_line.chars())
                        .zip(co_line.chars())
                        .enumerate()
                    {
                        self.term.queue(MoveTo(
                            x_main + u16::try_from(dx_title + dx).unwrap(),
                            y_main + y_selection + u16::try_from(dy).unwrap(),
                        ))?;

                        let color = match dynamic_title_style.rem_euclid(Self::W_MAIN as isize + 2)
                        {
                            // Default title colors.
                            0 => {
                                if c_char == ' ' {
                                    Color::Reset
                                } else {
                                    *self
                                        .settings
                                        .palette()
                                        .get(
                                            &falling_tetromino_engine::Tetromino::VARIANTS
                                                [c_char.to_string().parse::<usize>().unwrap()]
                                            .tile_id(),
                                        )
                                        .unwrap_or(&Color::Reset)
                                }
                            }
                            1 => {
                                if co_char == ' ' {
                                    Color::Reset
                                } else {
                                    color_tetromino_rainbow[(co_char
                                        .to_string()
                                        .parse::<isize>()
                                        .unwrap()
                                        + dynamic_color_offset)
                                        .rem_euclid(color_tetromino_rainbow.len() as isize)
                                        as usize]
                                }
                            }
                            n => {
                                let width = n - 1;
                                color_tetromino_rainbow[(((dx + dy) as isize
                                    + dynamic_color_offset)
                                    / width)
                                    .rem_euclid(color_tetromino_rainbow.len() as isize)
                                    as usize]
                            }
                        };

                        self.term
                            .queue(PrintStyledContent(t_char.to_string().with(color)))?;
                    }
                }
            } else {
                let title_ascii = [
                    r" / /____ / /________   __ / /___ __(_)",
                    r"/ __/ -_) __/ __/ _ \ /_// __/ // / / ",
                    r"\__/\__/\__/_/  \___/    \__/\_,_/_/  ",
                ];
                // let title_ascii = [
                //     ".......  .... .......  ....    .... ",
                //     "   .°   :..      .°   .:..°  .:   .:",
                //     "  :°   :......  :°   :°  °:  °....° ",
                // ];

                //let color16_rainbow = [Color::DarkRed, Color::Red, Color::DarkYellow, Color::Yellow, Color::DarkGreen, Color::Green, Color::DarkBlue, Color::Blue, Color::DarkCyan, Color::Cyan, Color::DarkMagenta, Color::Magenta];
                let color_tetromino_rainbow = "1643502"
                    .chars()
                    .map(|ch| {
                        self.settings
                            .palette()
                            .get(
                                &falling_tetromino_engine::Tetromino::VARIANTS
                                    [ch.to_string().parse::<usize>().unwrap()]
                                .tile_id(),
                            )
                            .unwrap_or(&Color::Reset)
                    })
                    .copied()
                    .collect::<Vec<_>>();

                for (dy, bline) in title_ascii.iter().enumerate() {
                    for (dx, bchar) in bline.chars().enumerate() {
                        self.term.queue(MoveTo(
                            x_main + u16::try_from(dx_title + dx).unwrap(),
                            y_main + y_selection + u16::try_from(dy).unwrap(),
                        ))?;

                        let color = color_tetromino_rainbow[(((dx + dy) as isize
                            + dynamic_color_offset)
                            / (dynamic_title_style.rem_euclid(Self::W_MAIN as isize) + 1))
                            .rem_euclid(color_tetromino_rainbow.len() as isize)
                            as usize];

                        self.term
                            .queue(PrintStyledContent(bchar.to_string().with(color)))?;
                    }
                }
            }

            let names = selection
                .iter()
                .map(|menu| menu.str_for_list_menu_selection())
                .collect::<Vec<_>>();
            let n_names = names.len();
            for (i, name) in names.into_iter().enumerate() {
                self.term
                    .queue(MoveTo(
                        x_main,
                        y_main + y_selection + 5 + u16::try_from(i).unwrap(),
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
                    y_main + y_selection + 5 + u16::try_from(n_names).unwrap() + 2,
                ))?
                .queue(PrintStyledContent(
                    format!("{:^w_main$}", "[Enter/Esc/Del/←↓↑→] or Vim,",).italic(),
                ))?
                .queue(MoveTo(
                    x_main,
                    y_main + y_selection + 5 + u16::try_from(n_names).unwrap() + 3,
                ))?
                .queue(PrintStyledContent(
                    format!("{:^w_main$}", "press [?] to view keybinds anytime",).italic(),
                ))?;

            self.term.flush()?;

            // Wait for new input.
            match event::read()? {
                // Abort program.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c' | 'C'),
                    modifiers: KeyModifiers::CONTROL,
                    kind: KeyEventKind::Press | KeyEventKind::Repeat,
                    ..
                }) => break Ok(MenuUpdate::Push(Menu::Quit)),

                // Keybinds help menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('?'),
                    kind: KeyEventKind::Press | KeyEventKind::Repeat,
                    ..
                }) => {
                    let client_menu_name = "Title screen";
                    let legend = vec![
                        (
                            "Normal keybinds".to_owned(),
                            [
                                ("Enter e", "Select"),
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
                                    "Reload app from savefile (overwrites current data!)",
                                ),
                                (
                                    "Ctrl+Alt+S",
                                    "Perform savefile store (respects save preferences)",
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

                // 'Quit menu' => Move cursor to 'quit' position, do not actually quit. This avoid spamming / double-clicking q or so and accidentally leaving, which might erase progress.
                Event::Key(KeyEvent {
                    code: KeyCode::Esc | KeyCode::Char('q' | 'Q') | KeyCode::Backspace,
                    kind: KeyEventKind::Press,
                    ..
                }) => {
                    selected = selection.len() - 1;
                }

                // Select next menu.
                Event::Key(KeyEvent {
                    code: KeyCode::Enter | KeyCode::Char('e' | 'E'),
                    kind: KeyEventKind::Press,
                    ..
                }) if !selection.is_empty() => {
                    let menu = selection.into_iter().nth(selected).unwrap();
                    break Ok(MenuUpdate::Push(menu));
                }

                // Move selector up.
                Event::Key(KeyEvent {
                    code: KeyCode::Up | KeyCode::Char('k' | 'K'),
                    kind: KeyEventKind::Press | KeyEventKind::Repeat,
                    ..
                }) => {
                    selected += selection.len() - 1;
                    dynamic_color_offset += 1;
                }

                // Move selector down.
                Event::Key(KeyEvent {
                    code: KeyCode::Down | KeyCode::Char('j' | 'J'),
                    kind: KeyEventKind::Press | KeyEventKind::Repeat,
                    ..
                }) => {
                    selected += 1;
                    dynamic_color_offset -= 1;
                }

                // Move l.
                Event::Key(KeyEvent {
                    code: KeyCode::Left | KeyCode::Char('h' | 'H'),
                    kind: KeyEventKind::Press | KeyEventKind::Repeat,
                    ..
                }) => {
                    dynamic_title_style -= 1;
                }

                // Reload from savefile.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('l' | 'L'),
                    modifiers,
                    kind: KeyEventKind::Press | KeyEventKind::Repeat,
                    ..
                }) if { modifiers.contains(KeyModifiers::CONTROL.union(KeyModifiers::ALT)) } => {
                    self.temp_data.loadfile_result = self.savefile_load();
                }

                // Store to savefile.
                Event::Key(KeyEvent {
                    code: KeyCode::Char('s' | 'S'),
                    modifiers,
                    kind: KeyEventKind::Press | KeyEventKind::Repeat,
                    ..
                }) if { modifiers.contains(KeyModifiers::CONTROL.union(KeyModifiers::ALT)) } => {
                    self.temp_data.storefile_result = self.savefile_store();
                }

                // Move r.
                Event::Key(KeyEvent {
                    code: KeyCode::Right | KeyCode::Char('l' | 'L'),
                    kind: KeyEventKind::Press | KeyEventKind::Repeat,
                    ..
                }) => {
                    dynamic_title_style += 1;
                }

                // Other event: don't care.
                _ => {}
            }
            selected = selected.rem_euclid(selection.len());
        }
    }
}
