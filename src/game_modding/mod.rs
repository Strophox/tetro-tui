mod ascent;
mod cheese;
mod combo;
mod display_finesse;
mod placement_practice;
mod print_msgs;
#[allow(unused)]
mod print_recency_stats;
mod puzzle;
mod revive_top_out;
mod start_board;
mod survival;

use crate::core_game_engine::{
    Game, GameBuilder, GameModifier, MiscPceRots, MiscTetGens, TileType,
};

use crate::savefile_logic::from_savefile_str;

pub use ascent::Ascent;
pub use cheese::{Cheese, CheeseConfig};
pub use combo::{Combo, ComboConfig};
pub use display_finesse::DisplayFinesse;
pub use placement_practice::PlacementPractice;
pub use print_msgs::PrintMsgs;
pub use print_recency_stats::PrintRecencyStats;
pub use puzzle::Puzzle;
pub use revive_top_out::ReviveTopOut;
pub use start_board::StartBoard;
pub use survival::{Survival, SurvivalConfig};

pub fn reconstruct_modded(
    builder: &GameBuilder,
    mod_ids_cfgs: &Vec<(String, String)>,
) -> Result<(Game, Vec<String>), String> {
    let mut modifiers: Vec<Box<dyn GameModifier<MiscTetGens, MiscPceRots, TileType>>> = Vec::new();
    let mut unrecognized_mod_ids = Vec::new();

    // NOTE: We can actually only deserialize to owned types, so if a mod accepts `&str` in cfgs, we need to instead parse `String`.
    fn parse_mod_cfg<'de, T: serde::Deserialize<'de>>(
        mod_cfg_str: &'de str,
        mod_id: &str,
    ) -> Result<T, String> {
        match from_savefile_str(mod_cfg_str) {
            Ok(config) => Ok(config),
            Err(e) => Err(format!(
                "cannot parse for mod_id '{mod_id}' the cfg '{mod_cfg_str}': {e}"
            )),
        }
    }

    for (mod_id, mod_cfg_str) in mod_ids_cfgs {
        if mod_id == Puzzle::MOD_ID {
            modifiers.push(Box::new(Puzzle::new()));
        } else if mod_id == Ascent::MOD_ID {
            modifiers.push(Box::new(Ascent::new()));
        } else if mod_id == Cheese::MOD_ID {
            let config: CheeseConfig = parse_mod_cfg(mod_cfg_str, mod_id)?;
            modifiers.push(Box::new(Cheese::with_cfg(config)));
        } else if mod_id == Survival::MOD_ID {
            let config: SurvivalConfig = parse_mod_cfg(mod_cfg_str, mod_id)?;
            modifiers.push(Box::new(Survival::with_cfg(config)));
        } else if mod_id == Combo::MOD_ID {
            let config: ComboConfig = parse_mod_cfg(mod_cfg_str, mod_id)?;
            modifiers.push(Box::new(Combo::with_cfg(config)));
        } else if mod_id == PlacementPractice::MOD_ID {
            modifiers.push(Box::new(PlacementPractice::new()));
        } else if mod_id == StartBoard::MOD_ID {
            let encoded_board: String = parse_mod_cfg(mod_cfg_str, mod_id)?;
            modifiers.push(Box::new(StartBoard::with_board(encoded_board)));
        } else if mod_id == PrintRecencyStats::MOD_ID {
            modifiers.push(Box::new(PrintRecencyStats::new()));
        } else if mod_id == PrintMsgs::MOD_ID {
            let messages: Vec<String> = parse_mod_cfg(mod_cfg_str, mod_id)?;
            modifiers.push(Box::new(PrintMsgs::with_msgs(messages)));
        } else if mod_id == DisplayFinesse::MOD_ID {
            modifiers.push(Box::new(DisplayFinesse::new()));
        } else if mod_id == ReviveTopOut::MOD_ID {
            modifiers.push(Box::new(ReviveTopOut::new()));
        } else {
            unrecognized_mod_ids.push(mod_id.to_owned());
        }
    }

    let game = builder.build_modded(modifiers);

    Ok((game, unrecognized_mod_ids))
}
