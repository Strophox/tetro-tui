!["tetro-tui logo"](https://github.com/Strophox/tetro-tui/blob/a78483f9ba145798201f83ad2e4dc760ba918916/assets/tetro_logo_glow.png?raw=true)


# Tetro TUI - Cross-platform Terminal Game

[![Crates.io](https://img.shields.io/crates/v/tetro-tui.svg)](https://crates.io/crates/tetro-tui)
[![License](https://img.shields.io/crates/l/tetro-tui)](https://github.com/Strophox/tetro-tui#license)
<!--[![Documentation](https://docs.rs/tetro-tui/badge.svg)](https://docs.rs/tetro-tui)-->

A cross-platform terminal game where tetrominos fall and stack.

!["tetro-tui demonstration GIF"](https://github.com/Strophox/tetro-tui/blob/ec952782218e360e38efb945c849cfe69c3f00c3/assets/tetro-tui_demo-1.0.0.gif?raw=true)


## Ways to Run

### Download

1. [Download a release](<https://github.com/Strophox/tetro-tui/releases>) for your platform (windows, linux/unix/macos) if available.
2. Open your favorite terminal (preferably [Kitty](<https://sw.kovidgoyal.net/kitty/>) or [Alacritty](<https://alacritty.org/>)).
3. Run the application (e.g. `./tetro-tui` or `tetro-tui.exe`).


### Compile from source

1. Ensure [Rust](<https://doc.rust-lang.org/book/ch01-01-installation.html>) installed.
2. `git clone https://github.com/Strophox/tetro-tui` or manually download the source code.
3. Go inside `tetro-tui/` and go `cargo run`.


### Install via cargo

Tetro TUI is available on [crates.io](<https://crates.io/crates/tetro-tui>). It can be installed via [cargo](<https://doc.rust-lang.org/cargo/>) like so:
```
cargo install tetro-tui
```
Then you can run the game with `tetro-tui`.


## FAQ


### How does the game work?

> *Tetro* is about [tetromino](<https://en.wikipedia.org/wiki/Tetromino>) pieces falling from the sky and stacking on a rectangular playing field.
> 
> Whenever a line is filled up horizontally, it clears away, and the rest of what you 'stacked' moves down.
> This way a skilled player can keep playing without Blocking Out the entire board.


### How good is customization?

> For what started as a small project, solid:
> - **Graphics:** Unicode/ASCII/Electronika, 10 default color palettes, FPS, toggle effects...
> - **Keybinds:** to your heart's desire.
> - **Gameplay/Handling:** Rotation system, tetromino generator, preview count; DAS, ARR, SDF, LDC, ARE (timings), IRS/IHS
> - **Gamemode selection:** 40-Lines, Marathon, Time Trial, Master; Puzzle, Cheese, Combo; Custom mode (choose initial gravity, gravity progress on/off, custom/no goal, *commandline options:* start board or seed).


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
> The exact location of the config file is visible in the *Settings* TUI menu:
> - Location based on `dirs::config_dir()` (e.g. `C:/User/username/AppData/Roaming/.tetro-tui_1.0.0_savefile.json`, `/home/username/.config/.tetro-tui_..`),
> - Otherwise directory of execution.
> 
> Savefile size may grow primarily due to saved replays (though good care has been taken to compress those well). You can choose past games to delete in the *Scores and Replays* menu.


### *Experienced Stackers:* Why do timing-settings (DAS/ARR/SDF etc.) not always apply?

> *TL;DR* use a terminal like [kitty](<https://sw.kovidgoyal.net/kitty/>) (or [some other](https://docs.rs/crossterm/latest/crossterm/event/struct.PushKeyboardEnhancementFlags.html)) for 'true' (smooth) gameplay experience.
> In all other cases some timing configurations depend on how your terminal/keyboard/OS simulates key-repetitions.
> 
> The problem lies in how terminals only send 'key-pressed-once' signals, but none 'key-released-again'.
> This makes it impossible to implement mechanics like: "If `[←]` is pressed, move left repeatedly *until key is released again*".
> Precisely this issue is fixed with 'kitty protocol' / ['progressive enhancement'](<https://sw.kovidgoyal.net/kitty/keyboard-protocol/#progressive-enhancement>) / 'enhanced keyboard events'.


### *Experienced Stackers:* How 'polished' are the precision mechanics?

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
> - Variable gravity/fall delay, possibly in-between 'frames', '20G' (fall delay = 0s),
> - Simple but flexible programming of custom fall and lock delay progressions (`DelayParameters`),
> - (Arbitrary) piece preview,
> - Pre-spawn action toggle ('Initial Hold/Rotation System'),
> - Rotation systems: 'Ocular' (engine-specific, playtested), 'Classic', 'Super',
> - Tetromino generators: 'Uniform', 'Stock' (generalized Bag), 'Recency' (history), 'Balancerelative',
> - Spawn delay (ARE),
> - Delayed auto-shift (DAS),
> - Auto-repeat rate (ARR),
> - Soft drop factor (SDF),
> - Lenient-lock-delay-reset toggle (reset lock delay even if rotation fails),
> - Lock-reset-cap factor (~maximum time before lock delay cannot be reset),
> - Line clear delay (LCD),
> - Custom win/loss conditions based on stats: time, pieces, lines, score,
> - Hold piece,
> - Higher score for higher lineclears and spins ('allspin')
> - Game reproducibility (PRNG),
> - Available player actions: MoveLeft, MoveRight; RotateLeft, RotateRight, RotateAround (180°); DropSoft, DropHard, TeleDown ('Sonic drop'), TeleLeft, TeleRight, HoldPiece.
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
> - Custom **Ocular Rotation** System, instead of the 'odd' industry standard.
> - Default controls set to **WASD + Arrow**.
> - **Recency/History generator** instead of 'overdeterministic' 7-bag.
> - **Scoring** system is different, more **simplified**.
>   - 'Allspin' (no 'minis') instead of preoccupation with 'T-spins'.
>   - Combos, but no back-to-back.
>   - Exact formula is: `score_bonus = if is_perfect_clear{ 4 }else{ 1 } * if is_spin{ 2 }else{ 1 } * lineclears * 2 - 1 + (combo - 1)`
> - Additional controls for Teleport Down (a.k.a. 'Sonic Drop') / Left / Right.
> - Different lock reset implementation ('max 15 moves' vs. 'max 10⋅current lock delay')
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
> | Keys | ~Meaning |
> | -: | :- |
> | `↓` `↑` / `j` `k` | Navigate up/down |
> | `←` `→` / `h` `l` | Change value |
> | `Enter`/`e` | Select |
> | `Esc`/`q`, `Back`/`b` | Go back |
> | `Del`/`d` | Delete/reset |
> | `1` `2` `3`... | Quickselect option 1,2,3 ... (→new game menu) |
> | `Shift`+... | 'Accelerate certain controls' |
> | `Shift`+`Del`/`d` | Delete replay  (→scores&replays menu) |
> | `Ctrl`+`C` | Abort application |
> 
> </details>
> 
>
> <details>
> <summary>Default game controls:</summary>
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
> | `Ctrl`+`E` | Store seed (→custom mode) |
> | `Ctrl`+`S` | Store savepoint (→new game menu) |
> | `Ctrl`+`Shift`+`B` | Toggle on/off visibility of tiles ('Blindfolded' mode) |
> | `Ctrl`+`C` | Abort application |
> 
> </details>
> 
>
> <details>
> <summary>Replay controls:</summary>
> 
> | Key | Action |
> | -: | :- |
> | `Esc`/`q`, `Back`/`b` | Stop replay |
> | `Space` | Pause replay |
> | `↓` `↑` / `j` `k` | Speed up / Slow down replay by 0.25 |
> | `Shift`+`↓` `↑` / `Shift`+`j` `k` | Speed up / Slow down replay by 0.05 |
> | `←` `→` / `h` `l` | Skip forward/backward 1s in time |
> | `Shift`+`→` / `Shift`+`l` | Skip forward one user input and pause |
> | `Enter`/`e` | Enter playable game from replay position |
> | `Ctrl`+`E` | Store seed (→custom mode) |
> | `Ctrl`+`S` | Store savepoint (→new game menu) |
> | `Ctrl`+`C` | Abort application |
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
- GrBtAce, KonSola5 and bennxt *for support early in development*,
- Dunspixel *for inspiration regarding ['O'-spin](<https://dunspixel.github.io/ospin-guide/chapter4.html#tetro-tui>)*,
- madkiwi *for advice regarding 4wide-6res layouts*,
- and RayZN *for advice regarding the Tetro logo*.
