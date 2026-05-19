use std::num::NonZeroU32;

use crate::core_game_engine::{
    BOARD_WIDTH, Game, GameAccess, GameBuilder, GameEndCause, GameModifier, Line, MiscPceRots,
    MiscTetGens, NotificationFeed, PLAYABLE_BOARD_HEIGHT, Phase, Tetromino, TileType,
};

use crate::savefile_logic::to_savefile_string;

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct Combo {
    // Configuration/reproducibility fields.
    config: ComboConfig,

    // Stateful fields.
    height_loaded: usize,
    cached_display_values: [(String, String); 1],
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct ComboConfig {
    /// Custom starting layout when playing Combo mode (4-wide rows), encoded as binary.
    /// Example: '▀▄▄▀' => 0b_1001_0110 = 150
    pub start_layout: u16,
    pub limit: Option<NonZeroU32>,
}

impl Default for ComboConfig {
    fn default() -> Self {
        Self {
            start_layout: Combo::LAYOUTS[0],
            limit: NonZeroU32::try_from(24).ok(),
        }
    }
}

impl Combo {
    pub const MOD_ID: &str = stringify!(Combo);

    pub fn build(builder: &GameBuilder, config: ComboConfig) -> Game {
        let modifier = Box::new(Self {
            config,
            height_loaded: 0,
            cached_display_values: [("Current Combo".to_string(), 0.to_string())],
        });

        builder.build_modded(vec![modifier])
    }
}

impl GameModifier<MiscTetGens, MiscPceRots, TileType> for Combo {
    fn id(&self) -> String {
        Self::MOD_ID.to_owned()
    }

    fn cfg(&self) -> String {
        to_savefile_string(&self.config).unwrap()
    }

    fn values(&self) -> &[(String, String)] {
        &self.cached_display_values
    }

    fn try_clone(
        &self,
    ) -> Result<Box<dyn GameModifier<MiscTetGens, MiscPceRots, TileType>>, String> {
        Ok(Box::new(self.clone()))
    }

    // Initialize board.
    fn on_game_built(&mut self, game: GameAccess) {
        game.state
            .board
            .resize(Self::PREGENERATED_HEIGHT, Default::default());
        for ((line, _is_frozen), four_well_line) in game
            .state
            .board
            .iter_mut()
            .take(Self::PREGENERATED_HEIGHT)
            .zip(Self::combo_lines(&mut self.height_loaded))
        {
            *line = four_well_line;
        }

        let mut y = 0;
        let mut layout = self.config.start_layout;
        while layout != 0 {
            if layout & 0b1000 != 0 {
                game.state.board[y].0[3] = Some(TileType::Generic);
            }
            if layout & 0b0100 != 0 {
                game.state.board[y].0[4] = Some(TileType::Generic);
            }
            if layout & 0b0010 != 0 {
                game.state.board[y].0[5] = Some(TileType::Generic);
            }
            if layout & 0b0001 != 0 {
                game.state.board[y].0[6] = Some(TileType::Generic);
            }

            layout /= 0b1_0000;
            y += 1;
        }
    }

    // Check game condition.
    fn on_lock_post(&mut self, game: GameAccess, _feed: &mut NotificationFeed) {
        // If combo broken.
        if game.state.consecutive_lineclears == 0 {
            *game.phase = Phase::GameEnd {
                cause: GameEndCause::Custom("Combo broken".to_owned()),
                is_win: false,
            };
        }
    }

    // Insert new line.
    fn on_lines_clear_post(&mut self, game: GameAccess, _feed: &mut NotificationFeed) {
        if let Some(limit) = self.config.limit
            && game.state.consecutive_lineclears >= limit.get()
        {
            *game.phase = Phase::GameEnd {
                cause: GameEndCause::Custom("Combo reached".to_owned()),
                is_win: true,
            };
            return;
        }

        game.state.board.push((
            Self::combo_lines(&mut self.height_loaded).next().unwrap(),
            false,
        ));

        // Overwrite with combo length.
        self.cached_display_values[0].1 = game.state.consecutive_lineclears.to_string();
    }
}

impl Combo {
    pub const LAYOUTS: [u16; 5] = [
        0b0000_0000_1100_1000, // "r "
        0b0000_0000_0000_1110, // "_ "
        0b0000_1100_1000_1011, // "f _"
        0b0000_1100_1000_1101, // "k ."
        0b1000_1000_1000_1101, // "L ."
                               /*0b0000_1001_1001_1001, // "I I"
                               0b0001_0001_1001_1100, // "l i"
                               0b1000_1000_1100_1100, // "b"
                               0b0000_0000_1110_1011, // "rl"*/
    ];

    const PREGENERATED_HEIGHT: usize = PLAYABLE_BOARD_HEIGHT + 4;

    fn combo_lines<'a>(height_loaded: &'a mut usize) -> impl Iterator<Item = Line> + 'a {
        let rainbow_tiles = [
            Tetromino::Z,
            Tetromino::L,
            Tetromino::O,
            Tetromino::S,
            Tetromino::I,
            Tetromino::J,
            Tetromino::T,
        ]
        .map(|tet| Some(TileType::Tet(tet)));

        let color_tiles_0 = (*height_loaded..).map(move |i| rainbow_tiles[i / 3 % 7]);
        let color_tiles_1 = color_tiles_0.clone().skip(1);

        color_tiles_0
            .zip(color_tiles_1)
            .map(move |(color_tile_0, color_tile_1)| {
                let mut line = [None; BOARD_WIDTH];
                line[0] = color_tile_0;
                line[1] = color_tile_1;
                line[2] = Some(TileType::Generic);
                line[7] = Some(TileType::Generic);
                line[8] = color_tile_1;
                line[9] = color_tile_0;

                *height_loaded += 1;
                line
            })
    }
}
