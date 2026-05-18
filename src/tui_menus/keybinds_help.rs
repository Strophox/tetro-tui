use std::io::{self, Write};

use crate::{Application, tui_menus::MenuUpdate};

impl<W: Write> Application<W> {
    pub fn run_menu_keybinds_help(
        &mut self,
        client_menu_name: &str,
        keybinds_legend: &[(String, Vec<(String, String)>)],
    ) -> io::Result<MenuUpdate> {
        let title = format!("$ Keybinds Overview for '{client_menu_name}' $",);
        let w_lhs_max = keybinds_legend
            .iter()
            .map(|x| x.0.chars().count())
            .max()
            .unwrap_or(0);
        // let w_rhs_max = keybinds_legend.iter().map(|x| x.1.chars().count()).max().unwrap_or(0);

        let body = keybinds_legend
            .iter()
            .map(|(groupname, entries)| {
                format!(
                    "{}\n{}",
                    groupname.to_uppercase(),
                    entries
                        .iter()
                        .map(|(lhs, rhs)| format!("{lhs: >w_lhs_max$} = {rhs}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        self.run_text_menu(&title, &body, false, "Keybinds Overview")
    }
}
