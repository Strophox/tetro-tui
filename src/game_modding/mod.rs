mod ascent;
mod cheese;
mod combo;
mod print_msgs;
mod print_recency_stats;
mod puzzle;
mod start_board;

use falling_tetromino_engine::{Game, GameBuilder, GameModifier};

use crate::savefile_logic::from_savefile_str;

pub use ascent::Ascent;
pub use cheese::{Cheese, CheeseConfig};
pub use combo::{Combo, ComboConfig};
pub use print_msgs::PrintMsgs;
pub use print_recency_stats::PrintRecencyStats;
pub use puzzle::Puzzle;
pub use start_board::StartBoard;

pub fn reconstruct_modded<'a>(
    builder: &'a GameBuilder,
    mod_ids_cfgs: &Vec<(String, String)>,
) -> Result<(Game, Vec<String>), String> {
    let mut compounding_mods: Vec<Box<dyn GameModifier>> = Vec::new();

    #[allow(clippy::type_complexity)]
    let mut building_mod: Option<(&str, Box<dyn FnOnce(&'a GameBuilder) -> Game>)> = None;

    let mut store_building_mod = |mod_id, build| {
        if let Some((other_id, _)) = building_mod {
            return Err(format!("incompatible mods: {other_id:?} + {mod_id:?}"));
        }
        building_mod.replace((mod_id, build));
        Ok(())
    };

    let mut unrecognized_mod_ids = Vec::new();

    // NOTE: We can actually only deserialize to owned types, so if a mod accepts `&str` in cfgs, we need to instead parse `String`.
    fn get_mod_config<'de, T: serde::Deserialize<'de>>(
        mod_cfg_str: &'de str,
        mod_id: &str,
    ) -> Result<T, String> {
        match from_savefile_str(mod_cfg_str) {
            Ok(config) => Ok(config),
            Err(e) => Err(format!(
                "mod cfg parse error for {mod_id}: {mod_cfg_str} ({e}"
            )),
        }
    }

    for (mod_id, mod_cfg_str) in mod_ids_cfgs {
        if mod_id == Puzzle::MOD_ID {
            let build = Box::new(Puzzle::build);
            store_building_mod(mod_id, build)?;
        } else if mod_id == Ascent::MOD_ID {
            let build = Box::new(Ascent::build);
            store_building_mod(mod_id, build)?;
        } else if mod_id == Cheese::MOD_ID {
            let config: CheeseConfig = get_mod_config(mod_cfg_str, mod_id)?;
            let build = Box::new(move |builder| Cheese::build(builder, config));
            store_building_mod(mod_id, build)?;
        } else if mod_id == Combo::MOD_ID {
            let config: ComboConfig = get_mod_config(mod_cfg_str, mod_id)?;
            let build = Box::new(move |builder| Combo::build(builder, config));
            store_building_mod(mod_id, build)?;
        } else if mod_id == StartBoard::MOD_ID {
            let encoded_board: String = get_mod_config(mod_cfg_str, mod_id)?;
            let build = Box::new(move |builder| StartBoard::build(builder, encoded_board));
            store_building_mod(mod_id, build)?;
        } else if mod_id == PrintRecencyStats::MOD_ID {
            let modifier = PrintRecencyStats::modifier();
            compounding_mods.push(modifier);
        } else if mod_id == PrintMsgs::MOD_ID {
            let messages: Vec<String> = get_mod_config(mod_cfg_str, mod_id)?;
            let modifier = PrintMsgs::modifier(messages);
            compounding_mods.push(modifier);
        } else {
            unrecognized_mod_ids.push(mod_id.to_owned());
        }
    }

    let mut game = if let Some((_, build)) = building_mod {
        build(builder)
    } else {
        builder.build()
    };

    game.modifiers.extend(compounding_mods);

    Ok((game, unrecognized_mod_ids))
}
