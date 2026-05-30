use falling_tetromino_engine::BOARD_WIDTH;
use rand::RngExt;

use crate::core_game_engine::{
    GameAccess, GameModifier, MiscPceRots, MiscTetGens, NotificationFeed, Phase, Tetromino,
    TileType,
};

// This modifier does not have fields for configuration/reproducibility.
// This modifier does not have fields for keeping state.
#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct PlacementPractice;

impl PlacementPractice {
    pub const MOD_ID: &str = "PlacementPractice";

    pub fn new() -> Self {
        PlacementPractice
    }
}

impl GameModifier<MiscTetGens, MiscPceRots, TileType> for PlacementPractice {
    fn id(&self) -> String {
        Self::MOD_ID.to_owned()
    }

    fn cfg(&self) -> String {
        "".to_owned()
    }

    fn values(&self) -> &[(String, String)] {
        &[]
    }

    fn try_clone(
        &self,
    ) -> Result<Box<dyn GameModifier<MiscTetGens, MiscPceRots, TileType>>, String> {
        Ok(Box::new(self.clone()))
    }

    fn on_spawn_post(&mut self, game: GameAccess, _feed: &mut NotificationFeed) {
        let Phase::PieceInPlay { piece, .. } = game.phase else {
            return;
        };
        let rand_orient = game.state.rng.random_range(0..=3);
        let template_lines: &[&str] = match piece.tetromino {
            Tetromino::O => &[
                "77777777  77777777",
                "       7  7       ",
                "        00        ",
            ],
            Tetromino::I => match rand_orient {
                0 | 2 => &[
                    // "                ",
                    "777777    777777",
                    "      1111      ",
                    // "                ",
                ],
                _ => &[
                    "777777771 177777777",
                    "777777771 177777777",
                    "777777771 177777777",
                    "777777771 177777777",
                ],
            },
            Tetromino::S => match rand_orient {
                0 | 2 => &[
                    "7777777  27777777",
                    "       22        ",
                    // "                 ",
                ],
                _ => &[
                    "77777777  77777777",
                    "        2 7       ",
                    "         2        ",
                ],
            },
            Tetromino::Z => match rand_orient {
                0 | 2 => &[
                    "77777773  7777777",
                    "        33       ",
                    // "                 ",
                ],
                _ => &[
                    "77777777  77777777",
                    "       7 3        ",
                    "        3         ",
                ],
            },
            Tetromino::T => match rand_orient {
                0 => &[
                    "7777777   7777777",
                    "       444       ",
                    // "                 ",
                ],
                1 => &[
                    "77777777  77777777",
                    "       7 4        ",
                    "        4         ",
                ],
                2 => &[
                    "7777777   7777777",
                    "       4 4       ",
                    "        4        ",
                ],
                _ => &[
                    "77777777  77777777",
                    "        4 7       ",
                    "         4        ",
                ],
            },
            Tetromino::L => match rand_orient {
                0 => &[
                    "7777777   7777777",
                    "       555       ",
                    // "                 ",
                ],
                1 => &[
                    "77777777  77777777",
                    "        55        ",
                    // "                  ",
                ],
                2 => &[
                    "7777777   7777777",
                    "      7 55       ",
                    "       5         ",
                ],
                _ => &[
                    "77777777  77777777",
                    "        5 7       ",
                    "        5 7       ",
                ],
            },
            Tetromino::J => match rand_orient {
                0 => &[
                    "7777777   7777777",
                    "       666       ",
                    // "                 ",
                ],
                1 => &[
                    "77777777  77777777",
                    "       7 6        ",
                    "       7 6        ",
                ],
                2 => &[
                    "7777777   7777777",
                    "       66 7      ",
                    "         6       ",
                ],
                _ => &[
                    "77777777  77777777",
                    "        66        ",
                    // "                  ",
                ],
            },
        };

        game.state.board = vec![Default::default(); template_lines.len()];

        // Determine a consistent offset for the pattern, according to how much leeway over the width we have.
        let max_offset = template_lines[0].len() - BOARD_WIDTH;
        let offset = if max_offset == 0 {
            0
        } else {
            game.state.rng.random_range(0..=max_offset)
        };
        for ((game_line, _is_frozen), template_line) in
            game.state.board.iter_mut().zip(template_lines.iter().rev())
        {
            for (game_tile, ch) in game_line.iter_mut().zip(template_line[offset..].chars()) {
                *game_tile = ch
                    .to_digit(TileType::VARIANTS.len() as u32)
                    .map(|digit| TileType::VARIANTS[digit as usize]);
            }
        }
    }
}
