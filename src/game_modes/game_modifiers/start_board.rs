use falling_tetromino_engine::{Game, GameAccess, GameBuilder, GameModifier};

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
        let start_board =
            crate::application::NewGameSettings::decode_board(self.encoded_board.as_str());

        game.state.board = start_board;
    }
}
