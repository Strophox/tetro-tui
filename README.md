<div align="center"><img width="440" src="https://repository-images.githubusercontent.com/816034047/9eba09ef-d6da-4b4c-9884-630e7f87e102" /></div>

# Tetromino Game Engine + A Playable Terminal Implementation

## How to run `tetrs_terminal`
Pre-compiled:
- Download a release for your platform if available and run the application.

Compiling yourself:
- Have the [Rust](https://www.rust-lang.org/) compiler (and Cargo) installed.
- [Download](<https://github.com/Strophox/tetrs/archive/refs/heads/main.zip>) (or `git clone`) this repo.
- Navigate to `tetrs/` (or `tetrs_terminal/`) and compile with `cargo run`.
- (Relevant keys [`Esc` `Enter` `←` `→` `↑` `↓` `A` `D`] also shown inside the application)

Additional notes:
- Set the game framerate with `./tetrs_terminal --fps=60` (or `cargo run -- --fps=60`) (default is 30fps).
- Use a terminal like [kitty](<https://sw.kovidgoyal.net/kitty/>) for smoothest gameplay and visual experience. *Explanation:* Terminals do not usually send "key released" signals, which is a problem for mechanics such as "press left to move left repeatedly **until key is released**". We rely on [Crossterm](https://docs.rs/crossterm/latest/crossterm/event/struct.PushKeyboardEnhancementFlags.html) to automatically detect kitty-protocol-compatible terminals where this is not an issue (check page). In all other cases DAS/ARR is be determined by Keyboard/OS/terminal settings.)

## Usage of the `tetrs_engine`
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

# Features of the Frontend / Terminal Application
TODO: GIFs and screenshots.
- 

# Features of the Tetrs Engine
TODO: `all` the features here.


# Further Notes
This project allowed me to have my first 'proper' learning experiences with programming a larger Rust project, interactive game (in the console), and the intricacies of Tetris all at once.

On the Rust side of things I learned about
- some [coding](https://docs.kernel.org/rust/coding-guidelines.html) [style](https://doc.rust-lang.org/nightly/style-guide/) [guidelines](https://github.com/rust-lang/rust-analyzer/blob/master/docs/dev/style.md#getters--setters) & `cargo fmt` (~and `#[rustfmt::skip]`~),
- "[How to order Rust code](https://deterministic.space/how-to-order-rust-code.html)",
- introduction to [writing](https://doc.rust-lang.org/book/ch14-02-publishing-to-crates-io.html) [documentation](https://rust-lang.github.io/api-guidelines/documentation.html) (and the fact they can [contain tested examples](https://blog.guillaume-gomez.fr/articles/2020-03-12+Guide+on+how+to+write+documentation+for+a+Rust+crate#Hiding-lines)) & `cargo doc`,
- the [`std` traits](https://rust-lang.github.io/api-guidelines/interoperability.html),
- the `format!` macro (lovely analogue to Python's f-strings),
- using [Crossterm](https://crates.io/crates/crossterm) for the inputs (instead of something like [device_query](https://crates.io/crates/device_query) - also I did not end up using [ratatui](https://crates.io/crates/ratatui/) :c Someone will have to write a frontend with that)
- the [annoyances](https://sw.kovidgoyal.net/kitty/keyboard-protocol/#progressive-enhancement) of terminal emulators,
- the handy drop-in [`BufWriter`](https://doc.rust-lang.org/std/io/struct.BufWriter.html) wrapper to diminish flickering,
- [clap](https://docs.rs/clap/latest/clap/) to parse simple command line arguments,
- more practice with Rust's [module system](https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html),
- [cargo workspaces](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html) to fully separate frontend and backend,
- [cargo git dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#specifying-dependencies-from-git-repositories) so other people *could* reuse the backend,
- and finally [cross-compilation](https://blog.logrocket.com/guide-cross-compilation-rust/#how-rust-represents-platforms) for releases.

Gamedev-wise I learned about the [modern](https://gafferongames.com/post/fix_your_timestep/) [game](http://gameprogrammingpatterns.com/game-loop.html) [loop](https://dewitters.com/dewitters-gameloop/), and how to decouple it from render framerate to come up with the proper `Game::update` abstraction.

~I also found that there already *are*, like, a billion other [`tetrs`](https://github.com/search?q=%22tetrs%22&type=repositories)'s on GitHub - oops.~
