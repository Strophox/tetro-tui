!["Tetro TUI logo"](https://github.com/Strophox/tetro-tui/blob/3d98435167c779cb57651383d6b290d31e015013/demo_assets/tetro_logo_glow.png?raw=true)


# Tetro TUI - Terminal Game

[![Crates.io](https://img.shields.io/crates/v/tetro-tui.svg)](https://crates.io/crates/tetro-tui)
[![License](https://img.shields.io/crates/l/tetro-tui)](https://github.com/Strophox/tetro-tui#license)
<!--[![Documentation](https://docs.rs/tetro-tui/badge.svg)](https://docs.rs/tetro-tui)-->

Tetro TUI is a terminal-based but modern tetromino-stacking game that is very customizable and runs cross-platform.

!["tetro-tui demonstration GIF"](https://github.com/Strophox/tetro-tui/blob/3d98435167c779cb57651383d6b290d31e015013/demo_assets/tetro-tui_demo-1.0.0.gif?raw=true)


## Ways to Run

### Download & run

1. [Download a release](<https://github.com/Strophox/tetro-tui/releases>) for your platform (Linux/MacOS/Windows/...) if available.
2. Navigate to the application (`tetro-tui`/`tetro-tui.exe`/...) and run it


### Compile from source

Tetro TUI is written in [Rust](<https://doc.rust-lang.org/book/ch01-01-installation.html>) and can be compiled as usual:
```
git clone https://github.com/Strophox/tetro-tui # Or otherwise download source code.
cd tetro-tui
cargo run
```


### Install via cargo

Tetro TUI is available on [crates.io](<https://crates.io/crates/tetro-tui>).
It can be installed via [cargo](<https://doc.rust-lang.org/cargo/>):
```
cargo install tetro-tui
```
This makes `tetro-tui` available to run for your terminal.


### Install on Arch Linux

Tetro TUI is available on [aur.archlinux.org](<https://aur.archlinux.org/packages?K=tetro-tui>).
It can be installed e.g. via [yay](<https://github.com/Jguer/yay>) or [paru](<https://github.com/Morganamilo/paru>):
```
yay -S tetro-tui
```


## FAQ


### How does the base game work?

> *Tetro TUI* is about [tetromino](<https://en.wikipedia.org/wiki/Tetromino>) pieces falling from the sky and stacking inside a 2D playing field. When a horizontal line is full it automatically clears away and everything 'stacked' above shifts down.
> 
> A skilled player may keep playing indefinitely.
> Different game modes will change up the gameplay while still using the same base mechanics.


### How good is it in terms of customization / features?

> We provide a solid amount of configuration options and features:
> - **Graphics:** Unicode/ASCII/Electronika, a handful of provided color palettes, FPS, toggle effects, ...
> - **Gameplay / handling:** Rotation systems, randomizers, preview, timings (DAS, ARR, SDF, LDC, ARE), IRS/IHS (\*[caveat](#why-do-some-gameplay-settings-dasarretc-or-a-simple-shift-keypress-not-work-for-me)).
> - **Game keybinds:** to your heart's desire. 
> - **Game mode miscellany:** Swift ('40lines'), Classic ('Marathon'), Master, Puzzle, Cheese, Combo, Custom (select goal, initial gravity, toggle gravity progress, *cmdline flags:* start board, seed).
> - **Highscores, replays, statistics...** - can can be accessed as well as backed up with a **simple savefile**.
>
> TUI visuals depend on / can be customized using your underlying terminal settings.
>
> <details>
> <summary>
> E.g. set a bigger font to scale the game, or use <a href="https://github.com/Swordfish90/cool-retro-term">cool-retro-term</a> for a nostalgic look:
> </summary>
>
> !["tetro-tui running in cool-retro-term"](https://github.com/Strophox/tetro-tui/blob/3d98435167c779cb57651383d6b290d31e015013/demo_assets/tetro-tui_cool-retro-term.jpg?raw=true)
>
> </details>
> 
> <details>
> <summary>
> (Comprehensive overview of relevant menus (Tetro TUI v2.0):)
> </summary>
> 
> **New game/**
> - Swift: How fast can you clear 40 lines?
> - Classic: Clear 150 lines at increasing gravity.
> - Master: Clear 150 lines at instant gravity.
> - Puzzle: Clear 24 hand-crafted puzzles.
> - Cheese-20: Eat through lines like Swiss cheese. Limit∈[None, Some(10), Some(11), .., Some(20), ..]
> - Combo-30: Get consecutive line clears. Limit∈[None, Some(10), Some(11), .., Some(30), ..]
> - Ascent*: (experimental, req. Ocular + 180° rot.)
> - Custom: [Del]=reset
>   * Initial fall delay = 1.0s (Gravity: 1.0 Hz)
>   * Progressive gravity ∈ [on, off]
>   * Limit ∈ [None, TimeElapsed(300s), .., PointsScored(200), .., PiecesLocked(100), .., LinesCleared(40), ..]
> - Game save: (Only shows up after using `Ctrl+S`)
> 
> **Settings/Adjust-Graphics/**
> * Slot ∈ ['Default', 'Focus+', 'Guideline', 'High Compat.', 'Elektronika 60', 'Custom I'/'II'/..]
> * Glyphset ∈ [Unicode, ASCII, Elektronika_60]
> * Color Palette (modifiable presets) ∈ ['Monochrome', 'ANSI', 'Standard', 'Okpalette', 'Gruvbox', 'Solarized', 'Terafox', 'Fahrenheit', 'The Matrix', 'Sequoia']
> * Color locked tiles ∈ [on, off]
> * Show effects ∈ [on, off]
> * Show shadow piece ∈ [on, off]
> * Show button state ∈ [on, off]
> * Max framerate ∈ [1, .., 60, ..]
> * Show FPS ∈ [on, off]
> 
> **Settings/Adjust-Keybinds/**
> * Slot (modifiable preset) ∈ ['Default', 'Control+', 'Guideline', 'Vim', 'Custom I'/'II'/..]
> * MoveLeft, MoveRight
> * RotateLeft, RotateRight, Rotate180
> * DropSoft, DropHard
> * TeleDown, TeleLeft, TeleRight
> * HoldPiece
> 
> **Settings/Adjust-Gameplay/**
> * Slot ∈ ['Default', 'Finesse+', 'Guideline', 'NES', 'Gameboy', 'Custom I'/'II'/..]
> * Piece rotation system ∈ [Ocular, ClassicL, ClassicR, Super]
> * Piece randomization ∈ [Completely random, 7-Bag, 14-Bag, .., Recency (^2.5), Recency (^2.6), .., Balance out]
> * Piece preview count ∈ [0, 1, .., 3, ..]
> * Delayed auto move (DAS) ∈ [0ms, 1ms, .., 167ms, ..]
> * Auto move rate (ARR) ∈ [0ms, 1ms, .., 33ms, ..]
> * Soft drop speedup (SDF) ∈ [0x, 0.25x, .., 15x, ..]
> * Line clear duration (LCD) ∈ [0ms, 5ms, .., 200ms, ..]
> * Spawn delay (ARE) ∈ [0ms, 5ms, .., 50ms, ..]
> * Allow initial rotation/hold (IRS/IHS) ∈ [on, off]
> * Convert double-tap to teleport ∈ [None, Some(5ms), Some(10ms), ..]
> 
> **Settings/Advanced-Settings/**
> * Save contents ∈ ["Nothing", "Only settings - No scores,replays", "Only settings,scres - No replays", "Everything (settings,scores,replays)"]
> * Assume enhanced-key-events available ∈ [on, off]
> * Blindfold gameplay ∈ [on, off]
> * Renderertype ∈ [Default, Legacy debug, Halfcell, Braille]
> 
> </details>


### Why do some gameplay settings (DAS/ARR/etc.) or a simple `Shift` keypress not work for me?

> *In short:* If possible use an enhanced terminal like <a href="https://sw.kovidgoyal.net/kitty/">Kitty</a> or <a href="https://alacritty.org/">Alacritty</a> (also <a href="https://docs.rs/crossterm/latest/crossterm/event/struct.PushKeyboardEnhancementFlags.html">others</a>) for flawless game handling.
> Otherwise e.g. timings might depend solely on how quickly your *terminal* sends key-repeat events.
> 
> <details>
> <summary>
> List of possible terminal limitations:
> </summary>
> 
> - Unenhanced terminals **cannot** implement mechanics related to **holding**. This includes:
>     * DAS, ARR, SDF; holding Soft Drop not locking the piece; holding a Teleport button; hold-type IRS/IHS.
> - Unenhanced terminals **cannot** recognize `Ctrl`/`[Shift]`/`[Alt]` modifiers as individual keys (only in combination, e.g. `[Ctrl+C]`).
> 
> </details>
> 
> <details>
> <summary>
> Explanation:
> </summary>
> 
> The fundamental problem lies in how terminals usually send signals.
> - Since most only send "key pressed" but not "key released again", this makes it impossible to implement mechanics such as: "If `[←]` is pressed, move left with a certain speed *until key is released again*."
> - Modifiers like `Ctrl` and `Shift` can only modify 'actual' text signals and aren't sent by themselves. 
> These issues precisely are fixed with 'enhanced keyboard events' / ['progressive enhancement'](<https://sw.kovidgoyal.net/kitty/keyboard-protocol/#progressive-enhancement>) / 'kitty protocol'.
> 
> Some terminals e.g. on Windows *do* send key-release signals, without this being auto-detected:
> Use the 'Override' in *Advanced Settings* in this case.
>
> </details>


### Is there a table of all controls / keybinds?

> Please refer to the tables below.
> 
> <details>
> <summary>
> General TUI menu controls:
> </summary>
> 
> | Keys | ≈ Meaning |
> | -: | :- |
> | `↓`/`↑`, `j`/`k` | Navigate up/down |
> | `←`/`→`, `h`/`l` | Change value |
> | `Enter`, `e` | Select |
> | `Esc`, `q`, `Back`, | Go back |
> | `Del`, `d` | Delete/reset |
> | `1`/`2`/`3`... | Quickselect option (in 'New game') |
> | `Home`/`End` | Navigate to top/bottom in 'Scores & Replays' |
> | `Alt`+? | Change value but differently (in 'Start game'⇝['Combo','Game save','Custom'], in 'Gameplay settings'⇝'Tetromino generation') |
> | `Alt`+`Del`, `Alt`+`d` | Delete replay  (in 'Scores and Replays') |
> | `Ctrl`+`U` | (For experienced/impatient players) unlock all game modes (in 'Start game') |
> | `Ctrl`+`C` | Exit application (respects save preferences) |
> 
> </details>
> 
>
> <details>
> <summary>
> Default (live) Game controls:
> </summary>
> 
> | Key | Action |
> | -: | :- |
> | `Esc` | Pause game |
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
> | Key | Special Action |
> | `Ctrl`+`D` | Forfeit game |
> | `Ctrl`+`R` | Restart game mode (Caution: discards current game) |
> | `Ctrl`+`L` | Load game save (Caution: discards current game) |
> | `Ctrl`+`S` | Store game save (accessible in 'Start game'⇝'Game save' or '(live) Game'⇝`Ctrl`+`L`) |
> | `Ctrl`+`E` | Store seed (accessible in 'Start game'⇝'Custom') |
> | `Ctrl`(+`Alt`)+`G` | Cycle through graphics settings slots |
> | `Ctrl`+`Alt`+`B` | Toggle on/off visibility of tiles ('Blindfolded') |
> | `Ctrl`+`Alt`+`L` | Re-load from savefile (Caution: discards current save progress!) |
> | `Ctrl`+`C` | Exit application (respects save preferences) |
> 
> </details>
> 
>
> <details>
> <summary>
> Replay Game controls:
> </summary>
> 
> | Key | Action |
> | -: | :- |
> | `Esc`, `q`, `Back` | Exit replay |
> | `Space` | Pause replay |
> | `↓`/`↑` , `j`/`k` | Speed up / slow down replay by ±0.25x |
> | `Alt`+`↓`/`↑`, `Alt`+`j`/`k` | Speed up / slow down replay by ±0.05x |
> | `-` | Reset replay speed to =1.0x |
> | `←`/`→` , `h`/`l` | Skip forward / backward 1s in time |
> | `1`/`2`/`3`... | Jump to 10%/20%/30%/... |
> | `.` | Skip forward one player input + pause |
> | `Alt`+`.` | Skip forward one game state change\* + pause (\*might not work properly for modded games) |
> | `Enter`, `e` | Start (live) Game from current replay state |
> 
> | Key | Special Action |
> | `Ctrl`+`L` | Loop replay on game end |
> | `Ctrl`+`S` | Store game save (accessible in 'Start game'⇝'Game save' or '(live) Game'⇝`Ctrl`+`L`) |
> | `Ctrl`+`E` | Store seed (accessible in 'Start game'⇝'Custom') |
> | `Ctrl`+`I` | (Experimental) Toggle Instant Interactive Input Intervention |
> | `Ctrl`(+`Alt`)+`G` | Cycle through graphics settings slots |
> | `Ctrl`+`Alt`+`L` | Re-load from savefile (Caution: discards current save progress!) |
> | `Ctrl`+`C` | Exit application (respects save preferences) |
> 
> </details>


### Where's the config file? Will it clutter my system?

> <details>
> <summary>
> The application will not store anything by default and 'Keep save file' needs to be opted in:
> </summary>
> 
> The exact location of the config file is shown in the *Advanced Settings* menu and is based on `dirs::config_dir()` (usually `C:/User/yourname/AppData/Roaming/.tetro-tui_v1.0_savefile.json` or `/home/yourname/.config/.tetro-tui_v1.0_savefile.json`).
> 
> Savefile grows mostly with number/length of replays saved.
> If you end up with a lot of play time but can't/don't want to spare the kB / MB, you can:
> - Delete some entries (// just their replay data) in *Scores and Replays* using `[Del]` (// `[Alt+Del]`).
> - Configure which categories of data get stored in the first place on program exit (see *Advanced Settings*).
> - As a rule of thumb, 1min of gameplay with fast inputs adds ≈ 1 kB.
>
> </details>


### *Experienced players:* How extensive are the stacker mechanics exactly?

> <details>
> <summary>
> See the following feature list from the <a href="https://crates.io/crates/falling-tetromino-engine">Falling Tetromino Engine</a> powering our game logic:
> </summary>
> 
> In terms of advanced game mechanics the engine aims to compare with other modern tetromino stackers.
> It should already incorporate many features desired by familiar/experienced players, such as:
> - Available player actions:
>     - **Move** left/right,
>     - **Rotate** left/right/180°
>     - **Drop** soft/hard
>     - **Teleport** down(='Sonic drop') and left/right
>     - **Hold** piece,
> - **Tetromino randomizers**: 'Uniform', 'Stock' (generalized Bag), 'Recency' (history), 'Balance-out',
> - **Piece preview** (arbitrary size),
> - **Spawn delay** (ARE),
> - **Spawn actions** (IRS/IHS; by keeping rotate/hold pressed during spawn),
> - **Rotation systems**: 'Ocular' (engine-specific, playtested), 'Classic', 'Super',
> - **Delayed auto-move** (DAS),
> - **Auto-move rate** (ARR),
> - **Soft drop factor** (SDF),
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


### *Experienced players:* Does it play like familiar / industry-standard stacker games?

> We put the customizability of Tetro TUI to good use and provide a handful of settings templates, e.g. to emulate guideline (gameplay, keybinds, graphics).
>
> <details>
> <summary>
> The default settings – though they should feel acutely familiar – do take liberty in shifting some mechanics 'closer to the platonic ideal' of the game. This is obviously an informal distinction and in practice means:
> </summary>
> 
> **Keybinds:**
> - Default controls set to **WASD + Arrow keys** (this also works better due to terminal limitations).
> - Dedicated binds possible for **Rotate 180°**, **Teleport Down** ('Sonic Drop'), even Teleport Left/Right.
> 
> **Gameplay:**
> - Default use of the flexible/intuitive/symmetrical [**Ocular Rotation** System](#experienced-players-what-is-this-ocular-rotation-system) (instead of the quirky/sometimes asymmetrical industry default).
> - Default **Recency (History) Randomizer** (instead of 'overdeterministic' 7-Bag).
> - **Points (score) bonus** system is currently kept custom and simple.
>   - '1pt for simple line clear, with increasing bonus for larger lineclears, combos, spins and perfect clears.'
>   - *Note:* 'Allspin' (instead of preoccupation with 'T-spins'), currently no 'minis'.
>   - *Note:* Combos (but no additional points for 'back to back' other than existing incentives for special maneuvers).
>   - Exact formula: `point_bonus = if is_perfect_clear{ 4 }else{ 1 } * if is_spin{ 2 }else{ 1 } * (lineclears * 2 - 1) + (combo - 1)`
> - Different **lock reset** limit: 'max time = 10⋅current lock delay' (instead of 'max 15 moves with current lock delay').
> - Speed/gravity/fall curve technically a customizable mix of exponential decay + linear decrease.
> 
> </details>


### *Experienced players:* What is this 'Ocular Rotation System'?

> <details>
> <summary>
> An extensive attempt at better tetromino rotation with regards to symmetry and visual intuition:
> </summary>
>
> The Ocular rotation system affords:
> - Symmetric/mirrored situations should lead to symmetric/mirrored outcomes (e.g. no distinct but visually identical states).
> - Rotation generally based on 'proximity where it looks like the piece should (be able to) go'.
> - Pieces should prefer downwards placement, not 'teleport up' in general.
>
> See this visual/'heatmap' comparison of industry default vs. Ocular rotation:
> 
> !["super rotation system heatmap"](https://github.com/Strophox/tetro-tui/blob/3d98435167c779cb57651383d6b290d31e015013/demo_assets/super-rotation_heatmap.png?raw=true)
> 
> !["ocular rotation system heatmap"](https://github.com/Strophox/tetro-tui/blob/3d98435167c779cb57651383d6b290d31e015013/demo_assets/ocular-rotation_heatmap.png?raw=true)
> 
> </details>


### *Terminal enthusiasts:* How was this terminal game programmed (and why doesn't it use [Ratatui](<https://ratatui.rs/>))?

> Ever since its inception as a proof-of-concept this terminal user interface (TUI) has directly used [Crossterm](<https://crates.io/crates/crossterm>) for all I/O.
> The most complicated terminal interaction we currently implement is custom diff'ing so we render the minimum number of visual game changes (minimize flicker).
> Currently there appears no need to change this situation, though Ratatui will be reconsidered if necessary (e.g. language/localization...)
>
> *Author's Note on program complexity:* Though Tetro TUI has grown far beyond a quick weekend project in terms of features, the code should not be doing anything too crazy / bloated. Consider it a quick weekend project with basic feature breadth and polish scaled by ×42.


### What is the background behind this project?

> <details>
> <summary>
> Tetro TUI started as a passion project from someone who loves programming, minimalistic games and ASCII art:
> </summary>
> 
> Personal motivation drove me to research 'Tetr\*slikes':
> Basic versions are simple to code up, yet it can get surprisingly nontrivial when it comes to comprehensive support of modern/advanced/'quality of life' mechanics seen in top-level play.
> 
> All said and done, I've put in my best effort to implement a most featureful and customizable version that not only remains totally faithful to the [basic idea of the game](<https://github.com/Strophox/falling-tetromino-engine>), but also runs and looks nice within the confines of a mere terminal.
> 
> Enjoy! ☻[Strophox](<https://github.com/Strophox>)
> 
> </details>


## License

Licensed under MIT.


## Provenance

100% human-sourced spaghetti code

Color palettes used: [Gruvbox](<https://github.com/morhetz/gruvbox>), [Solarized](<https://ethanschoonover.com/solarized/>), [Terafox](<https://wezterm.org/colorschemes/t/index.html#terafox>), [Fahrenheit](<https://wezterm.org/colorschemes/f/index.html#fahrenheit>), [matrix](<https://wezterm.org/colorschemes/m/index.html#matrix>), [Sequoia Monochrome](<https://wezterm.org/colorschemes/s/index.html#sequoia-monochrome>).


## Acknowledgements

A Thank You to the [AUR package](#install-on-arch-linux) maintainers:
- [wcasanova](<https://github.com/wcasanova>) and [druxorey](<https://github.com/druxorey>), and Dominiquini

Special Thanks go to:
- GrBtAce, KonSola5 and bennxt – for early support
- madkiwi – for advice regarding 4wide-6residual combo layouts
- Dunspixel – for inspiration regarding ['O'-spins](<https://dunspixel.github.io/ospin-guide/chapter4.html#tetro-tui>)
- Martín G – for inspiration regarding new line clear effect from his own PICO-8 game
- Akousoukos – for making [Apotris](<https://apotris.com/>)
- and RayZN and ˗ˋˏthe One and Onlyˎˊ˗ – for advice regarding the Tetro logo
