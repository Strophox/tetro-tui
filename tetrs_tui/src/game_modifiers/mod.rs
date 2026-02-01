use std::num::NonZero;

use tetrs_engine::{Game, GameBuilder, Modifier};

pub mod ascent;
pub mod cheese;
pub mod combo_board;
pub mod misc_modifiers;
pub mod puzzle;

pub fn reconstruct_modded<'a>(
    builder: &'a GameBuilder,
    mod_descriptors: impl IntoIterator<Item = &'a str>,
) -> Result<Game, String> {
    let mut compounding_mod: Vec<Modifier> = Vec::new();
    #[allow(clippy::type_complexity)]
    let mut building_mod: Option<(&str, Box<dyn Fn(&'a GameBuilder) -> Game>)> = None;

    let mut store_building_mod = |mod_id, build| {
        if let Some((other_id, _)) = building_mod {
            return Err(format!("incompatible mods: {other_id:?} + {mod_id:?}"));
        }
        building_mod.replace((mod_id, build));
        Ok(())
    };

    // NOTE: We can actually only deserialize to owned types, so if a mod accepts `&str` in args, we need to instead parse `String`.
    fn get_mod_args<'de, T: serde::Deserialize<'de>>(
        lines: &mut std::str::Lines<'de>,
        mod_id: &str,
    ) -> Result<T, String> {
        let Some(mod_args_str) = lines.next() else {
            return Err(format!("mod args missing for {mod_id:?}"));
        };
        let Ok(args) = serde_json::from_str(mod_args_str) else {
            return Err(format!("mod args parse error for {mod_id}: {mod_args_str}"));
        };
        Ok(args)
    }

    for mod_descriptor in mod_descriptors {
        let mut lines = mod_descriptor.lines();
        let mod_id = lines.next().unwrap_or("");

        if mod_id == puzzle::MOD_ID {
            let build = Box::new(puzzle::build);
            store_building_mod(mod_id, build)?;

        } else if mod_id == ascent::MOD_ID {
            let build = Box::new(ascent::build);
            store_building_mod(mod_id, build)?;

        } else if mod_id == cheese::MOD_ID {
            let (linelimit, gapsize, gravity) = get_mod_args::<(Option<NonZero<usize>>, usize, u32)>(&mut lines, mod_id)?;
            let build =
                Box::new(move |builder| cheese::build(builder, linelimit, gapsize, gravity));
            store_building_mod(mod_id, build)?;

        } else if mod_id == combo_board::MOD_ID {
            let linelimit = get_mod_args::<u16>(&mut lines, mod_id)?;
            let modifier = combo_board::modifier(linelimit);
            compounding_mod.push(modifier);

        } else if mod_id == misc_modifiers::print_recency_tet_gen_stats::MOD_ID {
            let modifier = misc_modifiers::print_recency_tet_gen_stats::modifier();
            compounding_mod.push(modifier);

        } else if mod_id == misc_modifiers::custom_start_board::MOD_ID {
            let encoded_board = get_mod_args::<String>(&mut lines, mod_id)?;
            let modifier = misc_modifiers::custom_start_board::modifier(&encoded_board);
            compounding_mod.push(modifier);

        } else {
            return Err(format!("unrecognized mod {mod_id:?}"));
        }

    }

    Ok(if let Some((_, build)) = building_mod {
        let mut game = build(builder);
        game.modifiers.extend(compounding_mod);
        game

    } else {
        builder.build_modded(compounding_mod)
    })
}
