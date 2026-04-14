use crossterm::{cursor, style, terminal, QueueableCommand};

use super::*;

// "|⠁|⠂|⠄|⠈|⠐|⠠|⡀|⢀|"
const BRAILLE: &str = "⠀⠁⠂⠃⠄⠅⠆⠇⠈⠉⠊⠋⠌⠍⠎⠏⠐⠑⠒⠓⠔⠕⠖⠗⠘⠙⠚⠛⠜⠝⠞⠟⠠⠡⠢⠣⠤⠥⠦⠧⠨⠩⠪⠫⠬⠭⠮⠯⠰⠱⠲⠳⠴⠵⠶⠷⠸⠹⠺⠻⠼⠽⠾⠿⡀⡁⡂⡃⡄⡅⡆⡇⡈⡉⡊⡋⡌⡍⡎⡏⡐⡑⡒⡓⡔⡕⡖⡗⡘⡙⡚⡛⡜⡝⡞⡟⡠⡡⡢⡣⡤⡥⡦⡧⡨⡩⡪⡫⡬⡭⡮⡯⡰⡱⡲⡳⡴⡵⡶⡷⡸⡹⡺⡻⡼⡽⡾⡿⢀⢁⢂⢃⢄⢅⢆⢇⢈⢉⢊⢋⢌⢍⢎⢏⢐⢑⢒⢓⢔⢕⢖⢗⢘⢙⢚⢛⢜⢝⢞⢟⢠⢡⢢⢣⢤⢥⢦⢧⢨⢩⢪⢫⢬⢭⢮⢯⢰⢱⢲⢳⢴⢵⢶⢷⢸⢹⢺⢻⢼⢽⢾⢿⣀⣁⣂⣃⣄⣅⣆⣇⣈⣉⣊⣋⣌⣍⣎⣏⣐⣑⣒⣓⣔⣕⣖⣗⣘⣙⣚⣛⣜⣝⣞⣟⣠⣡⣢⣣⣤⣥⣦⣧⣨⣩⣪⣫⣬⣭⣮⣯⣰⣱⣲⣳⣴⣵⣶⣷⣸⣹⣺⣻⣼⣽⣾⣿";

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, Default)]
pub struct BrailleRenderer {
    x: u16,
    y: u16,
    w: u16,
    h: u16,
}

impl Renderer for BrailleRenderer {
    fn update_feed(
        &mut self,
        _feed: impl IntoIterator<Item = (Notification, InGameTime)>,
        _settings: &Settings,
    ) {
        // We do not use/display feedback_msg-related things for this renderer at this time.
    }

    fn reset_veffects_state(&mut self) {
        // We do not store any effects state associated with the game at this time.
    }

    fn reset_viewport_state_with_offset_and_area(
        &mut self,
        (x, y): (u16, u16),
        (w, h): (u16, u16),
    ) {
        self.x = x;
        self.y = y;
        self.w = w;
        self.h = h;
    }

    fn render<T: Write>(
        &mut self,
        term: &mut T,
        game: &Game,
        _meta_data: &GameMetaData,
        _settings: &Settings,
        _temp_data: &TemporaryAppData,
        _keybinds_legend: &KeybindsLegend,
        _replay_extra: Option<(InGameTime, f64)>,
    ) -> io::Result<()> {
        let mut board = game.state().board;
        if let Some(piece) = game.phase().piece() {
            for ((x, y), tile_id) in piece.tiles() {
                board[y as usize][x as usize] = Some(tile_id);
            }
        }

        let braille = BRAILLE.chars().collect::<Vec<char>>();
        let (delim_l, delim_r) = ('▐', '▌'); //'░';

        let btxt_lines = [
            [19, 18, 17, 16],
            [15, 14, 13, 12],
            [11, 10, 9, 8],
            [7, 6, 5, 4],
            [3, 2, 1, 0],
        ]
        .iter()
        .map(|[i0, i1, i2, i3]| {
            let [l0, l1, l2, l3] = [board[*i0], board[*i1], board[*i2], board[*i3]];
            [[0, 1], [2, 3], [4, 5], [6, 7], [8, 9]]
                .iter()
                .map(|[j0, j1]| {
                    let b0 = if l0[*j0].is_some() { 1 } else { 0 };
                    let b1 = if l1[*j0].is_some() { 2 } else { 0 };
                    let b2 = if l2[*j0].is_some() { 4 } else { 0 };
                    let b3 = if l3[*j0].is_some() { 64 } else { 0 };
                    let b4 = if l0[*j1].is_some() { 8 } else { 0 };
                    let b5 = if l1[*j1].is_some() { 16 } else { 0 };
                    let b6 = if l2[*j1].is_some() { 32 } else { 0 };
                    let b7 = if l3[*j1].is_some() { 128 } else { 0 };
                    braille[b0 + b1 + b2 + b3 + b4 + b5 + b6 + b7]
                })
                .collect::<String>()
        });

        term.queue(terminal::Clear(terminal::ClearType::All))?;

        let (w_render, h_render) = (1 + 5 + 1, 5);
        let (x_render, y_render) = (
            self.x + self.w.saturating_sub(w_render) / 2,
            self.y + self.h.saturating_sub(h_render) / 2,
        );

        for (dy, b_line) in btxt_lines.enumerate() {
            term.queue(cursor::MoveTo(
                x_render,
                y_render + u16::try_from(dy).unwrap(),
            ))?
            .queue(style::Print(format!("{delim_l}{b_line}{delim_r}")))?;
        }

        term.flush()?;

        Ok(())
    }
}
