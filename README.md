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

> *Tetro* is about [tetromino](<https://en.wikipedia.org/wiki/Tetromino>) pieces falling from the sky and stacking inside a 2D playing field. When a horizontal line is full it automatically clears away and everything 'stacked' above shifts down.
> 
> A skilled player can keep playing indefinitely.
> Different gamemodes may put slight twists on the base mechanics.


### How good are the features / customization options?

> A solid amount is available:
> - **Graphics:** Unicode/ASCII/Electronika, a handful of provided color palettes, FPS, toggle effects, ...
> - **Game keybinds:** to your heart's desire. (\*Note: `Shift`/`Alt`/.. might not work due to terminal limitations.)
> - **Gameplay/handling:** Rotation systems, randomizers, preview, timings (DAS, ARR, SDF, LDC, ARE), IRS/IHS.
> - **Gamemode selection:** Swift ('40lines'), Classic ('Marathon'), Master, Puzzle, Cheese, Combo, Custom (select goal, initial gravity, toggle gravity progress, *cmdline flags:* start board, seed).
> - **Scoreboard, Replays, Statistics...** - can all be accessed and automatically stored to savefile.
>
> TUI visuals depend on / can be customized using underlying terminal settings.
>
> <details>
> <summary>
> For example: Use a bigger font to scale the game, Use <a href="https://github.com/Swordfish90/cool-retro-term">cool-retro-term</a> for a retro look;
> </summary>
>
> !["tetro-tui running in cool-retro-term"](https://github.com/Strophox/tetro-tui/blob/3d98435167c779cb57651383d6b290d31e015013/demo_assets/tetro-tui_cool-retro-term.jpg?raw=true)
>
> </details>
> 
> <details>
> <summary>
> Full overview of featureful menus in Tetro TUI v2.0
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
> 
> **Settings/Adjust-Graphics/**
> * Slot ∈ ['Default', 'Focus+', 'Guideline', 'High Compat.', 'Elektronika 60', 'Custom I'/'II'/..]
> * Glyphset ∈ [Unicode, ASCII, Elektronika_60]
> * Color Palette (modifiable presets) ∈ ['Monochrome', 'ANSI', 'Fullcolor', 'Okpalette', 'Gruvbox', 'Solarized', 'Terafox', 'Fahrenheit', 'The Matrix', 'Sequoia']
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


### Why do some of the gameplay settings (DAS/ARR/SDF etc.) not work for me?

> <details>
> <summary>
> *In short:* Use an enhanced terminal like <a href="https://sw.kovidgoyal.net/kitty/">Kitty</a> or <a href="https://alacritty.org/">Alacritty</a> (also <a href="https://docs.rs/crossterm/latest/crossterm/event/struct.PushKeyboardEnhancementFlags.html">others</a>) for 'true' (smoother) game handling.
> Otherwise timings might depend solely on how quickly your terminal sends key-repeat events.
> </summary>
> 
> The fundamental problem lies in how terminals usually send signals for "key pressed", but not "key released again".
> This makes it impossible to implement mechanics such as, "if `[←]` is pressed, move left with a certain speed *until key is released again*."
> Precisely this issue is fixed with 'enhanced keyboard events' / ['progressive enhancement'](<https://sw.kovidgoyal.net/kitty/keyboard-protocol/#progressive-enhancement>) / 'kitty protocol'.
> 
> Note 1: Some terminals e.g. on Windows do send key-release signals, without this being auto-detected;
> Use the Override toggle in *Advanced Settings* if this is the case.
> 
> Note 2: A similar technicality affects the recognition of `[Shift]`/`[Alt]`/... key-presses.
> On unenhanced terminals, those keys cannot send key-presses by themselves (only in combination with a nonspecial-keys, e.g. `[Ctrl+C]`). 
>
> </details>


### How to navigate the terminal user interface (TUI) – is there a table of all controls?

> Refer to the following tables for all available controls:
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
> | `Alt`+? | Change value but differently (in 'New game'⇝['Combo','Savepoint','Custom'], in 'Gameplay settings'⇝'Tetromino generation') |
> | `Alt`+`Del`, `Alt`+`d` | Delete replay  (in 'Scores and Replays') |
> | `Ctrl`+`U` | (For experienced/impatient players) unlock all gamemodes (in 'New game') |
> | `Ctrl`+`C` | Exit application (respects save preferences) |
> 
> </details>
> 
>
> <details>
> <summary>
> Default live Game controls:
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
> | `Ctrl`+`D` | Forfeit game |
> | `Ctrl`+`E` | Store seed (accessible in 'New game'⇝'Custom') |
> | `Ctrl`+`S` | Store savepoint (accessible in 'New game'⇝'Savepoint' or in '(live) Game'⇝`Ctrl`+`L`) |
> | `Ctrl`+`L` | Load savepoint (Caution: overwrites live game) |
> | `Ctrl`+`Alt`+`B` | Toggle on/off visibility of tiles ('Blindfolded') |
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
> | `Ctrl`+`E` | Store seed (accessible in 'New game'⇝'Custom') |
> | `Ctrl`+`S` | Store savepoint (accessible in 'New game'⇝'Savepoint' or in '(live) Game'⇝`Ctrl`+`L`) |
> | `Ctrl`+`I` | Toggle experimental Instant Interactive Input Intervention |
> | `Ctrl`+`C` | Exit application (respects save preferences) |
> 
> </details>


### Where's the config file? Will it clutter my system?

> <details>
> <summary>
> The application will not store anything by default and 'Keep save file' needs to be opted in.
> </summary>
> 
> The exact location of the config file is shown in the *Advanced Settings* menu and is based on `dirs::config_dir()` (usually `C:/User/yourname/AppData/Roaming/.tetro-tui_v1.0_savefile.json` or `/home/yourname/.config/.tetro-tui_v1.0_savefile.json`).
> 
> Savefile grows mostly with number/length of replays saved.
> If you end up with a lot of play time but don't want to spare the kBs/MBs, you can
> - Delete select entries (or just their replay data) in *Scores and Replays* using `[Del]` (or `[Alt+Del]`, respectively).
> - Configure which data gets stored in the first place by the application (in *Advanced Settings*).
>
> </details>


### *Experienced players:* How 'deep'/extensive are the precise stacker mechanics?

> <details>
> <summary>
> See this feature list from the <a href="https://crates.io/crates/falling-tetromino-engine">Falling Tetromino Engine</a> that powers the actual game logic:
> </summary>
> 
> The engine aims to compete on the order of modern tetromino stackers;
> It should incorporate many mechanics desired by familiar/experienced players, such as:
> - Available player actions:
>     - Move left/right,
>     - Rotate left/right/180°
>     - Drop soft/hard
>     - Teleport down(='Sonic drop')/left/right
>     - Hold piece,
> - **Tetromino randomizers**: 'Uniform', 'Stock' (generalized Bag), 'Recency' (history), 'Balance-out',
> - **Piece preview** (arbitrary size),
> - **Spawn delay** (ARE),
> - **Initial actions** on-piece-spawn toggle ('Initial Hold/Rotation System'),
> - **Rotation systems**: 'Ocular' (engine-specific, playtested), 'Classic', 'Super',
> - **Delayed auto-move** (DAS),
> - **Auto-move rate** (ARR),
> - **Soft drop factor** (SDF),
> - **Customizable gravity/fall and lock delay curves** (including '20G' = 0s fall delay),
> - **Ensure move delay less than lock delay** toggle (i.e. DAS/ARR are automatically shortened when lock delay is very low),
> - **Allow lenient lock-reset** toggle (i.e. reset lock delay even if rotate/move fails),
> - **Lock-reset cap factor** (i.e. maximum time before lock delay cannot be reset),
> - **Line clear duration** (LCD),
> - **Customizable win/loss conditions** based on the time, pieces, lines, score,
> - Score more **points** for larger lineclears and spins ('allspin'),
> - Game **reproducibility** (PRNG).
> 
> </details>


### *Experienced players:* In which ways is it unlike familiar stacker games?

> <details>
> <summary>
> This project takes some liberties to adapt/experiment with certain aspects of the game, though it should feel extremely familiar:
> </summary>
> 
> - Default controls set to **WASD + Arrow keys** (swappable).
> - Default use of the symmetrical and flexible **Ocular Rotation** System (instead of the sometimeis quirky industry standard) (swappable).
> - Default **Recency (History) Randomizer** (instead of 'overdeterministic' 7-Bag) (swappable).
> - **Points (score) bonus** system is custom and kept simple.
>   - "1pt for simple line clear, more points incentivizing larger lineclears, spins, perfects and combos."
>   - 'Allspin' (instead of preoccupation with 'T-spins'), currently no 'minis'.
>   - Combos (but no 'back-to-back').
>   - ...Exact formula: `points_bonus = if is_perfect_clear{ 4 }else{ 1 } * if is_spin{ 2 }else{ 1 } * (lineclears * 2 - 1) + (combo - 1)`
> - Controls availble for **Teleport Down ('Sonic Drop')** / Left / Right.
> - Different **lock reset** / lock-down cutoff: 'max time = 10⋅current lock delay' (instead of 'max 15 moves with current lock delay').
> - Speed/Gravity/Fall curve practically same but technically slightly adapted (adjustable for custom game via savefile).
> 
> </details>


### *Experienced players:* What is the 'Ocular Rotation System'?

> <details>
> <summary>
> A serious attempt at better tetromino rotation, based on visual intuition and symmetry:
> </summary>
>
> The Ocular rotation system affords:
> - Rotation generally based on 'proximity where it looks like the piece should be able to go'.
> - Symmetric/mirrored situations should lead to symmetric/mirrored outcomes.
> - Pieces should not 'teleport up' a lot.
>
> See visual 'heatmap' comparison of Super vs. Ocular rotation:
> 
> !["super rotation system heatmap"](https://github.com/Strophox/tetro-tui/blob/3d98435167c779cb57651383d6b290d31e015013/demo_assets/super-rotation_heatmap.png?raw=true)
> 
> !["ocular rotation system heatmap"](https://github.com/Strophox/tetro-tui/blob/3d98435167c779cb57651383d6b290d31e015013/demo_assets/ocular-rotation_heatmap.png?raw=true)
> 
> </details>


### *CLI enthusiasts:* How was the TUI programmed and why isn't it [Ratatui](<https://ratatui.rs/>)?

> Since its origin as a proof-of-concept this project has directly used [Crossterm](<https://crates.io/crates/crossterm>).
> As of writing, the most complicated terminal interaction is custom diff'ing so the game is rendered smoothly (no flicker).
> Currently there appears no need to change this situation, though Ratatui will be reconsidered if necessary.


### What is the background behind this project?

> <details>
> <summary>
> Tetro TUI started as a passion project from someone who loves programming, minimalistic games and ASCII art.
> </summary>
> 
> Personal motivation drove me to research 'Tetr\*slikes':
> Basic versions are simple to code up, yet it can get surprisingly nontrivial when it comes to comprehensive support of modern/advanced/'quality of life' mechanics!
> 
> In the end I've put in my best effort to implement a most featureful and customizable version that not only remains totally faithful to the [basic idea of the game](<https://github.com/Strophox/falling-tetromino-engine>), but also fulfils the requirement that it should run and look nice in any mere terminal!
> 
> Enjoy :-). --Strophox
> 
> </details>


## License

Licensed under MIT.


## Provenance

100% human-sourced spaghetti code

Color palettes used: [Gruvbox](<https://github.com/morhetz/gruvbox>), [Solarized](<https://ethanschoonover.com/solarized/>), [Terafox](<https://wezterm.org/colorschemes/t/index.html#terafox>), [Fahrenheit](<https://wezterm.org/colorschemes/f/index.html#fahrenheit>), [matrix](<https://wezterm.org/colorschemes/m/index.html#matrix>), [Sequoia Monochrome](<https://wezterm.org/colorschemes/s/index.html#sequoia-monochrome>).


## Acknowledgements

Special Thanks go to:
- [wcasanova](<https://github.com/wcasanova>) and [druxorey](<https://github.com/druxorey>) – AUR package maintainers
- GrBtAce, KonSola5 and bennxt – for early support
- madkiwi – for advice regarding 4wide-6residual combo layouts
- Dunspixel – for inspiration regarding ['O'-spins](<https://dunspixel.github.io/ospin-guide/chapter4.html#tetro-tui>)
- Martín G – for inspiration regarding new line clear effect from his own PICO-8 game
- Akousoukos – for making [Apotris](<https://apotris.com/>)
- and RayZN and ˗ˋˏthe One and Onlyˎˊ˗ – for advice regarding the Tetro logo
