use falling_tetromino_engine::{GameAccess, GameModifier, NotificationFeed};

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct DisplayFinesse {
    // This modifier does not have fields for configuration/reproducibility.

    // Stateful fields.
    player_inputs_counted: u32,
    cached_stats: [String; 1],
}

impl DisplayFinesse {
    pub const MOD_ID: &str = stringify!(DisplayFinesse);

    pub fn modifier() -> Box<dyn GameModifier> {
        let modifier = DisplayFinesse {
            player_inputs_counted: 0,
            cached_stats: [Self::fmt_finesse(0)],
        };
        Box::new(modifier)
    }
}

impl GameModifier for DisplayFinesse {
    fn id(&self) -> String {
        Self::MOD_ID.to_owned()
    }

    fn cfg(&self) -> String {
        "".to_owned()
    }

    fn try_clone(&self) -> Result<Box<dyn GameModifier>, String> {
        Ok(Box::new(self.clone()))
    }

    fn stats(&self) -> &[String] {
        &self.cached_stats
    }

    fn on_receive_player_input(
        &mut self,
        _game: GameAccess,
        _feed: &mut NotificationFeed,
        _time: &mut falling_tetromino_engine::InGameTime,
        player_input: &mut Option<falling_tetromino_engine::Input>,
    ) {
        if matches!(
            player_input,
            Some(falling_tetromino_engine::Input::Activate(_))
        ) {
            self.player_inputs_counted += 1;
            self.cached_stats[0] = Self::fmt_finesse(self.player_inputs_counted);
        }
    }
}

impl DisplayFinesse {
    fn fmt_finesse(num: u32) -> String {
        format!("Finesse: {num}")
    }
}
