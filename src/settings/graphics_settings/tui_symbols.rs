use falling_tetromino_engine::Button;

use crate::settings::SlotMachine;

/**
Currently, we want to deal with TUI styles with the form like this:
```text

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
```

In practice, we decompose it as such:
```text

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
```
 */
#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
#[serde(
    into = "CompactTuiSymbols<String>",
    try_from = "CompactTuiSymbols<String>"
)]
pub struct TuiSymbols {
    /// Whether to use the ASCII title screen variant.
    pub blocky_title_logo: bool,
    /// Left- and right indicators used to show what is selected in a menu.
    pub menu_pointers: [String; 2],
    /// "Z"
    pub headingline: [char; 1],
    /// "ABCDEFGH"
    pub boardframe: [char; 8],
    /// Some("TUVW")
    pub boardframe2: Option<[char; 4]>,
    /// "IJKL"
    pub holdframe: [char; 4],
    /// "MNOPQRS"
    pub nextframe: [char; 7],
    /// Use for button display.
    pub buttons: [char; Button::VARIANTS.len()],
    /// Use for lock-down count down display.
    pub timer: Vec<String>,
    /// Use for replay progress bar.
    pub progressbar: (Vec<char>, char),
}

pub fn tui_symbols_presets() -> SlotMachine<TuiSymbols> {
    let slots = vec![
        ("ASCII".to_owned(), TuiSymbols::ascii()),
        ("Frame UTF8".to_owned(), TuiSymbols::unicode()),
        (
            "Rounded frame UTF8".to_owned(),
            TuiSymbols::rounded_unicode(),
        ),
        ("No frame UTF8".to_owned(), TuiSymbols::borderless_unicode()),
        (
            "No hold/next-frame UTF8".to_owned(),
            TuiSymbols::borderless_hold_next_unicode(),
        ),
        ("Braille".to_owned(), TuiSymbols::braille()),
        ("Elektronika 60".to_owned(), TuiSymbols::elektronika_60()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "TUI symbols".to_owned())
}

impl TuiSymbols {
    pub fn ascii() -> Self {
        CompactTuiSymbols {
            blocky_title_logo: false,
            menu_pointers: [">>", "<<"].map(|s| s.to_owned()),
            headingline: "-",
            boardframe: "+-+|#=#|",
            boardframe2: None,
            holdframe: "-+|+",
            nextframe: "-+|+++-",
            buttons: "<>LR@v!w{}H",
            timer: ["1", "2", "3", "4", "5", "6", "7", "8", "9"]
                .map(|s| s.to_owned())
                .into(),
            progressbar: (" .:!", '|'),
        }
        .try_into()
        .unwrap()
    }

    pub fn unicode() -> TuiSymbols {
        CompactTuiSymbols {
            blocky_title_logo: true,
            menu_pointers: ["▶", "◀"].map(|s| s.to_owned()), // ▶◀ ▷◁ ◆◆ ►◄ ▻◅ ?
            headingline: "─",
            boardframe: "╓╴╖║╜▀╙║",
            boardframe2: None,
            holdframe: "─┌│└",
            nextframe: "─┐│┤┘┬╴",
            buttons: "←→↺↻↔↓↨⇓⇐⇒⇋", // ⤓🗘 ?
            timer: ["⠈", "⠘", "⠸", "⢸", "⣸", "⣼", "⣾", "⣿"]
                .map(|s| s.to_owned())
                .into(),
            progressbar: (" ▏▎▍▌▋▊▉", '█'),
        }
        .try_into()
        .unwrap()
    }

    pub fn rounded_unicode() -> TuiSymbols {
        CompactTuiSymbols {
            blocky_title_logo: true,
            menu_pointers: ["▶", "◀"].map(|s| s.to_owned()),
            headingline: "─",
            boardframe: "╓╴╖║╜▀╙║",
            boardframe2: None,
            holdframe: "─╭│╰",
            nextframe: "─╮│┤╯┬╴",
            buttons: "←→↺↻↔↓↨⇓⇐⇒⇋",
            timer: ["⠈", "⠘", "⠸", "⢸", "⣸", "⣼", "⣾", "⣿"]
                .map(|s| s.to_owned())
                .into(),
            progressbar: (" ▏▎▍▌▋▊▉", '█'),
        }
        .try_into()
        .unwrap()
    }

    pub fn borderless_unicode() -> Self {
        CompactTuiSymbols {
            blocky_title_logo: true,
            menu_pointers: ["▶", "◀"].map(|s| s.to_owned()),
            headingline: " ",
            boardframe: "        ",
            boardframe2: None,
            holdframe: "    ",
            nextframe: "       ",
            buttons: "←→↺↻↔↓↨⇓⇐⇒⇋",
            timer: ["⠈", "⠘", "⠸", "⢸", "⣸", "⣼", "⣾", "⣿"]
                .map(|s| s.to_owned())
                .into(),
            progressbar: (" ▏▎▍▌▋▊▉", '█'),
        }
        .try_into()
        .unwrap()
    }

    pub fn borderless_hold_next_unicode() -> Self {
        CompactTuiSymbols {
            blocky_title_logo: true,
            menu_pointers: ["▶", "◀"].map(|s| s.to_owned()),
            headingline: "─",
            boardframe: "╓╴╖║╜▀╙║",
            boardframe2: None,
            holdframe: "    ",
            nextframe: "       ",
            buttons: "←→↺↻↔↓↨⇓⇐⇒⇋",
            timer: ["⠈", "⠘", "⠸", "⢸", "⣸", "⣼", "⣾", "⣿"]
                .map(|s| s.to_owned())
                .into(),
            progressbar: (" ▏▎▍▌▋▊▉", '█'),
        }
        .try_into()
        .unwrap()
    }

    pub fn braille() -> TuiSymbols {
        CompactTuiSymbols {
            blocky_title_logo: true,
            menu_pointers: ["⠕⠕", "⠪⠪"].map(|s| s.to_owned()), // ⠒⠗⠺ ?
            headingline: "⠒",
            boardframe: "⡖⠂⢲⢸⠚⠒⠓⡇",
            boardframe2: None,
            holdframe: "⠒⡖⡇⠓",
            nextframe: "⠒⢲⢸⢺⠚⢲⠂",
            buttons: "←→↺↻↔↓↨⇓⇐⇒⇋",
            timer: ["⠈", "⠘", "⠸", "⢸", "⣸", "⣼", "⣾", "⣿"]
                .map(|s| s.to_owned())
                .into(),
            progressbar: (" ⡀⡄⡆⡇⡏⡟⡿", '⣿'),
        }
        .try_into()
        .unwrap()
    }

    pub fn elektronika_60() -> Self {
        CompactTuiSymbols {
            blocky_title_logo: false,
            menu_pointers: [">>", "<<"].map(|s| s.to_owned()),
            headingline: "=",
            boardframe: "   !!=!!",
            boardframe2: Some(r"<\/>"),
            holdframe: "    ",
            nextframe: "       ",
            buttons: "<>LR@v!w{}H",
            timer: ["1", "2", "3", "4", "5", "6", "7", "8", "9"]
                .map(|s| s.to_owned())
                .into(),
            progressbar: (" .:!", '|'),
        }
        .try_into()
        .unwrap()
    }
}

// -- Compaction helper code. --

impl<S: AsRef<str>> TryFrom<CompactTuiSymbols<S>> for TuiSymbols {
    type Error = String;

    fn try_from(value: CompactTuiSymbols<S>) -> Result<Self, Self::Error> {
        fn fmt_err(vec: Vec<char>) -> String {
            format!("Could not convert {vec:?}")
        }

        let menuglyphs = value
            .headingline
            .as_ref()
            .chars()
            .collect::<Vec<char>>()
            .try_into()
            .map_err(fmt_err)?;
        let frameglyphs = value
            .boardframe
            .as_ref()
            .chars()
            .collect::<Vec<char>>()
            .try_into()
            .map_err(fmt_err)?;
        let frame2glyphs = if let Some(frame2) = value.boardframe2 {
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
            .holdframe
            .as_ref()
            .chars()
            .collect::<Vec<char>>()
            .try_into()
            .map_err(fmt_err)?;
        let nextglyphs = value
            .nextframe
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
        let progressbarglyphs = (
            value.progressbar.0.as_ref().chars().collect::<Vec<char>>(),
            value.progressbar.1,
        );
        Ok(TuiSymbols {
            blocky_title_logo: value.blocky_title_logo,
            menu_pointers: value.menu_pointers,
            headingline: menuglyphs,
            boardframe: frameglyphs,
            boardframe2: frame2glyphs,
            holdframe: holdglyphs,
            nextframe: nextglyphs,
            buttons: buttonsglyphs,
            timer: value.timer,
            progressbar: progressbarglyphs,
        })
    }
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct CompactTuiSymbols<T> {
    pub blocky_title_logo: bool,
    pub menu_pointers: [String; 2],
    pub headingline: T,
    pub boardframe: T,
    pub boardframe2: Option<T>,
    pub holdframe: T,
    pub nextframe: T,
    pub buttons: T,
    pub timer: Vec<String>,
    pub progressbar: (T, char),
}

impl From<TuiSymbols> for CompactTuiSymbols<String> {
    fn from(value: TuiSymbols) -> Self {
        CompactTuiSymbols {
            blocky_title_logo: value.blocky_title_logo,
            menu_pointers: value.menu_pointers,
            headingline: value.headingline.iter().collect(),
            boardframe: value.boardframe.iter().collect(),
            boardframe2: value.boardframe2.map(|frame2| frame2.iter().collect()),
            holdframe: value.holdframe.iter().collect(),
            nextframe: value.nextframe.iter().collect(),
            buttons: value.buttons.iter().collect(),
            timer: value.timer,
            progressbar: (value.progressbar.0.iter().collect(), value.progressbar.1),
        }
    }
}
