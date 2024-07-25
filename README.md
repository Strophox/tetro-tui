<div align="center"><img width="440" src="https://repository-images.githubusercontent.com/816034047/9eba09ef-d6da-4b4c-9884-630e7f87e102" /></div>


# Tetromino Game Engine + Playable Terminal Implementation

## Frontend: `tetrs_terminal`

### How to run
*Pre-compiled:*
- Download a release for your platform if available and run the application.

*Compiling yourself:*
- Have [Rust](https://www.rust-lang.org/) installed.
- Download / `git clone` this repository.
- Navigate to `tetrs_terminal/` and `cargo run`.

> [!NOTE]
> Use a terminal like [kitty](<https://sw.kovidgoyal.net/kitty/>) for smoothest gameplay and visual experience.
> > *Explanation:* Terminals do not usually send "key released" signals, which is a problem for mechanics such as "press left to move left repeatedly **until key is released**". Crossterm automatically detects [kitty-protocol-compatible terminals](https://docs.rs/crossterm/latest/crossterm/event/struct.PushKeyboardEnhancementFlags.html) which solve issue. Otherwise DAS/ARR will be determined by Keyboard/OS/terminal emulator settings.)

### Features of the Terminal Game

<!--TODO: GIFs and screenshots.-->

Implemented features:
- **Gamemodes**:
  - Marathon, Sprint, Ultra, Master, Endless.
  - Puzzle Mode: Find all perfect clears through some piece acrobatics (\*Requires and demonstrates the Ocular Rotation System).
  - Custom Mode: level start, level increment, limit *(Time, Score, Pieces, Lines, Level; None)*.
- **Gameplay**:
  - Colored pieces (guideline).
  - Next piece preview (N=1).
  - Ghost piece.
  - Animations for: Hard drops, Line clears and Piece locking.
  - Current game stats: Level, Score, Lines, Time, Pieces generated.
  - For technical details see [Features of the Tetrs Engine](#features-of-the-tetrs-engine).
- **Settings**:
  - Configurable controls.
  - Adjustable render rate and toggleable FPS counter.
  - Rotation systems
    - Default: [*Ocular Rotation System*](#ocular-rotation-system)
    - *Classic* and *Super* also available.
- **Scoreboard** (stored to / loaded from *tetrs_terminal_scores.json* if available).

The game controls default to the following:
| Key | Action |
| -: | :-: |
| `A` | Rotate left |
| `D` | Rotate right |
| (not set) | Rotate around/180° |
| `←` | Move left |
| `→` | Move right |
| `↓` | Soft drop |
| `↑` | Hard drop |
| `Esc` | Pause game |
| `Ctrl`+`D` | Forfeit game |
| `Ctrl`+`C` | Exit program |

## Backend: `tetrs_engine`

### Usage of the Tetrs Engine
Adding `tetrs_engine` as a [dependency from git](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html) to your project:
```toml
[dependencies]
tetrs_engine = { git = "https://github.com/Strophox/tetrs.git" }
```

The tetrs engine shifts the responsibility of detecting player input, and choosing the time to update, to the user of the engine.
The key interactions with the engine look the following:
```rust
// Starting a game:
let game = tetrs_engine::Game::with_gamemode(gamemode, time_started);

// Updating the game with a new button state at a point in time:
game.update(Some(new_button_state), update_time);
// Updating the game with *no* change in button state (since the last):
game.update(None, update_time_2);

// Retrieving the game state (to render the board, active piece, next pieces, etc.):
let GameState { board, .. } = game.state();
```

For more info see crate documentation (`cargo doc --open`).

### Features of the Tetrs Engine
TODO: Reformulate all this.

The library at the time presents a moderately feature-rich abstraction over a singleplayer game. The goal was to strike the balance between interesting, useful and ergonomic game mechanics, while being as simple and efficient as possible. Luckily Rust being a safe and efficient language with high-level features (abstract datatypes, iterators) proved very apt for this.

**Game Configuration Features**
- *Gamemodes: Marathon, Sprint, Ultra, Master, CUSTOM (start lvl, inc lvl, limit, optimize) todo!("table here")
- *Rotation Systems: Ocular, Classic, Super
- *Tetromino Generators
- Piece Preview (N=1)
- Delayed Auto Shift (DAS) 200ms
- Auto Repeat Rate (ARR) 50ms
- Soft Drop Factor (SDF) 15.0
- Drop delay 1s@1->0.833ms@19
- Hard Drop Delay 0.1ms
- *Locking: ground_time_max 2.25s and lock curve 500ms@19->150ms@30
- Line Clear Delay 
- Appearance Delay (ARE)

**Game State Features**
- Abstract game time 
- Finished state/ Game Over: Lock out, Block out
- Event queue
- Buttons pressed state
- Board state 
- Active piece (tetromino, orientation, pos) including locking data
- Next Pieces
- Pieces played so far
- Lines cleared
- *Level and speed curve
- *Scoring and combo (b2b)
**Feedback Events**
- Piece locked: active piece
- line clears: lines and line clear delay
- hard drop: active piece before after
- Accolade: score bonus, shape, spin, lines, perfect, combo, opportunity
- Debug: String.

### Tetrs Engine: Highlights
TODO

#### Ocular Rotation System
TODO

## Author Notes
This project allowed me to have first proper learning experience with programming a larger Rust project, an interactive game (in the console), and the intricacies of the Game mechanics themselves (see [Features of the Tetrs Engine](#features-of-the-tetrs-engine)).

On the Rust side of things I learned about;
- Some [coding](https://docs.kernel.org/rust/coding-guidelines.html) [style](https://doc.rust-lang.org/nightly/style-guide/) [guidelines](https://github.com/rust-lang/rust-analyzer/blob/master/docs/dev/style.md#getters--setters) & `cargo fmt` (~`#[rustfmt::skip]`~),
- "[How to order Rust code](https://deterministic.space/how-to-order-rust-code.html)",
- introduction to [writing](https://doc.rust-lang.org/book/ch14-02-publishing-to-crates-io.html) [documentation](https://rust-lang.github.io/api-guidelines/documentation.html) (and the fact they can [contain tested examples](https://blog.guillaume-gomez.fr/articles/2020-03-12+Guide+on+how+to+write+documentation+for+a+Rust+crate#Hiding-lines)) & `cargo doc`,
- the [`std` traits](https://rust-lang.github.io/api-guidelines/interoperability.html),
- using [serde](https://serde.rs/derive.html) a little for a hacky way to [save some structured data locally](https://stackoverflow.com/questions/62771576/how-do-i-save-structured-data-to-file),
- [conditionally derive](https://stackoverflow.com/questions/42046327/conditionally-derive-based-on-feature-flag) feature flags & `cargo check --features serde`,
- [clap](https://docs.rs/clap/latest/clap/) to parse simple command line arguments & `cargo run -- --fps=60`,
- [formatting](https://docs.rs/chrono/latest/chrono/struct.DateTime.html#method.format) the time with [chrono](https://rust-lang-nursery.github.io/rust-cookbook/datetime/parse.html#display-formatted-date-and-time) my favourite way,
- the `format!` macro (which I discovered is the analogue to Python's f-strings my beloved),
- using [Crossterm](https://crates.io/crates/crossterm) for the inputs (instead of something like [device_query](https://crates.io/crates/device_query) - also I did not end up using [ratatui](https://crates.io/crates/ratatui/) :c Someone will have to write a frontend with that)
- the [annoyances](https://sw.kovidgoyal.net/kitty/keyboard-protocol/#progressive-enhancement) of terminal emulators,
- the handy drop-in [`BufWriter`](https://doc.rust-lang.org/std/io/struct.BufWriter.html) wrapper to diminish flickering,
- more practice with Rust's [module system](https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html),
- multithreading with [`std::sync::mpsc`](https://doc.rust-lang.org/std/sync/mpsc/)
- [cargo workspaces](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html) to fully separate frontend and backend,
- [cargo git dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#specifying-dependencies-from-git-repositories) so other people *could* reuse the backend,
- and finally, [cross-compilation](https://blog.logrocket.com/guide-cross-compilation-rust/#how-rust-represents-platforms) for releases.

Gamedev-wise I learned about the [modern](https://gafferongames.com/post/fix_your_timestep/) [game](http://gameprogrammingpatterns.com/game-loop.html) [loop](https://dewitters.com/dewitters-gameloop/) and finding the proper abstraction for `Game::update` (allow arbitrary-time user input, make updates decoupled from framerate). I also spent time looking at the menu navigation of [Noita](https://noitagame.com/) to help me come up with my own.

<sup>~~Lastly, I also found that there already *are*, like, a billion other [`tetrs`](https://github.com/search?q=%22tetrs%22&type=repositories)'s on GitHub, oops.~~</sup>

*„Piecement Places.“* - [CTWC 2016](https://www.youtube.com/watch?v=RlnlDKznIaw&t=121).
