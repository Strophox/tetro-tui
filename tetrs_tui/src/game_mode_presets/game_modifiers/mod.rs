use std::num::NonZero;

use tetrs_engine::{Game, GameBuilder, Modifier};

pub mod ascent;
pub mod cheese;
pub mod combo_board;
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
            let (linelimit, gapsize, gravity) =
                get_mod_args::<(Option<NonZero<usize>>, usize, u32)>(&mut lines, mod_id)?;
            let build =
                Box::new(move |builder| cheese::build(builder, linelimit, gapsize, gravity));
            store_building_mod(mod_id, build)?;
        } else if mod_id == combo_board::MOD_ID {
            let linelimit = get_mod_args::<u16>(&mut lines, mod_id)?;
            let modifier = combo_board::modifier(linelimit);
            compounding_mod.push(modifier);
        } else if mod_id == print_recency_tet_gen_stats::MOD_ID {
            let modifier = print_recency_tet_gen_stats::modifier();
            compounding_mod.push(modifier);
        } else if mod_id == custom_start_board::MOD_ID {
            let encoded_board = get_mod_args::<String>(&mut lines, mod_id)?;
            let modifier = custom_start_board::modifier(&encoded_board);
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

pub mod custom_start_board {
    use tetrs_engine::Modifier;

    pub const MOD_ID: &str = "custom_start_board";

    pub fn modifier(encoded_board: &str) -> Modifier {
        let board = crate::application::NewGameSettings::decode_board(encoded_board);
        let mut init = false;
        Modifier {
            descriptor: format!(
                "{MOD_ID}\n{}",
                serde_json::to_string(&encoded_board).unwrap()
            ),
            mod_function: Box::new(move |_point, _config, _init_vals, state, _phase, _msgs| {
                if !init {
                    state.board.clone_from(&board);
                    init = true;
                }
            }),
        }
    }
}

// NOTE: Can be / was used for debugging.
#[allow(dead_code)]
pub mod print_recency_tet_gen_stats {
    use tetrs_engine::{
        tetromino_generator::TetrominoGenerator, Feedback, Modifier, Tetromino, UpdatePoint,
    };

    pub const MOD_ID: &str = "print_recency_tet_gen_stats";

    pub fn modifier() -> Modifier {
        Modifier {
            descriptor: MOD_ID.to_owned(),
            mod_function: Box::new(|point, _config, _init_vals, state, _phase, msgs| {
                if !matches!(point, UpdatePoint::PieceSpawned) {
                    return;
                }
                let TetrominoGenerator::Recency {
                    last_generated,
                    snap: _,
                } = state.piece_generator
                else {
                    return;
                };
                let mut pieces_played_strs = Tetromino::VARIANTS;
                pieces_played_strs.sort_by_key(|&tet| last_generated[tet as usize]);

                let [o, i, s, z, t, l, j] = state.pieces_locked;
                let str_piece_tallies = format!("{o}o {i}i {s}s {z}z {t}t {l}l {j}j");
                let str_piece_likelihood = pieces_played_strs
                    .map(|tet| {
                        format!(
                            "{tet:?}{}{}{}",
                            last_generated[tet as usize],
                            // "█".repeat(lg[t] as usize),
                            "█".repeat(
                                (last_generated[tet as usize] * last_generated[tet as usize])
                                    as usize
                                    / 8
                            ),
                            [" ", "▏", "▎", "▍", "▌", "▋", "▊", "▉"][(last_generated[tet as usize]
                                * last_generated[tet as usize])
                                as usize
                                % 8]
                        )
                        .to_ascii_lowercase()
                    })
                    .join("");
                msgs.push((state.time, Feedback::Text("".to_owned())));
                msgs.push((state.time, Feedback::Text(str_piece_likelihood)));
                msgs.push((state.time, Feedback::Text(str_piece_tallies)));
                // config.line_clear_delay = Duration::ZERO;
                // config.appearance_delay = Duration::ZERO;
                // state.board.remove(0);
                // state.board.push(Default::default());
                // state.board.remove(0);
                // state.board.push(Default::default());
            }),
        }
    }
}
