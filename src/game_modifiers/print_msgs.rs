use falling_tetromino_engine::{
    GameAccess, GameModifier, InGameTime, Notification, NotificationFeed,
};

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct PrintMsgs {
    messages: Vec<String>,

    init: bool,
}

impl PrintMsgs {
    pub const MOD_ID: &str = stringify!(PrintMsgs);

    pub fn modifier(messages: Vec<String>) -> Box<dyn GameModifier> {
        Box::new(Self {
            messages,
            init: false,
        })
    }
}

impl GameModifier for PrintMsgs {
    fn id(&self) -> String {
        Self::MOD_ID.to_owned()
    }

    fn args(&self) -> String {
        serde_json::to_string(&self.messages).unwrap()
    }

    fn try_clone(&self) -> Result<Box<dyn GameModifier>, String> {
        Ok(Box::new(self.clone()))
    }

    fn on_spawn_pre(
        &mut self,
        _game: GameAccess,
        feed: &mut NotificationFeed,
        time: &mut InGameTime,
    ) {
        if self.init {
            return;
        }
        self.init = true;

        for message in self.messages.iter() {
            feed.push((Notification::Custom(message.to_owned()), *time));
        }
    }
}
