use crate::core_game_engine::{
    Board, GameAccess, GameModifier, Line, MiscPceRots, MiscTetGens, TileType,
};

use crate::savefile_logic::to_savefile_string;

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct StartBoard {
    // Configuration/reproducibility fields.
    encoded_board: String,
    // This modifier does not have fields for keeping state.
}

impl StartBoard {
    pub const MOD_ID: &str = "StartBoard";

    pub fn modifier(
        encoded_board: String,
    ) -> Box<dyn GameModifier<MiscTetGens, MiscPceRots, TileType>> {
        let modifier = Self { encoded_board };
        Box::new(modifier)
    }
}

impl GameModifier<MiscTetGens, MiscPceRots, TileType> for StartBoard {
    fn id(&self) -> String {
        Self::MOD_ID.to_owned()
    }

    fn cfg(&self) -> String {
        to_savefile_string(&self.encoded_board).unwrap()
    }

    fn values(&self) -> &[(String, String)] {
        &[]
    }

    fn try_clone(
        &self,
    ) -> Result<Box<dyn GameModifier<MiscTetGens, MiscPceRots, TileType>>, String> {
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
            .map(|(line, _is_frozen)| {
                line.iter()
                    .map(|tile| if tile.is_some() { 'O' } else { ' ' })
                    .collect::<String>()
            })
            .collect::<String>()
            .trim_end()
            .to_owned()
    }

    pub fn decode_board(board_str: &str) -> Board {
        Vec::from_iter(Self::decoded_lines(board_str))
    }

    pub fn decoded_lines<'a>(board_str: &'a str) -> impl Iterator<Item = (Line, bool)> + 'a {
        let mut chars = board_str.chars();
        let mut done = false;
        std::iter::from_fn(move || {
            if done {
                return None;
            }
            let mut line = Line::default();
            'tiles: for tile in &mut line {
                'chars: for char in chars.by_ref() {
                    if char == '/' {
                        // Skip to next line.
                        break 'tiles;
                    } else if char == '\n' {
                        // Ignore newline chars.
                        continue 'chars;
                    } else if char == ' ' || char == '.' || char == '_' {
                        // Empty tile found.
                        *tile = None;
                        continue 'tiles;
                    } else {
                        // Filled tile found. (falltrough)
                        *tile = Some(TileType::Generic);
                        continue 'tiles;
                    }
                }
                // We ran out of 'chars, return last line.
                // FIXME: Ensure we don't return an unnecessary empty line one past the end of the input string. We currently allow this because for most use cases this will not matter unless some mod pushes some line at the top and this creates a gap.
                done = true;
                break 'tiles;
            }
            Some((line, false))
        })
    }
}
