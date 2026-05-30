use crate::core_game_engine::{
    GameAccess, GameModifier, MiscPceRots, MiscTetGens, Notification, NotificationFeed, RecencyGen,
    Tetromino, TetrominoGenerator, TileType,
};

// This modifier does not have fields for configuration/reproducibility and does not keep state.
#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct PrintRecencyStats;

impl PrintRecencyStats {
    pub const MOD_ID: &str = "PrintRecencyStats";

    pub fn modifier() -> Box<dyn GameModifier<MiscTetGens, MiscPceRots, TileType>> {
        Box::new(PrintRecencyStats)
    }
}

impl GameModifier<MiscTetGens, MiscPceRots, TileType> for PrintRecencyStats {
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
        &[]
    }

    fn on_spawn_post(&mut self, game: GameAccess, feed: &mut NotificationFeed) {
        if !game.config.send_notifications {
            return;
        }
        // Only works for `Recency` generator.
        let MiscTetGens::Recency(RecencyGen {
            tets_last_emitted,
            factor,
            is_base_not_exp,
        }) = game.state.tetromino_generator
        else {
            return;
        };

        let get_weight = |n| {
            if is_base_not_exp {
                // Ensure weight is positive.
                factor.get().powf(f64::from(n)).max(f64::MIN_POSITIVE)
            } else {
                f64::from(n).powf(factor.get())
            }
        };

        let mut tetrominos_data = Tetromino::VARIANTS.map(|t| {
            (
                t,
                tets_last_emitted[t as usize],
                get_weight(tets_last_emitted[t as usize]),
            )
        });

        tetrominos_data.sort_by(|t_weight_0, t_weight_1| t_weight_0.2.total_cmp(&t_weight_1.2));

        let text_tetrominos_last_emitted = tetrominos_data
            .iter()
            .map(|(t, t_last_emitted, _)| {
                format!(
                    "{}{}",
                    ['O', 'I', 'S', 'Z', 'T', 'L', 'J'][*t as usize],
                    t_last_emitted,
                )
            })
            .collect::<Vec<String>>()
            .join(" ");

        let text_tetrominos_weight = tetrominos_data
            .iter()
            .map(|(t, _, t_weight)| {
                format!(
                    "{}{}{}",
                    ['O', 'I', 'S', 'Z', 'T', 'L', 'J'][*t as usize],
                    "█".repeat(*t_weight as usize / 8),
                    [" ", "▏", "▎", "▍", "▌", "▋", "▊", "▉"][*t_weight as usize % 8]
                )
            })
            .collect::<Vec<String>>()
            .join("");

        feed.push((Notification::Custom("".to_owned()), game.state.time));
        feed.push((
            Notification::Custom(text_tetrominos_last_emitted),
            game.state.time,
        ));
        feed.push((
            Notification::Custom(text_tetrominos_weight),
            game.state.time,
        ));
    }
}
