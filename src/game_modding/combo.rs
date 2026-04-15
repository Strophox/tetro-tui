use std::num::NonZeroU32;

use falling_tetromino_engine::{
    Game, GameAccess, GameBuilder, GameEndCause, GameLimits, GameModifier, Line, NotificationFeed,
    Phase, Stat, Tetromino,
};

use crate::tui_settings::Palette;

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct Combo {
    // Modifier configuration.
    config: ComboConfig,
    // Modifier state fields.
    height_loaded: usize,
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
            limit: NonZeroU32::try_from(30).ok(),
        }
    }
}

impl Combo {
    pub const MOD_ID: &str = stringify!(Combo);

    pub fn build(builder: &GameBuilder, config: ComboConfig) -> Game {
        let modifier = Box::new(Self {
            config,
            height_loaded: 0,
        });

        builder
            .clone()
            .game_limits(match config.limit {
                Some(c) => GameLimits::single(Stat::PointsScored(c.get()), true),
                None => GameLimits::new(),
            })
            .build_modded(vec![modifier])
    }
}

impl GameModifier for Combo {
    fn id(&self) -> String {
        Self::MOD_ID.to_owned()
    }

    fn cfg(&self) -> String {
        serde_json::to_string(&self.config).unwrap()
    }

    fn try_clone(&self) -> Result<Box<dyn GameModifier>, String> {
        Ok(Box::new(*self))
    }

    // Initialize board.
    fn on_game_built(&mut self, game: GameAccess) {
        for (line, four_well_line) in game
            .state
            .board
            .iter_mut()
            .take(Game::HEIGHT)
            .zip(Self::combo_lines(&mut self.height_loaded))
        {
            *line = four_well_line;
        }

        let mut y = 0;
        let mut layout = self.config.start_layout;
        while layout != 0 {
            if layout & 0b1000 != 0 {
                game.state.board[y][3] = Some(Palette::GRAY);
            }
            if layout & 0b0100 != 0 {
                game.state.board[y][4] = Some(Palette::GRAY);
            }
            if layout & 0b0010 != 0 {
                game.state.board[y][5] = Some(Palette::GRAY);
            }
            if layout & 0b0001 != 0 {
                game.state.board[y][6] = Some(Palette::GRAY);
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
        game.state.board[Game::HEIGHT - 1] =
            Self::combo_lines(&mut self.height_loaded).next().unwrap();

        // Overwrite game score with combo length.
        // FIXME: Proper solution for displaying progress instead of overwriting score?
        game.state.points = game.state.consecutive_lineclears;
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
        .map(|tet| Some(tet.tile_id()));

        let color_tiles_0 = (*height_loaded..).map(move |i| rainbow_tiles[i / 2 % 7]);
        let color_tiles_1 = color_tiles_0.clone().skip(1);

        color_tiles_0
            .zip(color_tiles_1)
            .map(move |(color_tile_0, color_tile_1)| {
                let mut line = [None; Game::WIDTH];
                line[0] = color_tile_0;
                line[1] = color_tile_1;
                line[2] = Some(Palette::GRAY);
                line[7] = Some(Palette::GRAY);
                line[8] = color_tile_1;
                line[9] = color_tile_0;

                *height_loaded += 1;
                line
            })
    }
}
