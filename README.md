!["tetro-tui logo"](https://github.com/Strophox/tetro-tui/blob/a78483f9ba145798201f83ad2e4dc760ba918916/assets/tetro_logo_glow.png?raw=true)


# Tetro TUI - Cross-platform Terminal Game

[![Crates.io](https://img.shields.io/crates/v/falling-tetromino-engine.svg)](https://crates.io/crates/falling-tetromino-engine)
[![Documentation](https://docs.rs/falling-tetromino-engine/badge.svg)](https://docs.rs/falling-tetromino-engine)
[![License](https://img.shields.io/crates/l/falling-tetromino-engine)](https://github.com/Strophox/falling-tetromino-engine#license)

A cross-platform terminal game where tetrominos fall and stack.


!["tetro-tui demonstration GIF"](./assets/tetro-tui_demo.gif)


## Ways to Run

### Release download

1. [Download a release](<https://github.com/Strophox/tetro-tui/releases>) for your platform (windows, linux/unix/macos) if available.
2. Open your favorite terminal (preferably [Kitty](<https://sw.kovidgoyal.net/kitty/>) or [Alacritty](<https://alacritty.org/>)).
3. Run the application (e.g. `./tetro-tui` or `tetro-tui.exe`).


### Build from source

1. Ensure [Rust](<https://doc.rust-lang.org/book/ch01-01-installation.html>) installed.
2. `git clone https://github.com/Strophox/tetro-tui` or manually download repository.
3. Do `cargo run` inside `tetro-tui/`.


### Cargo install

Install using cargo:
```
cargo install tetro-tui
```
Then run:
```
tetro-tui
```


## FAQ


### Which Gamemodes are available?

- **Basic modes:** 40-Lines, Marathon (150 lines), Time Trial (3 min.), Master (300 lines).
- **Special modes:** Puzzle (24 stages), Cheese, Combo.
- **Custom mode:** customize initial gravity, toggle gravity progression, custom goal (incl. 'no limit') (*using commandline options:* board, seed).


### How good is customization?

Solid:
- **Graphics:** Unicode/ASCII/Electronika, 10 default color palettes, FPS, toggle effects...
- **Keybinds:** to your heart's desire.
- **Gameplay/Handling:** Rotation system, tetromino generator, preview count; DAS, ARR, SDF, LDC, ARE (timings), IRS/IHS
- **Comprehensive game replay:** speed, time skipping/jumping.


### What's the 'purpose' of this project?

This is a passion project. The addition of the many features stem from personal motivation to make them available, while maintaining high code quality (especially in the game logic).

The result is (hopefully!) solid customizability, including editable json savefiles, deterministic replays, nontrivial gamemodes, a compile-time modding system, and as many modern stacker mechanics as I could fit.


### Where's the config file? Will it bloat or clutter my system?

The application will not store anything UNLESS 'Keep save file' is opted in.

The exact location of the config file is visible in the *Settings* TUI menu:
- Location based on `dirs::config_dir()` (e.g. `C:/User/username/AppData/Roaming/.tetro-tui_1.0.0_savefile.json`, `/home/username/.config/.tetro-tui_..`),
- Otherwise directory of execution.

Savefile size may grow primarily due to saved replays (though good care has been taken to compress those well). You can choose past games to delete in the *Scores and Replays* menu.


### *Experienced Stackers:* Why does custom DAS/ARR/SDF etc. not always work?

TL;DR use a terminal like [kitty](<https://sw.kovidgoyal.net/kitty/>) (or [some other](https://docs.rs/crossterm/latest/crossterm/event/struct.PushKeyboardEnhancementFlags.html)) for true/smooth gameplay experience.

Otherwise certain timings depend on how your terminal/keyboard/OS settings for key-repetitions.

The problem lies in how terminals only send 'key-pressed-once' signals, but none 'key-released-again'. This makes it impossible to implement mechanics like: "If `[←]` is pressed, move left repeatedly *until key is released again*".

Precisely this issue is fixed with 'kitty protocol' / ['progressive enhancement'](<https://sw.kovidgoyal.net/kitty/keyboard-protocol/#progressive-enhancement>) / 'enhanced keyboard events'.


### *Experienced Stackers:* How does depth of mechanics/configuration compare to other games?

Quote from the [Falling Tetromino Engine](<https://crates.io/crates/falling-tetromino-engine>) powering the actual game logic:

<details>

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

</details>


### *Experienced Stackers:* What's the "Ocular Rotation System"?

A 'better' implementation of tetromino rotation.

It is based off visual intuition and symmetry.

<details>
<summary>Visual heatmap comparison of rotation systems.</summary>

!["super rotation system heatmap"](./assets/super-rotation_heatmap.png.png)

!["ocular rotation system heatmap"](./assets/ocular-rotation_heatmap.png.png)

</details>


### How was the TUI programmed?

This basic but usable TUI was programmed using raw [Crossterm](<https://crates.io/crates/crossterm>).
Care has been taken to implement diff'ing - I/O operations should not bottleneck smooth rendering of gameplay.


### How do I navigate the TUI?

The TUI menu controls are generally:

<details>

| Keys | ~Meaning |
| :-: | :- |
| `↓`,`↑` / `j`,`k` | Navigate |
| `←`,`→` / `h`,`l` | Change value |
| `Enter`/`e` | Select |
| `Esc`/`q`, `Back`/`b` | Return back |
| `Del`/`d` | Delete/reset |
| `Ctrl`+`C` | Abort application |
| `Shift`+... | 'Accelerate' certain controls |
| `1`,`2`,`3`... | Jump to (→replay, new game menu) |

</details>

The Game controls generally default to:

<details>

| Key | Action |
| :-: | :- |
| `←` | Move left |
| `→` | Move right |
| `A` | Rotate left (CCW) |
| `D` | Rotate right (CW) |
| `S` | Rotate around (180°) |
| `↓` | Soft drop |
| `↑` | Hard drop |
| `W` | Teleport down |
| `Q` | Teleport left |
| `E` | Teleport right |
| `Esc` | Pause game |
| `Ctrl`+`D` | Forfeit game |
| `Ctrl`+`C` | Abort application |
| `Ctrl`+`S` | Store savepoint |
| `Ctrl`+`E` | Store seed (→custom mode) |
| `Ctrl`+`B` | Store board (→custom mode) |

</details>


## License

Licensed under MIT.


## Provenance

100% human-sourced spaghetti code

Color palettes:
- [Gruvbox](<https://github.com/morhetz/gruvbox>), [Solarized](<https://ethanschoonover.com/solarized/>), [Terafox](<https://wezterm.org/colorschemes/t/index.html#terafox>), [Fahrenheit](<https://wezterm.org/colorschemes/f/index.html#fahrenheit>), [matrix](<https://wezterm.org/colorschemes/m/index.html#matrix>), [Sequoia Monochrome](<https://wezterm.org/colorschemes/s/index.html#sequoia-monochrome>).


## Acknowledgements

Special thanks to:
- GrBtAce, KonSola5 and bennxt for support early in development;
- Dunspixel for the O-spin inspiration,
- madkiwi for advice regarding 4wide 6res layouts,
- and RayZN for advice regarding the Tetro logo.
