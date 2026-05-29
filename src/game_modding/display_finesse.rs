use crate::core_game_engine::{
    GameAccess, GameModifier, MiscPceRots, MiscTetGens, NotificationFeed, TileType,
};

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct DisplayFinesse {
    // This modifier does not have fields for configuration/reproducibility.

    // Stateful fields.
    player_inputs_counted: u32,
    cached_display_values: [(String, String); 1],
}

impl DisplayFinesse {
    pub const MOD_ID: &str = "DisplayFinesse";

    pub fn modifier() -> Box<dyn GameModifier<MiscTetGens, MiscPceRots, TileType>> {
        let modifier = DisplayFinesse {
            player_inputs_counted: 0,
            cached_display_values: [("Finesse".to_owned(), 0.to_string())],
        };
        Box::new(modifier)
    }
}

impl GameModifier<MiscTetGens, MiscPceRots, TileType> for DisplayFinesse {
    fn id(&self) -> String {
        Self::MOD_ID.to_owned()
    }

    fn cfg(&self) -> String {
        "".to_owned()
    }

    fn try_clone(
        &self,
    ) -> Result<Box<dyn GameModifier<MiscTetGens, MiscPceRots, TileType>>, String> {
        Ok(Box::new(self.clone()))
    }

    fn values(&self) -> &[(String, String)] {
        &self.cached_display_values
    }

    fn on_receive_player_input(
        &mut self,
        _game: GameAccess,
        _feed: &mut NotificationFeed,
        _time: &mut crate::core_game_engine::InGameTime,
        player_input: &mut Option<crate::core_game_engine::Input>,
    ) {
        if matches!(
            player_input,
            Some(crate::core_game_engine::Input::Activate(_))
        ) {
            self.player_inputs_counted += 1;
            self.cached_display_values[0].1 = self.player_inputs_counted.to_string();
        }
    }
}
