use crossterm::{cursor, style, terminal, QueueableCommand};

use super::*;

// "|⠁|⠂|⠄|⠈|⠐|⠠|⡀|⢀|"
const BRAILLE: &str = "⠀⠁⠂⠃⠄⠅⠆⠇⠈⠉⠊⠋⠌⠍⠎⠏⠐⠑⠒⠓⠔⠕⠖⠗⠘⠙⠚⠛⠜⠝⠞⠟⠠⠡⠢⠣⠤⠥⠦⠧⠨⠩⠪⠫⠬⠭⠮⠯⠰⠱⠲⠳⠴⠵⠶⠷⠸⠹⠺⠻⠼⠽⠾⠿⡀⡁⡂⡃⡄⡅⡆⡇⡈⡉⡊⡋⡌⡍⡎⡏⡐⡑⡒⡓⡔⡕⡖⡗⡘⡙⡚⡛⡜⡝⡞⡟⡠⡡⡢⡣⡤⡥⡦⡧⡨⡩⡪⡫⡬⡭⡮⡯⡰⡱⡲⡳⡴⡵⡶⡷⡸⡹⡺⡻⡼⡽⡾⡿⢀⢁⢂⢃⢄⢅⢆⢇⢈⢉⢊⢋⢌⢍⢎⢏⢐⢑⢒⢓⢔⢕⢖⢗⢘⢙⢚⢛⢜⢝⢞⢟⢠⢡⢢⢣⢤⢥⢦⢧⢨⢩⢪⢫⢬⢭⢮⢯⢰⢱⢲⢳⢴⢵⢶⢷⢸⢹⢺⢻⢼⢽⢾⢿⣀⣁⣂⣃⣄⣅⣆⣇⣈⣉⣊⣋⣌⣍⣎⣏⣐⣑⣒⣓⣔⣕⣖⣗⣘⣙⣚⣛⣜⣝⣞⣟⣠⣡⣢⣣⣤⣥⣦⣧⣨⣩⣪⣫⣬⣭⣮⣯⣰⣱⣲⣳⣴⣵⣶⣷⣸⣹⣺⣻⣼⣽⣾⣿";

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
pub struct BrailleRenderer {
    x_draw: usize,
    y_draw: usize,
}

impl Renderer for BrailleRenderer {
    fn push_game_notification_feed(
        &mut self,
        _feed: impl IntoIterator<Item = (Notification, InGameTime)>,
    ) {
        // We do not use/display feedback_msg-related things for this renderer at this time.
    }

    fn reset_game_associated_state(&mut self) {
        // We do not store any state associated with the game at this time.
    }

    fn reset_view_diff_state(&mut self) {
        // We do not implement diff'ing for this renderer at this time.
    }

    fn set_render_offset(&mut self, x: usize, y: usize) {
        self.x_draw = x;
        self.y_draw = y;
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

        let (w_term, h_term) = terminal::size()?;
        let (w_render, h_render) = (1 + 5 + 1, 5);

        let (x_render, y_render) = (
            w_term.saturating_sub(w_render) / 2,
            h_term.saturating_sub(h_render) / 2,
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
