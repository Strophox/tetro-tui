use std::num::{NonZeroU32, NonZeroU8};

use falling_tetromino_engine::{
    Game, GameAccess, GameBuilder, GameEndCause, GameLimits, GameModifier, Line, NotificationFeed,
    Phase, Stat, Tetromino,
};
#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct Combo {
    initial_layout: u16,
    combo_limit: Option<NonZeroU32>,

    height_loaded: usize,
}

impl Combo {
    pub const MOD_ID: &str = stringify!(Combo);

    pub fn build(
        builder: &GameBuilder,
        initial_layout: u16,
        combo_limit: Option<NonZeroU32>,
    ) -> Game {
        let modifier = Box::new(Self {
            initial_layout,
            combo_limit,
            height_loaded: 0,
        });

        builder
            .clone()
            .game_limits(match combo_limit {
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

    fn args(&self) -> String {
        serde_json::to_string(&(self.initial_layout, self.combo_limit)).unwrap()
    }

    fn try_clone(&self) -> Result<Box<dyn GameModifier>, String> {
        Ok(Box::new(self.clone()))
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

        let grey_tile = Some(NonZeroU8::try_from(254).unwrap());

        let mut y = 0;
        let mut layout = self.initial_layout;
        while layout != 0 {
            if layout & 0b1000 != 0 {
                game.state.board[y][3] = grey_tile;
            }
            if layout & 0b0100 != 0 {
                game.state.board[y][4] = grey_tile;
            }
            if layout & 0b0010 != 0 {
                game.state.board[y][5] = grey_tile;
            }
            if layout & 0b0001 != 0 {
                game.state.board[y][6] = grey_tile;
            }

            layout /= 0b1_0000;
            y += 1;
        }
    }

    // Check game condition.
    fn on_lock_post(&mut self, game: GameAccess, _feed: &mut NotificationFeed) {
        // If combo broken.
        if game.state.consecutive_line_clears == 0 {
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
        game.state.points = game.state.consecutive_line_clears;
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
        let color_tiles = [
            Tetromino::Z,
            Tetromino::L,
            Tetromino::O,
            Tetromino::S,
            Tetromino::I,
            Tetromino::J,
            Tetromino::T,
        ]
        .map(|tet| Some(tet.tile_id()));

        let grey_tile = Some(NonZeroU8::try_from(254).unwrap());

        let color_tiles_0 = (*height_loaded..).map(move |i| color_tiles[i / 2 % 7]);

        let color_tiles_1 = color_tiles_0.clone().skip(1);

        color_tiles_0
            .zip(color_tiles_1)
            .map(move |(color_tile_0, color_tile_1)| {
                let mut line = [None; Game::WIDTH];
                line[0] = color_tile_0;
                line[1] = color_tile_1;
                line[2] = grey_tile;
                line[7] = grey_tile;
                line[8] = color_tile_1;
                line[9] = color_tile_0;

                *height_loaded += 1;
                line
            })
    }
}
