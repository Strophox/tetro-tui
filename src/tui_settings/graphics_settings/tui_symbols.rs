use falling_tetromino_engine::Button;

use crate::tui_settings::SlotMachine;

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
    pub is_title_unicode: bool,
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
    /// Use for lock-down count down display.
    pub countdownglyphs: Vec<String>,
    /// Use for replay progress bar.
    pub progressbarglyphs: (Vec<char>, char),
}

pub fn tui_symbols_presets() -> SlotMachine<TuiSymbols> {
    let slots = vec![
        ("ASCII".to_owned(), TuiSymbols::ascii()),
        ("Unicode".to_owned(), TuiSymbols::unicode()),
        (
            "Borderless Unicode".to_owned(),
            TuiSymbols::borderless_unicode(),
        ),
        ("Elektronika 60".to_owned(), TuiSymbols::elektronika_60()),
    ];

    SlotMachine::with_unmodifiable_slots(slots, "TUI style".to_owned())
}

impl TuiSymbols {
    pub fn ascii() -> Self {
        CompactTuiSymbols {
            is_title_unicode: false,
            menuglyphs: "-",
            frame: "+-+|#=#|",
            frame2: None,
            hold: "-+|+",
            next: "-+|+++-",
            buttons: "<>LR@v!w{}H",
            countdown: ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"]
                .map(|s| s.to_owned())
                .into(),
            progressbar: (" .:!", '|'),
        }
        .try_into()
        .unwrap()
    }

    pub fn unicode() -> TuiSymbols {
        CompactTuiSymbols {
            is_title_unicode: true,
            menuglyphs: "─",
            frame: "╓╴╖║╜▀╙║",
            frame2: None,
            hold: "─┌│└",
            next: "─┐│┤┘┬╴",
            buttons: "←→↺↻↔↓⤓⇓⇐⇒⇋",
            countdown: ["⡀", "⡄", "⡆", "⡇", "⡏", "⡟", "⡿", "⣿"]
                .map(|s| s.to_owned())
                .into(),
            progressbar: (" ▏▎▍▌▋▊▉", '█'),
        }
        .try_into()
        .unwrap()
    }

    pub fn elektronika_60() -> Self {
        CompactTuiSymbols {
            is_title_unicode: false,
            menuglyphs: "=",
            frame: "   !!=!!",
            frame2: Some(r"<\/>"),
            hold: "    ",
            next: "       ",
            buttons: "<>LR@v!w{}H",
            countdown: ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"]
                .map(|s| s.to_owned())
                .into(),
            progressbar: (" .:!", '|'),
        }
        .try_into()
        .unwrap()
    }

    pub fn borderless_unicode() -> Self {
        CompactTuiSymbols {
            is_title_unicode: true,
            menuglyphs: " ",
            frame: "        ",
            frame2: None,
            hold: "    ",
            next: "       ",
            buttons: "←→↺↻↔↓⤓⇓⇐⇒⇋",
            countdown: ["⡀", "⡄", "⡆", "⡇", "⡏", "⡟", "⡿", "⣿"]
                .map(|s| s.to_owned())
                .into(),
            progressbar: (" ▏▎▍▌▋▊▉", '█'),
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
            .menuglyphs
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
        let progressbarglyphs = (
            value.progressbar.0.as_ref().chars().collect::<Vec<char>>(),
            value.progressbar.1,
        );
        Ok(TuiSymbols {
            is_title_unicode: value.is_title_unicode,
            menuglyphs,
            frameglyphs,
            frame2glyphs,
            holdglyphs,
            nextglyphs,
            buttonsglyphs,
            countdownglyphs: value.countdown,
            progressbarglyphs,
        })
    }
}

#[derive(
    PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Debug, serde::Serialize, serde::Deserialize,
)]
pub struct CompactTuiSymbols<T> {
    pub is_title_unicode: bool,
    pub menuglyphs: T,
    pub frame: T,
    pub frame2: Option<T>,
    pub hold: T,
    pub next: T,
    pub buttons: T,
    pub countdown: Vec<String>,
    pub progressbar: (T, char),
}

impl From<TuiSymbols> for CompactTuiSymbols<String> {
    fn from(value: TuiSymbols) -> Self {
        CompactTuiSymbols {
            is_title_unicode: value.is_title_unicode,
            menuglyphs: value.menuglyphs.iter().collect(),
            frame: value.frameglyphs.iter().collect(),
            frame2: value.frame2glyphs.map(|frame2| frame2.iter().collect()),
            hold: value.holdglyphs.iter().collect(),
            next: value.nextglyphs.iter().collect(),
            buttons: value.buttonsglyphs.iter().collect(),
            countdown: value.countdownglyphs,
            progressbar: (
                value.progressbarglyphs.0.iter().collect(),
                value.progressbarglyphs.1,
            ),
        }
    }
}
