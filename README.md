!["tetro-tui logo"](https://github.com/Strophox/tetro-tui/blob/a78483f9ba145798201f83ad2e4dc760ba918916/assets/tetro_logo_glow.png?raw=true)


# Tetro TUI - Cross-platform Terminal Game

[![Crates.io](https://img.shields.io/crates/v/tetro-tui.svg)](https://crates.io/crates/tetro-tui)
[![License](https://img.shields.io/crates/l/tetro-tui)](https://github.com/Strophox/tetro-tui#license)
<!--[![Documentation](https://docs.rs/tetro-tui/badge.svg)](https://docs.rs/tetro-tui)-->

A cross-platform<!--, very customizable--> terminal game where tetrominos fall and stack.

!["tetro-tui demonstration GIF"](https://github.com/Strophox/tetro-tui/blob/ec952782218e360e38efb945c849cfe69c3f00c3/assets/tetro-tui_demo-1.0.0.gif?raw=true)


## Ways to Run

### Download + run

1. [Download a release](<https://github.com/Strophox/tetro-tui/releases>) for your platform (Linux, MacOS, Windows, ..) if available.
2. Navigate to and run the application (`tetro-tui`)


### Compile from source

1. Ensure [Rust](<https://doc.rust-lang.org/book/ch01-01-installation.html>) is installed.
2. `git clone https://github.com/Strophox/tetro-tui` or otherwise download this repository.
3. Navigate inside `tetro-tui/` and do `cargo run`.


### Install via cargo

Tetro TUI is available on [crates.io](<https://crates.io/crates/tetro-tui>). It can be installed via [cargo](<https://doc.rust-lang.org/cargo/>):
```
cargo install tetro-tui
```
This makes `tetro-tui` available to run on your terminal.


<!-- TODO: Elaborate.
### AUR Package

https://aur.archlinux.org/packages/tetro-tui-bin
-->


## FAQ


### How does the game work?

> *Tetro* is about [tetromino](<https://en.wikipedia.org/wiki/Tetromino>) pieces falling from the sky and stacking on a rectangular playing field.
> 
> Whenever a line is filled up horizontally, it clears away, and the rest of what you 'stacked' moves down.
> This way a skilled player can keep playing without blocking out the entire board.


### How good is Customization? Features?

> For what originally started as a small/proof-of-concept, solid.
> - **Graphics:** Unicode/ASCII/Electronika; Handful of default color palettes, FPS, toggle effects, ...
> - **Game keybinds:** to your heart's desire. (\*Note: `Shift`/`Alt`/.. might not work due to terminal limitations.)
> - **Gameplay/handling:** Rotation system, tetromino randomization, preview count, DAS, ARR, SDF, LDC, ARE (timings), IRS/IHS.
> - **Gamemode selection:** Swift ('40lines'), Classic ('marathon'), Master, Puzzle, Cheese, Combo, Custom (select goal, start gravity, toggle gravity progress, *cmdline flags:* start board, seed).
> - **Scoreboard, Replays, Statistics**: Can all be accessed and automatically stored in savefile.
>
> <details>
> <summary>
>
> Game aesthetics are mostly based on / can be customized using own terminal settings, e.g. bigger font, or use of [cool-retro-term](<https://github.com/Swordfish90/cool-retro-term>);
>
> </summary>
>
> !["tetro-tui running in cool-retro-term"](https://github.com/Strophox/tetro-tui/blob/011251ba7cbfaca03414c910c5483791f3fef737/assets/tetro-tui_cool-retro-term.jpg?raw=true)
>
> </details>


### What was the motivation behind this project?

> This is a passion project!
> The additions of all the many features stem from personal motivation to make them available and make things enjoyable/customizable.
> 
> The result is hopefully decent customizability, advanced game mechanics, technical solutions across the board:
> Swappable settings slots/profiles to deal with all the knobs and buttons (manual json editing possible), basic game replay compression; nontrivial gamemodes, a compile-time modding system, and almost all the modern stacker game mechanics I saw fit.
> 
> Maintaining high Rust code quality, especially in the [game logic](<https://crates.io/crates/falling-tetromino-engine>), was also important.


### Where's the config file? Will it clutter my system?

> <details>
> <summary>
> 
> The application will **not** store anything by default; 'Keep save file' needs to be opted in.
>
> </summary>
> 
> The exact location of the config file is visible in the *Advanced settings* TUI menu.
> The location based on the `dirs::config_dir()` implemented, e.g. `C:/User/myuser/AppData/Roaming/.tetro-tui_v1.0_savefile.json` or `/home/myuser/.config/.tetro-tui_v1.0_savefile.json`),
> - Otherwise directory of execution.
> 
> Savefile size grows primarily with number of saved replays (for which custom input compression *is* used however).
> The *Scores and Replays* menu can be used to select and delete entries or their replay (`[Del]` or `[Alt+Del]`, respectively).
>
> </details>


### *Experienced Stackers:* Why do timing-settings (DAS/ARR/SDF etc.) not apply for me?

> <details>
> <summary>
> 
> *TL;DR* use a terminal like [kitty](<https://sw.kovidgoyal.net/kitty/>) or [Alacritty](<https://alacritty.org/>) (or [->other](https://docs.rs/crossterm/latest/crossterm/event/struct.PushKeyboardEnhancementFlags.html)) for 'true'/smoother handling in the terminal.
> Otherwise timings might solely depend on how quickly your terminal sends key-repetition events.
> 
> </summary>
> 
> The real problem lies in how terminals normally send "key pressed" signals, but no "key released again" signals.
> This makes it impossible to implement mechanics like: "If `[←]` is pressed, move left repeatedly *until key is released again*".
> Precisely this issue is fixed with 'kitty protocol' / ['progressive enhancement'](<https://sw.kovidgoyal.net/kitty/keyboard-protocol/#progressive-enhancement>) / 'enhanced keyboard events'.
> 
> Some Windows terminals support it but are not auto-detected, if so try the Override toggle in the Advanced Settings menu.
> 
> Note that a similar technicality affects the recognition of `[Shift]`,`[Alt]`,... key presses as separate keys.
> On unenhanced terminals, those keys do not cause signals by themselves, but only in combination with a nonspecial-key presses (e.g. `[Ctrl+C]`). 
>
> </details>


### *Experienced Stackers:* How 'polished' are the mechanics?

> <details>
> <summary>
> 
> Copy of the feature list from the [Falling Tetromino Engine](<https://crates.io/crates/falling-tetromino-engine>) powering the game logic:
>
> </summary>
> 
> The engine aims to compete on the order of modern tetromino stackers;
> It incorporates many features found in such games.
> Experienced players may be familiar with most of the following mechanics:
> - **Variable gravity/fall delay** (frame-agnostic); '20G' (= 0s fall delay),
> - Simple but flexible programming of **custom fall and lock delay progressions** (`DelayParameters`),
> - (Arbitrary) **piece preview**,
> - **Pre-spawn actions** toggle ('Initial Hold/Rotation System'),
> - **Rotation systems**: 'Ocular' (engine-specific, playtested), 'ClassicL', 'ClassicR', 'Super',
> - **Tetromino generators**: 'Uniform', 'Stock' (generalized Bag), 'Recency' (history), 'Balancerelative',
> - **Spawn delay** (ARE),
> - **Delayed auto-shift** (DAS),
> - **Auto-repeat rate** (ARR),
> - **Soft drop factor** (SDF),
> - **Lenient lock delay reset** toggle (reset lock delay even if rotate/move fails),
> - **Ensure move delay less than lock delay** toggle (DAS/ARR automatically shortened when lock delay is very low),
> - **Lock-reset-cap factor** (~maximum time before lock delay cannot be reset),
> - **Line clear duration** (LCD),
> - Custom **win/loss conditions based on stats**: time, pieces, lines, score,
> - **Hold** piece,
> - Higher **score** for larger lineclears and spins ('allspin')
> - Game **reproducibility** (PRNG),
> - Available player actions: MoveLeft, MoveRight; RotateLeft, RotateRight, Rotate180; DropSoft, DropHard, TeleDown ('Sonic drop'), TeleLeft, TeleRight, HoldPiece.
> 
> </details>


### *Experienced Stackers:* In which ways is it *unlike* familiar stacker games?

> <details>
> <summary>
> 
> This project took its liberties to adapt/experiment with certain aspects of the game, although it should still feel as familiar as it can:
> 
> </summary>
> 
> - Default controls set to **WASD + Arrow keys**.
> - Use of the symmetrical and flexible **Ocular Rotation** System as default (instead of the arguably quirky industry standard).
> - Default **Recency (History) randomizer** (instead of 'overdeterministic' 7-Bag).
> - **Scoring** system is custom and kept simple.
>   - "1pt for simple line clear, increasing score incentivizing higher clears, spins, perfects and combos."
>   - 'Allspin' (instead of preoccupation with 'T-spins'), but no 'minis' (TBD).
>   - Combos, but no 'back-to-back'.
>   - ...Exact formula: `score_bonus = if is_perfect_clear{ 4 }else{ 1 } * if is_spin{ 2 }else{ 1 } * lineclears * 2 - 1 + (combo - 1)`
> - Additional controls for Teleport Down (a.k.a. 'Sonic Drop') / Left / Right.
> - Different lock reset / lock-down cutoff: 'max time = 10⋅current lock delay' (instead of 'max 15 moves with current lock delay').
> - Speed/Gravity/Fall curve practically the same but technically slightly adapted.
> 
> </details>


### *Experienced Stackers:* What's the "Ocular Rotation System"?

> <details>
> <summary>
> 
> A 'better' implementation of tetromino rotation, based off visual intuition and symmetry:
> 
> </summary>
>
> The Ocular Rotation System affords:
> - Rotation based on 'where it looks like the piece should be able to go'.
> - Symmetric (mirrored) situations should lead to symmetric (mirrored) outcomes.
> - Tetrominos should not teleport up/down too much.
>
> Visual heatmap comparison of rotation systems:
> 
> !["super rotation system heatmap"](https://github.com/Strophox/tetro-tui/blob/d8de81636a9fe47ba2e1f222de5a43f174d292ce/assets/super-rotation_heatmap.png?raw=true)
> 
> !["ocular rotation system heatmap"](https://github.com/Strophox/tetro-tui/blob/d8de81636a9fe47ba2e1f222de5a43f174d292ce/assets/ocular-rotation_heatmap.png?raw=true)
> 
> </details>


### *CLI Enthusiasts:* How was the Terminal User Interface (TUI) programmed and why isn't it [Ratatui](<https://ratatui.rs/>)?

> The project started out simple and has been directly using the amazing [Crossterm](<https://crates.io/crates/crossterm>) since then.
> Crossterm handles all the placement of (colored) characters and reading inputs from the terminal. We implement custom diff'ing so I/O does not bottleneck smooth rendering. We find TUI should generally stay in its current, minimalistic form, although a rewrite with Ratatui might be considered. 


### How do I navigate the TUI? Can I see a table of all the controls?

Refer to the following tables for comprehensive controls:

> <details>
> <summary>General TUI menu controls:</summary>
> 
> | Keys | ≈Meaning |
> | -: | :- |
> | `↓`/`↑`, `j`/`k` | Navigate up/down |
> | `←`/`→`, `h`/`l` | Change value |
> | `Enter`, `e` | Select |
> | `Esc`, `q`, `Back`, | Go back |
> | `Del`, `d` | Delete/reset |
> | `1`/`2`/`3`... | Quickselect option (⇝'New game')|
> | `Alt`+? | Different value change' (⇝'New game'⇝['Combo','Savepoint','Custom'], ⇝'Gameplay settings'⇝'Tetromino generation') |
> | `Alt`+`Del`, `Alt`+`d` | Delete replay  (⇝'Scores and replays') |
> | `Ctrl`+`U` | (For experienced/impatient people) unlock all gamemodes (⇝'New game') |
> | `Ctrl`+`C` | Exit application (respects save preferences) |
> 
> </details>
> 
>
> <details>
> <summary>Default live Game controls:</summary>
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
> | `Ctrl`+`E` | Store seed (accessible in ⇝'New game'⇝'Custom') |
> | `Ctrl`+`S` | Store savepoint (accessible in ⇝'New game'⇝'Savepoint', ⇝(live) 'Game'⇝`Ctrl`+`L`) |
> | `Ctrl`+`L` | Load savepoint (Caution: overwrites live game) |
> | `Ctrl`+`Alt`+`B` | Toggle on/off visibility of tiles ('Blindfolded') |
> | `Ctrl`+`C` | Exit application (respects save preferences) |
> 
> </details>
> 
>
> <details>
> <summary>Replay Game controls:</summary>
> 
> | Key | Action |
> | -: | :- |
> | `Esc`, `q`, `Back` | Exit replay |
> | `Space` | Pause replay |
> | `↓`/`↑` , `j`/`k` | Speed up / Slow down replay by ±0.25x |
> | `Alt`+`↓`/`↑`, `Alt`+`j`/`k` | Speed up / Slow down replay by ±0.05x |
> | `-` | Reset replay speed to =1.0x |
> | `←`/`→` , `h`/`l` | Skip forward/backward 1s in time |
> | `1`/`2`/`3`... | Jump to 10%/20%/30%/... |
> | `.` | Skip forward one player input and pause |
> | `Alt`+`.` | Skip forward one game state change\* and pause (experimental, might not work properly for modded games) |
> | `Enter`, `e` | Start (live) Game from current replay state |
> | `Ctrl`+`E` | Store seed (accessible in ⇝'New game'⇝'Custom') |
> | `Ctrl`+`S` | Store savepoint (accessible in ⇝'New game'⇝'Savepoint', ⇝(live) 'Game'⇝`Ctrl`+`L`) |
> | `Ctrl`+`I` | Toggle Instant Interactive Input Intervention mode (experimental) |
> | `Ctrl`+`C` | Exit application (respects save preferences) |
> 
> </details>


## License

Licensed under MIT.


## Provenance

100% human-sourced spaghetti code

Color palettes used:
- [Gruvbox](<https://github.com/morhetz/gruvbox>), [Solarized](<https://ethanschoonover.com/solarized/>), [Terafox](<https://wezterm.org/colorschemes/t/index.html#terafox>), [Fahrenheit](<https://wezterm.org/colorschemes/f/index.html#fahrenheit>), [matrix](<https://wezterm.org/colorschemes/m/index.html#matrix>), [Sequoia Monochrome](<https://wezterm.org/colorschemes/s/index.html#sequoia-monochrome>).


## Acknowledgements

Special Thanks to:
- GrBtAce, KonSola5 and bennxt – for support early in development
- Dunspixel – for inspiration regarding ['O'-spins](<https://dunspixel.github.io/ospin-guide/chapter4.html#tetro-tui>)
- madkiwi – for advice regarding 4wide-6residual combo layouts
- ([Apostolos Kousoukos](<https://akouzoukos.com/>) for making [Apotris](<https://apotris.com/>)!)
- and RayZN and ˗ˋˏthe One and Onlyˎˊ˗ – for advice regarding the Tetro logo
