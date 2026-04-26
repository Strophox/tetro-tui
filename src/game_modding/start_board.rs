use falling_tetromino_engine::{Board, Game, GameAccess, GameBuilder, GameModifier};

use crate::{savefile_logic::to_savefile_string, tui_settings::Palette};

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct StartBoard {
    // Modifier configuration.
    encoded_board: String,
}

impl StartBoard {
    pub const MOD_ID: &str = stringify!(StartBoard);

    pub fn build(builder: &GameBuilder, encoded_board: String) -> Game {
        let modifier = Box::new(Self { encoded_board });

        builder.build_modded(vec![modifier])
    }
}

impl GameModifier for StartBoard {
    fn id(&self) -> String {
        Self::MOD_ID.to_owned()
    }

    fn cfg(&self) -> String {
        to_savefile_string(&self.encoded_board).unwrap()
    }

    fn stats(&self) -> &[String] {
        &[]
    }

    fn try_clone(&self) -> Result<Box<dyn GameModifier>, String> {
        Ok(Box::new(self.clone()))
    }

    fn on_game_built(&mut self, game: GameAccess) {
        let start_board = Self::decode_board(self.encoded_board.as_str());

        game.state.board = start_board;
    }
}

impl StartBoard {
    #[allow(dead_code)]
    pub fn encode_board(board: &Board) -> String {
        board
            .iter()
            .map(|line| {
                line.iter()
                    .map(|tile| if tile.is_some() { 'O' } else { ' ' })
                    .collect::<String>()
            })
            .collect::<String>()
            .trim_end()
            .to_owned()
    }

    pub fn decode_board(board_str: &str) -> Board {
        let mut new_board = Board::default();

        let mut chars = board_str.chars();

        'lines: for line in &mut new_board {
            'tiles: for tile in line {
                'chars: for char in chars.by_ref() {
                    if char == '/' {
                        // Skip to next line.
                        continue 'lines;
                    } else if char == '\n' {
                        // Ignore newline chars.
                        continue 'chars;
                    } else if char == ' ' || char == '.' || char == '_' {
                        // Empty tile found.
                        *tile = None;
                        continue 'tiles;
                    } else {
                        // Filled tile found. (falltrough)
                        *tile = Some(Palette::GRAY);
                        continue 'tiles;
                    }
                }
            }
        }

        new_board
    }
}
