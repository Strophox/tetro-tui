use falling_tetromino_engine::Button;

use crate::settings::SlotMachine;

/**
Currently, we want to deal with TUI styles with the form like this:

                  ┌─hold─╓╶╶╶╶╶╶╶╶╶╶╴╴╴╴╴╴╴╴╴╴╖────next────┐
                  │ ▄▄█  ║                    ║      ██    │
                  └──────║                    ║  ██████    │
   Puzzle                ║                    ║╴╴╴╴╴╴╴╴╴╴╴╴┤
 ───────────             ║                    ║    ██      │
  Time: 0min 56.24s      ║                    ║  ██████    │
  Lines: 0               ║                    ║╴╴╴╴╴╴╴╴╴╴╴╴┤
  Points: 0              ║                    ║    ██      │
  Gravity: 1.0 Hz        ║                    ║  ██████    │
  Lock delay: 480ms      ║                    ║╴╴╴╴╴╴╴╴┬───┘
                         ║                    ║  ▄█▀   │
  Replay speed: 1.00x    ║                    ║╴╴╴╴╴╴╴╴┤
  Replay: 1min 2.35s     ║                    ║   ██   │
                         ║                    ║╴╴╴╴╴╴╴╴┤
                         ║                    ║  ▄█▄   │
                         ║                    ║╴╴╴╴╴╴╴╴┤
 ───basic keybinds───    ║                    ║  █▄▄   │
  [Space] pause          ║                    ║╴╴╴╴╴╴╴╴┤
   [↓][↑] speed -/+      ║                    ║  ▄▄▄▄  ⠶ ⡇ ⠳ ⠞ ⠗ ⠧ ⠼
   [←][→] timeskip -/+   ║                    ║────────┘
    [Esc] stop           ║                    ║  40 Lines left
                         ╙▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀╜  (←↓→↺↔↻⇐⇓⇒⤓⇋)

                                Stage 24
                           +9, Mono J-spin x2

In practice, we decompose it as such:
                  JIholdIABBBBBBBBBBBBBBBBBBBBCMMMMnextMMMMN
                  K ▄▄█  H                    D      ██    O
                  LIIIIIIH                    D  ██████    O
   Puzzle               TH                    DSSSSSSSSSSSSP
 ZZZZZZZZZZZ            TH                    D    ██      O
  Time: 0min 56.24s     TH                    D  ██████    O
  Lines: 0              TH                    DSSSSSSSSSSSSP
  Points: 0             TH                    D    ██      O
  Gravity: 1.0 Hz       TH                    D  ██████    O
  Lock delay: 480ms     TH                    DSSSSSSSSRMMMQ
                        TH                    D  ▄█▀   O
  Replay speed: 1.00x   TH                    DSSSSSSSSP
  Replay: 1min 2.35s    TH                    D   ██   O
                        TH                    DSSSSSSSSP
                        TH                    D  ▄█▄   O
                        TH                    DSSSSSSSSP
 ZZZbasic keybindsZZZ   TH                    D  █▄▄   O
  [Space] pause         TH                    DSSSSSSSSP
   [↓][↑] speed -/+     TH                    D  ▄▄▄▄  ⠶ ⡇ ⠳ ⠞ ⠗ ⠧ ⠼
   [←][→] timeskip -/+  TH                    DMMMMMMMMQ
    [Esc] stop          TH                    DW 40 Lines left
                        TGFFFFFFFFFFFFFFFFFFFFEW (BUTTONS HERE)
                          UVUVUVUVUVUVUVUVUVUV
                                Stage 24
                           +9, Mono J-spin x2
 */
#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
#[serde(into = "TuiStyleCompact<String>", try_from = "TuiStyleCompact<String>")]
pub struct TuiStyle {
    /// "Z"
    pub menuglyphs: [char; 1],
    /// "ABCDEFGH"
    pub frameglyphs: [char; 8],
    /// Some("TUVW")
    pub frame2glyphs: Option<[char; 4]>,
    /// "IJKL"
    pub holdglyphs: [char; 4],
    /// "MNOPQRS"
    pub nextglyphs: [char; 7],
    /// "BUTTONS HERE"
    pub buttonsglyphs: [char; Button::VARIANTS.len()],
}

pub fn default_tui_style_slots() -> SlotMachine<TuiStyle> {
    let slots = vec![
        ("Unicode".to_owned(), TuiStyle::unicode()),
        ("ASCII".to_owned(), TuiStyle::ascii()),
        ("Elektronika 60".to_owned(), TuiStyle::elektronika_60()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "TUI style".to_owned())
}

impl TuiStyle {
    pub fn unicode() -> TuiStyle {
        TuiStyleCompact {
            menu: "─",
            frame: "╓╴╖║╜▀╙║",
            frame2: None,
            hold: "─┌|└",
            next: "─┐│┤┘┬╴",
            buttons: "←→↺↻↔↓⤓⇓⇐⇒⇋",
        }
        .try_into()
        .unwrap()
    }

    pub fn ascii() -> Self {
        TuiStyleCompact {
            menu: "-",
            frame: "+-+|#=#|",
            frame2: None,
            hold: "-+|+",
            next: "-+|+++-",
            buttons: "<>LR@v!w{}H",
        }
        .try_into()
        .unwrap()
    }
    pub fn elektronika_60() -> Self {
        TuiStyleCompact {
            menu: "=",
            frame: "   !!=!!",
            frame2: Some(r"<\/>"),
            hold: "    ",
            next: "       ",
            buttons: "<>LR@v!w{}H",
        }
        .try_into()
        .unwrap()
    }
}

// -- Compaction helper code. --

impl<S: AsRef<str>> TryFrom<TuiStyleCompact<S>> for TuiStyle {
    type Error = String;

    fn try_from(value: TuiStyleCompact<S>) -> Result<Self, Self::Error> {
        fn fmt_err(vec: Vec<char>) -> String {
            format!("Could not convert {vec:?}")
        }

        let menuglyphs = value
            .menu
            .as_ref()
            .chars()
            .collect::<Vec<char>>()
            .try_into()
            .map_err(fmt_err)?;
        let frameglyphs = value
            .frame
            .as_ref()
            .chars()
            .collect::<Vec<char>>()
            .try_into()
            .map_err(fmt_err)?;
        let frame2glyphs = if let Some(frame2) = value.frame2 {
            Some(
                frame2
                    .as_ref()
                    .chars()
                    .collect::<Vec<char>>()
                    .try_into()
                    .map_err(fmt_err)?,
            )
        } else {
            None
        };
        let holdglyphs = value
            .hold
            .as_ref()
            .chars()
            .collect::<Vec<char>>()
            .try_into()
            .map_err(fmt_err)?;
        let nextglyphs = value
            .next
            .as_ref()
            .chars()
            .collect::<Vec<char>>()
            .try_into()
            .map_err(fmt_err)?;
        let buttonsglyphs = value
            .buttons
            .as_ref()
            .chars()
            .collect::<Vec<char>>()
            .try_into()
            .map_err(fmt_err)?;
        Ok(TuiStyle {
            menuglyphs,
            frameglyphs,
            frame2glyphs,
            holdglyphs,
            nextglyphs,
            buttonsglyphs,
        })
    }
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct TuiStyleCompact<T> {
    pub menu: T,
    pub frame: T,
    pub frame2: Option<T>,
    pub hold: T,
    pub next: T,
    pub buttons: T,
}

impl From<TuiStyle> for TuiStyleCompact<String> {
    fn from(value: TuiStyle) -> Self {
        TuiStyleCompact {
            menu: value.menuglyphs.iter().collect(),
            frame: value.frameglyphs.iter().collect(),
            frame2: value.frame2glyphs.map(|frame2| frame2.iter().collect()),
            hold: value.holdglyphs.iter().collect(),
            next: value.nextglyphs.iter().collect(),
            buttons: value.buttonsglyphs.iter().collect(),
        }
    }
}
