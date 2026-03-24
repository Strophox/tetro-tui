!["tetro-tui logo"](https://github.com/Strophox/tetro-tui/blob/a78483f9ba145798201f83ad2e4dc760ba918916/assets/tetro_logo_glow.png?raw=true)


# Tetro TUI - Cross-platform Terminal Game

[![Crates.io](https://img.shields.io/crates/v/tetro-tui.svg)](https://crates.io/crates/tetro-tui)
[![License](https://img.shields.io/crates/l/tetro-tui)](https://github.com/Strophox/tetro-tui#license)
<!--[![Documentation](https://docs.rs/tetro-tui/badge.svg)](https://docs.rs/tetro-tui)-->

A cross-platform terminal game where tetrominos fall and stack.

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

Tetro TUI is available on [crates.io](<https://crates.io/crates/tetro-tui>). It can be installed via [cargo](<https://doc.rust-lang.org/cargo/>) like so:
```
cargo install tetro-tui
```
Then you can run the game with `tetro-tui`.


<!-- TODO: Elaborate.
### AUR Package

https://aur.archlinux.org/packages/tetro-tui-bin
-->


## FAQ


### How does the game work?

> *Tetro* is about [tetromino](<https://en.wikipedia.org/wiki/Tetromino>) pieces falling from the sky and stacking on a rectangular playing field.
> 
> Whenever a line is filled up horizontally, it clears away, and the rest of what you 'stacked' moves down.
> This way a skilled player can keep playing without Blocking Out the entire board.


### How good is customization?

> For what started as a small project, solid:
> - **Graphics:** Unicode/ASCII/Electronika, a handful of default color palettes, FPS, toggle effects...
> - **Game Keybinds:** to your heart's desire.
> - **Gameplay/Handling:** Rotation system, tetromino generator, preview count; DAS, ARR, SDF, LDC, ARE (timings), IRS/IHS
> - **Gamemode selection:** Swift (40 lines), Classic ('marathon'), Master, Puzzle, Cheese-X, Combo-X; Custom (choose start gravity, toggle gravity progress, select goal, *advanced flags:* start board or seed).
>
> <details>
> <summary>
>
> Terminal customizations can carry over to game graphics, e.g. using [cool-retro-term](<https://github.com/Swordfish90/cool-retro-term>):
>
> </summary>
>
> !["tetro-tui running in cool-retro-term"](https://github.com/Strophox/tetro-tui/blob/011251ba7cbfaca03414c910c5483791f3fef737/assets/tetro-tui_cool-retro-term.jpg?raw=true)
>
> </details>


### What was the motivation behind this project?

> This is a passion project.
> The addition of the many features stem from personal motivation to make them available and make things enjoyable.
> 
> The result is (hopefully!) solid customizability; Editable json savefiles, compressed game replays, nontrivial gamemodes, a compile-time modding system, and almost as many modern stacker game mechanics as one could fit.
> 
> Maintaining high Rust code quality, especially in the [game logic](<https://crates.io/crates/falling-tetromino-engine>), was also important.


### Where's the config file? Will it bloat or clutter my system?

> The application will not store anything UNLESS 'Keep save file' is opted in.
> 
> The exact location of the config file is visible in the *Advanced settings* TUI menu:
> - Location based on `dirs::config_dir()` (e.g. `C:/User/username/AppData/Roaming/.tetro-tui_1.0_savefile.json` or `/home/username/.config/.tetro-tui_1.0_savefile.json`),
> - Otherwise directory of execution.
> 
> Savefile size may grow primarily due to saved replays (though good care has been taken to compress those well).
> The *Scores and Replays* menu can be used to delete past games or only their replay (`[Del]` or `[Alt+Del]` respectively).


### *Experienced Stackers:* Why do timing-settings (DAS/ARR/SDF etc.) not apply for me?

> *TL;DR* use a terminal like [kitty](<https://sw.kovidgoyal.net/kitty/>) or [Alacritty](<https://alacritty.org/>) (or [->other](https://docs.rs/crossterm/latest/crossterm/event/struct.PushKeyboardEnhancementFlags.html)) for 'true'/smoother handling in the terminal.
> Otherwise timings might solely depend on how quickly your terminal sends key-repetition events.
> 
> <details>
> <summary> Explanation. </summary>
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
> Quote from the [Falling Tetromino Engine](<https://crates.io/crates/falling-tetromino-engine>) powering the actual game logic:
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
> The project took its liberties to adapt/experiment with certain aspects of game mechanics (to try and improve it):
> 
> </summary>
> 
> - Use of the intuitive/symmetrical **Ocular Rotation** System, instead of the 'odd' industry standard.
> - Default controls set to **WASD + Arrow**.
> - **Recency/History generator** instead of 'overdeterministic' 7-bag.
> - **Scoring** system is different, more **simplified**.
>   - 'Allspin' (no 'minis') instead of preoccupation with 'T-spins'.
>   - Combos, but no back-to-back.
>   - Exact formula is: `score_bonus = if is_perfect_clear{ 4 }else{ 1 } * if is_spin{ 2 }else{ 1 } * lineclears * 2 - 1 + (combo - 1)`
> - Additional controls for Teleport Down (a.k.a. 'Sonic Drop') / Left / Right.
> - Different lock reset implementation ('max 15 moves' instead of 'max 10⋅current lock delay')
> - Speed/Gravity/Fall curve slightly adapted.
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


### *CLI Enthusiasts:* How was the Terminal User Interface (TUI) programmed?

> This basic but hopefully decent TUI was programmed directly using the amazing [Crossterm](<https://crates.io/crates/crossterm>).
> Crossterm handles all the placement of (colored) characters and reading inputs from the terminal. We implement custom diff'ing so I/O does not bottleneck smooth rendering.


### How do I navigate the TUI? Can I see a table of all the controls?

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
