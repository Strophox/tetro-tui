use falling_tetromino_engine::{BOARD_WIDTH, PLAYABLE_BOARD_HEIGHT};

use crate::core_game_engine::{
    GameAccess, GameEndCause, GameModifier, MiscPceRots, MiscTetGens, Notification,
    NotificationFeed, Phase, TileType,
};

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct ReviveTopOut {
    // This modifier does not have fields for configuration/reproducibility.

    // Stateful fields.
    display_values: [(String, String); 1],
    //last_revive: Option<InGameTime>,
}

impl ReviveTopOut {
    pub const MOD_ID: &str = "ReviveTopOut";

    pub fn modifier() -> Box<dyn GameModifier<MiscTetGens, MiscPceRots, TileType>> {
        Box::new(ReviveTopOut {
            display_values: [("No Top Out".to_owned(), "on".to_owned())],
        })
    }
}

impl GameModifier<MiscTetGens, MiscPceRots, TileType> for ReviveTopOut {
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
        &self.display_values
    }

    fn on_game_end(&mut self, game: GameAccess, feed: &mut NotificationFeed) {
        let Phase::GameEnd { cause, is_win: _ } = game.phase else {
            return;
        };
        match cause {
            // Revive lock-/block-/buffer-outs by clearing the board.
            GameEndCause::LockOut { .. }
            | GameEndCause::BlockOut { .. }
            | GameEndCause::BufferOut => {}
            // Do not revive explicit forfeit, unknow custom cause, or limit.
            GameEndCause::Forfeit { .. } | GameEndCause::Custom(_) | GameEndCause::Limit(_) => {
                return;
            }
        }

        // Do a simple check whether the player is even able to manipulate the pieces.
        // This should also avoid infinite loops.
        if game.state.fall_delay.is_zero()
            && game.state.lock_delay.is_zero()
            && game.config.send_notifications
        {
            feed.push((
                Notification::Custom("...and it ends for good.".to_owned()),
                game.state.time,
            ));
            return;
        }

        game.state.board.clear();

        *game.phase = Phase::Spawning {
            spawn_time: game.state.time,
        };

        if game.config.send_notifications {
            feed.push((
                Notification::Custom("...but then it continues!".to_owned()),
                game.state.time,
            ));
            feed.push((
                Notification::LinesClearing {
                    lines: vec![(PLAYABLE_BOARD_HEIGHT, [TileType::Generic; BOARD_WIDTH])],
                    line_clear_duration: game.config.line_clear_duration,
                },
                game.state.time,
            ));
        }
    }
}
