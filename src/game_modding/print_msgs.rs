use crate::core_game_engine::{
    GameAccess, GameModifier, InGameTime, MiscPceRots, MiscTetGens, Notification, NotificationFeed,
    TileType,
};

use crate::savefile_logic::to_savefile_string;

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct PrintMsgs {
    // Configuration/reproducibility fields.
    messages: Vec<String>,

    // Stateful fields.
    init: bool,
}

impl PrintMsgs {
    pub const MOD_ID: &str = "PrintMsgs";

    pub fn with_msgs(messages: Vec<String>) -> Self {
        PrintMsgs {
            messages,
            init: false,
        }
    }
}

impl GameModifier<MiscTetGens, MiscPceRots, TileType> for PrintMsgs {
    fn id(&self) -> String {
        Self::MOD_ID.to_owned()
    }

    fn cfg(&self) -> String {
        to_savefile_string(&self.messages).unwrap()
    }

    fn try_clone(
        &self,
    ) -> Result<Box<dyn GameModifier<MiscTetGens, MiscPceRots, TileType>>, String> {
        Ok(Box::new(self.clone()))
    }

    fn values(&self) -> &[(String, String)] {
        &[]
    }

    fn on_spawn_pre(
        &mut self,
        game: GameAccess,
        feed: &mut NotificationFeed,
        time: &mut InGameTime,
    ) {
        if self.init {
            return;
        }
        self.init = true;

        if !game.config.send_notifications {
            return;
        }
        for message in self.messages.iter() {
            feed.push((Notification::Custom(message.to_owned()), *time));
        }
    }
}
