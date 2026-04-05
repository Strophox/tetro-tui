use falling_tetromino_engine::{Board, Game, GameAccess, GameBuilder, GameModifier};

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct StartBoard {
    encoded_board: String,
}

impl StartBoard {
    pub const MOD_ID: &str = stringify!(StartBoard);

    pub fn build(builder: &GameBuilder, encoded_board: String) -> Game {
        let modifier = Box::new(Self { encoded_board });

        builder.clone().build_modded(vec![modifier])
    }
}

impl GameModifier for StartBoard {
    fn id(&self) -> String {
        Self::MOD_ID.to_owned()
    }

    fn args(&self) -> String {
        serde_json::to_string(&self.encoded_board).unwrap()
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
        let grey_tile = Some(std::num::NonZeroU8::try_from(254).unwrap());

        let mut new_board = Board::default();

        let mut chars = board_str.chars();

        'lines: for line in &mut new_board {
            'tiles: for tile in line {
                'chars: for char in chars.by_ref() {
                    if char == '/' {
                        // Slash = jump to next line (i.e. above).
                        continue 'lines;
                    } else if char == '\n' {
                        // Newline = ignore, stay at tile but move on to next char.
                        continue 'chars;
                    } else if char == ' ' {
                        // Space = empty tile.
                        *tile = None;
                        continue 'tiles;
                    } else {
                        // Otherwise = filled tile.
                        *tile = grey_tile;
                        continue 'tiles;
                    }
                }
            }
        }

        new_board
    }
}
