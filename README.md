!["Tetro TUI logo"](https://github.com/Strophox/tetro-tui/blob/b66590cb461d34c95e988ef41b6d8b7d7783f37b/demo_assets/tetro_logo_glow.png?raw=true)


# Tetro TUI - Terminal Game

[![Crates.io](https://img.shields.io/crates/v/tetro-tui.svg)](https://crates.io/crates/tetro-tui)
[![License](https://img.shields.io/crates/l/tetro-tui)](https://github.com/Strophox/tetro-tui#license)
<!--[![Documentation](https://docs.rs/tetro-tui/badge.svg)](https://docs.rs/tetro-tui)-->

Tetro TUI is a terminal-based but modern tetromino-stacking game that is customizable and cross-platform.

!["tetro-tui demo GIF"](https://github.com/Strophox/tetro-tui/blob/15d6f8a13d146d2655f80559e0aee0430527f1c9/demo_assets/tetro-tui-v3.5.0_demo.gif?raw=true)


> <details>
> <summary>
> 6 more demo images available;
> </summary>
>
> !["tetro-tui coolretroterm-monochrome-classic-elektronika"](https://github.com/Strophox/tetro-tui/blob/b66590cb461d34c95e988ef41b6d8b7d7783f37b/demo_assets/tetro-tui-v3.0.0_coolretroterm-monochrome-classic-elektronika.png?raw=true)
>
> !["tetro-tui alacritty-terafox-combo-advc"](https://github.com/Strophox/tetro-tui/blob/b66590cb461d34c95e988ef41b6d8b7d7783f37b/demo_assets/tetro-tui-v3.0.0_alacritty-terafox-combo-advc.png?raw=true)
>
> !["tetro-tui gnome-solarized-cheese-monochromeboardgrid"](https://github.com/Strophox/tetro-tui/blob/b66590cb461d34c95e988ef41b6d8b7d7783f37b/demo_assets/tetro-tui-v3.0.0_gnome-solarized-cheese-monochromeboardgrid.png?raw=true)
>
> !["tetro-tui gnome-standard-replay-harddrop"](https://github.com/Strophox/tetro-tui/blob/b66590cb461d34c95e988ef41b6d8b7d7783f37b/demo_assets/tetro-tui-v3.0.0_gnome-standard-replay-harddrop.png?raw=true)
>
> !["tetro-tui kitty-okpalette-puzzle-ascii"](https://github.com/Strophox/tetro-tui/blob/b66590cb461d34c95e988ef41b6d8b7d7783f37b/demo_assets/tetro-tui-v3.0.0_kitty-okpalette-puzzle-ascii.png?raw=true)
>
> !["tetro-tui wezterm-gruvbox-swift-default"](https://github.com/Strophox/tetro-tui/blob/b66590cb461d34c95e988ef41b6d8b7d7783f37b/demo_assets/tetro-tui-v3.0.0_wezterm-gruvbox-swift-default.png?raw=true)
>
> </details>


## Ways to Run

### Download & Run

1. [Download a release](<https://github.com/Strophox/tetro-tui/releases>) for your platform (Linux/MacOS/Windows/...) if available.
2. Navigate to the application binary (`tetro-tui`/`tetro-tui.exe`/...) and run it in a terminal


### Quick Install

#### Cargo ([crates.io](<https://crates.io/crates/tetro-tui>)):
```bash
cargo install tetro-tui
#  tetro-tui
```


#### Arch Linux ([aur.archlinux.org](<https://aur.archlinux.org/packages?K=tetro-tui>)):

```bash
yay -S tetro-tui # Or paru, etc.
#  tetro-tui
```


### Compile from source

Tetro TUI is written in [Rust 1.95.0](<https://doc.rust-lang.org/book/ch01-00-getting-started.html>) and can be compiled as usual:
```bash
git clone https://github.com/Strophox/tetro-tui
cd tetro-tui
cargo run
```


## FAQ


### How does the base game work?

> *Tetro TUI* is about [tetromino](<https://en.wikipedia.org/wiki/Tetromino>) pieces falling from the sky and stacking inside a 2D playing field. When horizontal lines are filled they automatically clear away and everything 'stacked' above moves down.
> 
> A skilled player may keep playing indefinitely.
> Different game modes will change up the gameplay while still using the same base mechanics.


### How good is it in terms of configuration / features?

> We provide a solid amount of customization options / features for casual and (potentially) experienced players alike:
> - **Graphics:** Unicode/ASCII/Elektronika styles, old-terminal-compatible or very modern designs, 10 color palettes, hard drop/piece lock/line clear effects and much more.
> - **Game keybinds:** to your heart's desire. 
> - **Gameplay/handling:** Various rotation systems, piece randomizers, adjustable preview, timings (DAS¹, ARR¹, SDF¹, LCD, ARE), IRS/IHS/.. ([¹caveat](#why-do-some-gameplay-preferences-dasarrsdf-or-some-keybinds-ctrlshiftalt-not-work-for-me)).
> - **Game mode miscellany:** Regular ('Marathon'), Swift ('40-Lines'), Classic & Master (unlocked after Regular), Puzzle, Cheese, Combo, Custom (*select:* win condition, initial gravity, *toggle:* gravity progress, no top out, *cmdline flags:* start board, seed).
> - **Highscores, replays, statistics, ...** - can be viewed as well as backed up with a simple **savefile**.
>
> Visuals can be customized together with your underlying terminal settings.
> E.g. one can set a bigger font to scale the game, or use <a href="https://github.com/Swordfish90/cool-retro-term">cool-retro-term</a> for a nostalgic look etc.
> 
> <details>
> <summary>
> See also: Big overview of actual Tetro TUI v3.5 menus:
> </summary>
> 
> **- Title screen -**
> - New Game
> - Settings
> - Scores and Replays
> - All-Time Stats
> - About
> - Quit
> *[←↓↑→/Enter/Esc/Del] or Vim, Press [?] to view keybinds anytime*
> 
> **+ Start New Game +**
> - Regular: Clear 160 lines as gravity increases.
> - Swift: How quickly can you clear 40 lines?
> - *(unlocked after Regular)* Classic (lvl 0+)(*): 'NES' Gameplay/Graphics settings recommended.
> - *(unlocked after Regular)* Master: Clear 320 lines at instant gravity.
> - Puzzle: Clear 24 hand-crafted puzzles (feat. the Ocular rotation system)
> - Cheese-40: Efficiently eat through some lines. Limit∈[None, Some(10), Some(11), .., Some(40), ..]
> - Survival: Lines regenerate as you place more pieces.
> - Combo-30: Get consecutive line clears. Limit∈[None, Some(10), Some(11), .., Some(30), ..]
> - *(unlocked with Ctrl+U)* Placement: Practice placing pieces.
> - *(unlocked with Ctrl+U)* Ascent: Experimental gamemode (requires 180° rot.)
> - Custom: [Del]=reset
>   * Initial fall delay = 1.000000000s (Gravity: 1.00 Hz)
>   * Progressive gravity ∈ [on, off]
>   * Limit ∈ [None, TimeElapsed(300s), .., PointsScored(200), .., PiecesLocked(100), .., LinesCleared(40), ..]
> - *(available after using `Ctrl+S`)* Game save
> 
> **# Graphics Settings #**
> * Slot ∈ [Default, Guideline, Terminal compatibility, Focus+, Minimal, I⠐⢷⠗ Braille, Elektronika 60, NES; Custom I/II/..]
> * Tile colors ∈ [Terminal default, Just white, ANSI, Tetro Pastel, Guideline, Gruvbox, Solarized, Terafox, Fahrenheit, Matrix, Sequoia, Just amber, NES levels]
> * UI colors ∈ [Terminal default, Just black/white, Tetro Dark, Gruvbox Dark, Solarized Light, Matrix, Sequoia, Just amber, NES, OneHalfDark]
> * UI style ∈ [ASCII, Frame UTF8, Rounded frame UTF8, No frame UTF8, No hold/next-frame, Braille, Elektronika 60]
> * Hard drop effect ∈ [None, Particle trail ASCII, Particle trail 2 ASCII, Particle beam ASCII, Colored beam UTF8, White beam UTF8, Braille helix]
> * Lock effect ∈ [None, Flash white, Transforming ASCII, Pulsing block UTF8, Spiraling Braille]
> * Line clear effect ∈ [None, Disappear halfway, Disappear instantly, Blink, Flash white, Clear left-to-right, Clear outward, White clear inward, Burn outward, Pop, Pop (more), Confetti (gratuitous), Stardust, Blast, Sparks, Sparks Braille, Sparks ASCII]
> * Tile/Tetromino symbols ∈ [ASCII, Blocks UTF8, Braille, NES simulacra, Elektronika 60]
> * Small tetromino symbols ∈ [Dots ASCII, Blocks UTF8, Braille]
> * Mini tetromino symbols ∈ [Letters, Braille]
> * Normalsized tetromino previews ∈ [unlimited, 1, 2 ..]
> * Frames rendered per second ∈ [5, 10, .., 60, ..]
> * Grid ∈ [on, off]
> * Piece shadow ∈ [on, off]
> * Upcoming spawn preview (if stack high) ∈ [on, off]
> * Uniform locked tiles ∈ [on, off]
> * Main HUD ∈ [on, off]
> * Include basic keybinds HUD ∈ [on, off]
> * Show active/held buttons ∈ [on, off]
> * Lock delay visualizer ∈ [on, off]
> * FPS counter ∈ [on, off]
> 
> **@ Game Keybinds @**
> * Slot (modifiable preset) ∈ [Default, Guideline, Control+, Terminal finesse, Vim; Custom I/II/..]
> * MoveLeft, MoveRight;
> * RotateLeft, RotateRight, Rotate180;
> * DropSoft, DropHard;
> * TeleDown, TeleLeft, TeleRight;
> * HoldPiece.
> 
> **Settings/Adjust-Gameplay/**
> * Slot ∈ [Default, Guideline, Finesse+, Elektronika 60, Gameboy, NES; Custom I/II/..]
> * Piece rotation ∈ [Ocular, ClassicL, ClassicR, Super]
> * Piece randomization ∈ [Uniformly random, Classic (Reroll 1x), Rerollx 2x, .., 7-Bag, 14-Bag, .., Balance out, Recency (^2.5), Recency (^2.6), ..]
> * Piece preview count ∈ [0, 1, .., 4, ..]
> * Delayed auto move (DAS) ∈ [0ms, 1ms, .., 167ms, ..]
> * Auto repeat rate (ARR) ∈ [0ms, 1ms, .., 33ms, ..]
> * Delayed soft drop ∈ [None, Some(5ms), Some(10ms), .., Some(100ms), ..]
> * Soft drop rate (SDF) ∈ [raise gravity to 30 Hz, ..; 0x gravity, 0.25x gravity, .., 15x, .., inf x gravity]
> * Line clear duration (LCD) ∈ [0ms, 5ms, .., 200ms, ..]
> * Spawn delay (ARE) ∈ [0ms, 5ms, .., 50ms, ..]
> * Allow spawn manipulation (hold-IRS/IHS/IMS/ITS) ∈ [on, off]
> * Convert double-tap to teleport ∈ [None, Some(5ms), Some(10ms), ..]
> 
> **§ Advanced Settings §**
> * Save ∈ ["--Nothing", "Only settings,stats --No scores,replays", "Only settings,stats,scores --No replays", "Everything (settings,stats,scores,replays)"]
> * Renderer used ∈ [Main, Halfblock, Braille]
> * Pause on focus lost ∈ [on, off]
> * 'Blindfold' game ∈ [on, off]
> * Assume enhanced-key-events available ∈ [on, off]
> 
> </details>


### Why do some gameplay preferences (DAS/ARR/SDF...) or some keybinds (Ctrl/Shift/Alt/...) not work for me?

> It is likely that your current terminal provides **too little input information** to enable custom timings¹ or those special keys.
> (¹Instead, DAS/ARR/SDF will be determined by how quickly your *terminal* sends key-repeat events.)
> If possible use an **enhanced terminal** like <a href="https://sw.kovidgoyal.net/kitty/">Kitty</a> or <a href="https://alacritty.org/">Alacritty</a> (also <a href="https://docs.rs/crossterm/latest/crossterm/event/struct.PushKeyboardEnhancementFlags.html">others</a>) for flawless game handling.
> 
> <details>
> <summary>
> Explanation:
> </summary>
> 
> The fundamental problem lies in how terminals usually send input signals.
> - Due to historical reasons, most will only send "key pressed" but **not** "key released again".
>   This makes it impossible to implement mechanics such as:
>   "If `[←]` is pressed, move left with a certain speed *until key is released again*."
>   * Affected mechanics: Generally unable to actually 'hold' any buttons; DAS & ARR & SDF determined by terminal (& holding Soft Drop may 'accidentally' lock the piece), unable to hold Teleport, unable to hold for IRS/IHS/IMS/ITS.
>   * Note that some terminals e.g. on Windows *do* send key-release signals, without this being auto-detected:
>     Use the override in *Advanced Settings* for such cases.
> - Also due to history, modifier keys can only modify 'actual' text signals and are never sent by themselves. 
>   * Affected mechanics: Cannot register modifier `Ctrl`/`Alt`/`Shift`/`Win`/`⌘`/... as individual key presses.
> 
> Precisely these issues are fixed with ['enhanced keyboard events' / 'kitty keyboard protocol'](<https://sw.kovidgoyal.net/kitty/keyboard-protocol>).
>
> </details>


<!--NOTE: This list has not been updated since the addition of the in-app `?` (help) command in ~v3.X.
### Can you give me a table of all controls / shortcuts / keybinds?

> You can press `?` in every single menu to access a keybinds overview for it.
>
> Otherwise please refer to the tables below.
> 
> <details>
> <summary>
> Menu controls:
> </summary>
> 
> Universal:
> | Keys | ≈ Meaning |
> | -: | :- |
> | `Enter`, `e` | Select |
> | `Esc`, `q`, `Back`, | Go back |
> | `Del`, `d` | Delete/Reset |
> | `↓`/`↑`, `j`/`k` | Navigate down/up |
> | `←`/`→`, `h`/`l` | Adjust value |
> | `?` | Open Keybinds Overview |
> | `Ctrl`+`Alt`+`L` | Reload app from savefile (Caution: overwrites current data!) |
> | `Ctrl`+`Alt`+`S` | Perform savefile store (respects save preferences) |
> | `Ctrl`+`C` | Quit program (respects save preferences) |
>
> Specific to 'Scores and Replays':
> | Keys | Meaning |
> | -: | :- |
> | `Alt`+`Del`, `Alt`+`d` | Delete replay (in 'Scores and Replays') |
> | `Home`/`End` | Navigate to top/bottom |
>
> Specific to 'Start New Game':
> | Keys | Meaning |
> | -: | :- |
> | `Alt`+`←`/`→`, `Alt`+`h`/`l` | Adjust value differently (under ⇝['Combo','Custom','Game save']) |
> | `Home`/`End` | Set fall delay to infinite/zero (under ⇝'Custom'), jump to beginning/end of inpus (under ⇝'Game save') |
> | `Alt`+`Enter` | View replay instead of starting game save (under ⇝'Game save') |
> | `Ctrl`+`U` | (For experienced/impatient players) unlock all game modes |
> 
> Specific to 'Gameplay settings':
> | Keys | Meaning |
> | -: | :- |
> | `Alt`+`←`/`→`, `Alt`+`h`/`l` | Adjust value differently (under ⇝'Piece randomization'⇝['\*-Bag', 'Recency (\*)']) |
>
> </details>
> 
>
> <details>
> <summary>
> (Default) Game controls:
> </summary>
> 
> | Default Key (customizable) | Action |
> | -: | :- |
> | `←` | Move left |
> | `→` | Move right |
> | `A` | Rotate left (CCW) |
> | - | Rotate around (180°) |
> | `D` | Rotate right (CW) |
> | `↓` | Soft drop |
> | `↑` | Hard drop |
> | - | Teleport down |
> | - | Teleport left |
> | - | Teleport right |
> | `Space` | Hold piece |
> 
> | Key | Action |
> | -: | :- |
> | `Esc` | Pause game |
> | `?` | Open Keybinds Overview |
> 
> | Key | Special action |
> | -: | :- |
> | `Ctrl`+`D` | Forfeit game |
> | `Ctrl`+`R` | Restart game mode (Caution: discards current game) |
> | `Ctrl`+`Z` | Undo last input (Caution: overwrites current game) |
> | `Ctrl`+`L` | Load game save (Caution: overwrites current game) |
> | `Ctrl`+`S` | Store game save (accessible in 'Start New Game'⇝'Game save' or (live)'Game'⇝`Ctrl`+`L`) |
> | `Ctrl`+`E` | Store seed (accessible in 'Start New Game'⇝'Custom') |
> | `Ctrl`(+`Alt`)+`G` | Cycle through graphics settings slots |
> | `Ctrl`+`Alt`+`B` | Toggle on/off visibility of tiles ('Blindfolded') |
> | `Ctrl`+`Alt`+`L` | Reload app from savefile (Caution: overwrites current data!) |
> | `Ctrl`+`Alt`+`S` | Perform savefile store (respects save preferences) |
> | `Ctrl`+`C` | Quit program (respects save preferences) |
> 
> </details>
> 
>
> <details>
> <summary>
> Replay controls:
> </summary>
> 
> | Key | Action |
> | -: | :- |
> | `Enter`, `e` | Start Game from current replay state ('take over') |
> | `Esc`, `q`, `Back` | Exit replay |
> | `Space` | Pause replay |
> | `↓`/`↑` , `j`/`k` | Speed up / slow down replay by ±0.25x |
> | `-` | Reset replay speed to =1.0x |
> | `←`/`→` , `h`/`l` | Skip forward / backward 1s in time |
> | `.` | Skip forward one player input & pause |
> | `1`/`2`/`3`... | Jump to 10%/20%/30%/... |
> | `Home`/`End` | Jump to beginning/end |
> | `?` | Open Keybinds Overview |
> 
> | Key | Special action |
> | -: | :- |
> | `Alt`+`↓`/`↑`, `Alt`+`j`/`k` | Speed up / slow down replay by ±0.05x |
> | `Alt`+`.` | Skip forward one game state change & pause (might not work properly for modded games) |
> | `Ctrl`+`L` | Toggle replay looping |
> | `Ctrl`+`S` | Store game save (accessible in 'Start New Game'⇝'Game save' or (live)'Game'⇝`Ctrl`+`L`) |
> | `Ctrl`+`E` | Store seed (accessible in 'Start New Game'⇝'Custom') |
> | `Alt`+`I` | (Experimental) Toggle instantaneous interactive input intervention mode |
> | `Ctrl`(+`Alt`)+`G` | Cycle through Graphics Settings slots |
> | `Ctrl`+`Alt`+`L` | Reload app from savefile (Caution: overwrites current data!) |
> | `Ctrl`+`Alt`+`S` | Perform savefile store (respects save preferences) |
> | `Ctrl`+`C` | Quit program (respects save preferences) |
> 
> </details>-->


### Where's the config file? Will it clutter my system?

> <details>
> <summary>
> The application will not store anything by default and 'Keep save file' needs to be opted in;
> </summary>
> 
> The exact location of the config file is shown in the *Advanced Settings* menu and is based on `dirs::config_dir()`:
> - Usually `/home/yourname/.config/.tetro-tui_v1.0_savefile.json`
> - or `C:/User/yourname/AppData/Roaming/.tetro-tui_v1.0_savefile.json`
> 
> Savefile grows primarly due to number/length of saved replays.
> As a rule of thumb, 1min of gameplay with fast inputs adds ≲ 1 kB.
> If you end up with a lot of play time but can't/don't want to spare the kB / MB, you can:
> - Delete some entries (// just their replay data) in *Scores and Replays* using `[Del]` (// `[Alt+Del]`).
> - Configure which categories of data get stored in the first place on program exit (see *Advanced Settings*).
>
> </details>


### *Experienced players:* How does it compare to (or deviate from) common stacker games?

> At the time of writing we implement all the most common mechanics found in the wild (and then some).
> Groups of (e.g. gameplay) settings are bundled in a 'slot'(= settings template/profile) which allows us to provide common presets as well as our suggested defaults. 
> 
> <details>
> <summary>
> See list of notable differences:
> </summary>
>
> **Keybinds:**
> - Default controls suggested to be **WASD + Arrow keys** (reasoning: We prefer to assign movement and rotation to separate hands instead of mixing; do not use Shift key due to common [terminal limitations](#why-do-some-gameplay-preferences-dasarrsdf-or-some-keybinds-ctrlshiftalt-not-work-for-me)).
> - Dedicated buttons supplied for **Rotate 180°**, **Teleport Down** ('Sonic Drop') and Teleport Left/Right.
> 
> **Gameplay:**
> - Default rotation system suggested to be the [**Ocular Rotation** System](#experienced-players-what-is-this-ocular-rotation-system) (reasoning: Ocular is designed to be symmetrical, intuitive and flexible (in different ways) compared to the quirky / sometimes asymmetrical 'Super' Rot.Sys.).
> - Default piece randomizer suggested to be a **Recency Randomizer** (reasoning: 'Recency' is biased toward 'fairly' choosing less recent pieces but still technically allowing arbitrary piece sequences, compared to the 'overdeterministic' 7-Bag).
> - **Points (score) bonus** system is currently kept custom and simple.
>   - '1pt for simple line clear, with increasing bonus for larger lineclears, combos, spins and perfect clears.'
>   - *Note:* 'Allspin' without 'minis' (reasoning: we are not preoccupied with just 'T-spins').
>   - *Note:* Combos, without 'back-to-back' (reasoning: Back-to-back incentivizes playing special maneuvers, but those already yield disproportionate score bonus by themselves).
>   - Exact formula: `point_bonus = lineclears*lineclears * if is_spin{ 4 }else{ 1 } * if is_perfect{ 4 }else{ 1 } + (combo - 1)`
> - **Time-based lock reset limit** (reasoning: Providing a 'hard time limit before lock'(= `N`⋅current lock delay) seems more flexible/natural than 'hard move limit before lock'(=`N` 'moves').
> - Default speed/gravity/fall curve is slightly less aggressive but very close to standard (reason: We use a simpler formula).
>
> **Graphics:**
> - Default palette is more pastel (reasoning: Uniform perceptual brightness, 'looks pretty').
> 
> </details>
> 
> <details>
> <summary>
> See list of <a href="https://crates.io/crates/falling-tetromino-engine">core game engine</a> mechanics:
> </summary><!--NOTE: This list is quoted from the core game engine repository.-->
> 
> - Available player actions:
>     - **Move** left/right,
>     - **Rotate** left/right/180°
>     - **Drop** soft/hard
>     - **Teleport** down(='Sonic drop') and left/right
>     - **Hold** piece,
> - **Tetromino randomizers**: 'Uniform', 'Stock' (generalized Bag), 'Recency' (history), 'Balance-out',
> - **Piece preview** (arbitrary size),
> - **Spawn delay** (ARE),
> - **Spawn manipulation** (IRS/IHS/IMS/ITS; by keeping rotate/hold/move/teleport pressed during spawn),
> - **Rotation systems**: 'Ocular' (engine-specific, playtested), 'Classic', 'Super',
> - **Delayed auto-move** (DAS),
> - **Auto-move rate** (ARR),
> - **Soft drop rate** (SDF),
> - **Delayed soft drop** ('DAS but for Soft drop'),
> - **Customizable gravity/fall and lock delay curves** (exponential and/or linear; also, '20G' (fall rate of ≥1200 Hz) just becomes ≤00083s fall delay),
> - **Ensure move delay less than lock delay** toggle (i.e. DAS/ARR are automatically shortened when lock delay is very low),
> - **Allow lenient lock-reset** toggle (i.e. reset lock delay even if rotate/move fails),
> - **Lock-reset cap factor** (i.e. maximum time before lock delay cannot be reset),
> - **Line clear duration** (LCD),
> - **Customizable win/loss conditions** based on the time, pieces, lines, points,
> - Score more **points** for larger lineclears, spins ('allspin'), perfect clear, combo,
> - Game **reproducibility** (PRNG/determinism).
> 
> </details>


### *Experienced players:* What is the 'Ocular Rotation System'?

> <details>
> <summary>
> An extensive attempt at better tetromino rotation with regards to symmetry and visual intuition;
> </summary>
>
> The Ocular rotation system affords:
> - Symmetric/mirrored situations should lead to symmetric/mirrored outcomes (e.g. no distinct but visually identical states).
> - Rotation generally based on 'proximity where it looks like the piece should (be able to) go'.
> - Pieces should prefer downwards placement, not 'teleport up' in general.
>
> See this visual/'heatmap' comparison of the 'industry default' rotation system vs. Ocular rotation:
> 
> !["super rotation system heatmap"](https://github.com/Strophox/tetro-tui/blob/b66590cb461d34c95e988ef41b6d8b7d7783f37b/demo_assets/rotation-system-heatmap_srs.png?raw=true)
> 
> !["ocular rotation system heatmap"](https://github.com/Strophox/tetro-tui/blob/b66590cb461d34c95e988ef41b6d8b7d7783f37b/demo_assets/rotation-system-heatmap_ocular.png?raw=true)
> 
> </details>


### *Programmers / Terminal enthusiasts:* Can you provide some insight into the programming of this terminal game?

> <details>
> <summary>
> This project handles a handful of aspects to try and provide excellent user experience for a classic game. It also aims to maintain a decently high quality in code. The scope of this project consists of: 
> </summary>
> 
> - A **fully-featured [Tetromino game engine/backend](<https://github.com/Strophox/falling-tetromino-engine>)** featuring:
>   * Several dozens of **configurable, advanced and important options**.
>   * Provided pre-implementations of extremely common **standard mechanics**.
>   * Decoupled core game loop ('interpreter') with ergonomic API (hopefully).
>   * Basic but functional **modding functionality** with ergonomic API (hopefully).
> - **Different and interesting game modes**, including special game modes that rely on engine modding.
> - **Extensive TUI menus** to allow modifying all relevant configuration options without being forced to edit the save file manually:
    * **Graphics** options, **Configurable keybinds**, **Gameplay settings**.
>   * Providing many **curated configuration templates** for everything, inspired by existing standards and games.
>   * *Sidenote:* Ever since its inception as a proof-of-concept the terminal user interface (TUI) has directly and only relied on [Crossterm](<https://crates.io/crates/crossterm>). Currently there appears no need to change this situation, though a full TUI library like [Ratatui](<https://ratatui.rs/>) might be reconsidered e.g. to handle UI translation (displaying other languages) etc.
> - A competent **input-update-render game loop**.
> - Implementing **game replays**.
> - **Savefile** storage:
>   * In particular **replay data serialization and compression**.
> - **Scoreboard** and **statistics**.
> - Game **graphics renderer** that handles all of the **effects** and dozens of graphics settings, efficiently.
>   * Custom **buffer diff'ing** so we can guarantee we only send the minimum number of required changes to the terminal (this minimizes flicker), see <https://github.com/Strophox/tetro-tui/blob/7f0ebacee7a1ed8d399057e270f9071aa13aaaa8/src/game_renderers/standard_buffered/dense_terminal_double_buffer.rs#L103>.
> - Miscellaneous:
>   * Commandline arguments.
>   * Terminal limitations, all the time...
>   * Doing all of the above as simply, ergonomically and as correctly as possible while providing feedback to the user when something doesn't work as expected...
>   * Rust code quality.
> 
> </details>


### What is the motivation behind this project?

<!--NOTE: This text is the same as in the program 'About' menu. -->
> <details>
> <summary>
> Tetro TUI started as a passion project from someone who loves programming, minimalistic games and ASCII art;
> </summary>
> 
> Out of curiosity I snuck a peek to see how deep the mechanics of such a universal game can go:
> Basic versions are simple to code up, but it gets surprisingly complex when it comes to supporting all the modern/advanced features (especially while dealing with terminal limitations)!
> 
> To the best of my abilities I have implemented a most featureful & customizable version that still remains faithful to the essential
idea and also looks/runs nicely within a 'mere' terminal - Enjoy!
> ☺ [L.C.Werner](<https://github.com/Strophox>)
> 
> </details>


## License

Licensed under MIT.


## Provenance

100% human-sourced spaghetti code

Color palettes featured: [Gruvbox](<https://github.com/morhetz/gruvbox>), [Solarized](<https://ethanschoonover.com/solarized/>), [Terafox](<https://wezterm.org/colorschemes/t/index.html#terafox>), [Fahrenheit](<https://wezterm.org/colorschemes/f/index.html#fahrenheit>), [matrix](<https://wezterm.org/colorschemes/m/index.html#matrix>), [Sequoia Monochrome](<https://wezterm.org/colorschemes/s/index.html#sequoia-monochrome>), [OneHalfDark](<https://wezterm.org/colorschemes/o/index.html#onehalfdark>).


## Acknowledgements

A big thank you to the [AUR package](#arch-linux-aurarchlinuxorg) maintainers!
- [wcasanova](<https://github.com/wcasanova>), [druxorey](<https://github.com/druxorey>) and Dominiquini

Thank you to many sources of inspiration:
- Dunspixel – regarding ['O'-spins](<https://dunspixel.github.io/ospin-guide/chapter4.html#tetro-tui>)
- Martín G / mg1399 – regarding particle-based line clear effects from his own PICO-8 game
- thehuglet – regarding the [potential of terminal graphics](<https://github.com/thehuglet/germterm>)
- Akousoukos – regarding the [customizability of Apotris](<https://apotris.com/>)
- DoktorOcelot – regarding the addition of [Survival Zen mode in Tetr.js Enhanced](<https://doktorocelot.com/tetr.js/>)

Special Thanks
- GrBtAce, KonSola5 and bennxt – help during early dev/research
- madkiwi – help with 4-wide-6-residual combo layouts
- mathmaster13 and Kitaru – help with retro mechanics
- RayZN and ˗ˋˏthe One and Onlyˎˊ˗ – help with *Tetro* logo choice
- and Alexey Leonidovich Pajitnov – for discovering this universal genre of game
