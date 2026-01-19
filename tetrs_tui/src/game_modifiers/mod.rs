use tetrs_engine::{Game, GameBuilder, Modifier};

pub mod cheese;
pub mod combo_board;
pub mod descent;
pub mod misc;
pub mod puzzle;

pub fn reconstruct_modified<'a>(
    builder: &'a GameBuilder,
    mod_descriptors: impl IntoIterator<Item = &'a str>,
) -> Result<Game, String> {
    let mut compounding_modifiers: Vec<Modifier> = Vec::new();
    #[allow(clippy::type_complexity)]
    let mut building_modifier: Option<(&str, Box<dyn Fn(&'a GameBuilder) -> Game>)> = None;

    let mut set_building_modifier = |mod_id, build| {
        if let Some((other_id, _)) = building_modifier {
            return Err(format!("incompatible mods: {other_id:?} + {mod_id:?}"));
        }
        building_modifier.replace((mod_id, build));
        Ok(())
    };

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
            set_building_modifier(mod_id, build)?;
        } else if mod_id == descent::MOD_ID {
            let build = Box::new(descent::build);
            set_building_modifier(mod_id, build)?;
        } else if mod_id == cheese::MOD_ID {
            let (linelimit, gapsize, gravity) = get_mod_args(&mut lines, mod_id)?;
            let build =
                Box::new(move |builder| cheese::build(builder, linelimit, gapsize, gravity));
            set_building_modifier(mod_id, build)?;
        } else if mod_id == combo_board::MOD_ID {
            let linelimit = get_mod_args(&mut lines, mod_id)?;
            let modifier = combo_board::modifier(linelimit);
            compounding_modifiers.push(modifier);
        } else if mod_id == misc::custom_start_board::MOD_ID {
            let encoded_board = get_mod_args(&mut lines, mod_id)?;
            let modifier = misc::custom_start_board::modifier(encoded_board);
            compounding_modifiers.push(modifier);
        } else {
            return Err(format!("unrecognized mod {mod_id:?}"));
        }
    }
    Ok(if let Some((_, build)) = building_modifier {
        let mut game = build(builder);
        game.modifiers_mut().extend(compounding_modifiers);
        game
    } else {
        builder.build_modified(compounding_modifiers)
    })
}
