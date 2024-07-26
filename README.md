<div align="center"><img width="440" src="Gallery/tetrs_logo.png" /></div>


# Tetromino Game Engine + Playable Terminal Application

This repository hosts:
- `tetrs_terminal`, a simple, polished, efficient, cross-platform TUI implementation of the prototypical singleplayer game experience, and
- `tetrs_engine`, a tetromino game engine implementing an abstract interface with modern mechanics.


## How to run
> Pre-compiled.
> - Download a release for your platform if available and run the application.

> Compiling yourself.
> - Have [Rust](https://www.rust-lang.org/) installed.
> - Download / `git clone` this repository.
> - Navigate to `tetrs_terminal/` and `cargo run`.

> [!NOTE]
> Use a terminal like [kitty](<https://sw.kovidgoyal.net/kitty/>) (or any terminal with [support for progressive keyboard enhancement](https://docs.rs/crossterm/latest/crossterm/event/struct.PushKeyboardEnhancementFlags.html)) for smooth gameplay **controls** and/or visual experience. 
> 
> > <details>
> > 
> > <summary> Explanation. </summary>
> > 
> > Terminals do not usually send "key released" signals, which is a problem for mechanics such as "press left to move left repeatedly **until key is released**".
> > [Crossterm](https://docs.rs/crossterm/latest/crossterm/) automatically detects ['kitty-protocol'-compatible terminals]([https://docs.rs/crossterm/latest/crossterm/event/struct.PushKeyboardEnhancementFlags.html](https://sw.kovidgoyal.net/kitty/keyboard-protocol/#progressive-enhancement)) where this issue is solved.
> > Otherwise DAS/ARR will be determined by Keyboard/OS/terminal emulator settings.
> > *(This also affects Soft Drop, which with kitty can be held with the piece hitting ground without immediately locking piece.)*
> > 
> > </details>


## Gallery

<!--TODO: GIFs and screenshots.-->


## Features of the Application

**Gamemodes**
- Marathon, Sprint, Ultra, Master, Endless.
- Puzzle Mode: Find all perfect clears through some [*Ocular Rotation System*](#ocular-rotation-system) piece acrobatics (one retry per puzzle stage).
- Custom Mode: level start, level increment, limit *(Time, Score, Pieces, Lines, Level; None)*.

**Gameplay**
- Familiar game experience with moving, rotating, hard- and softdropping *tetrominos*.
- Colored pieces (guideline).
- Next piece preview (N=1).
- Ghost piece.
- Animations for: Hard drops, Line clears and Piece locking.
- Current game stats: Level, Score, Lines, Time, Pieces generated.
- For technical details see [Features of the Tetrs Engine](#features-of-the-tetrs-engine).
  
**Scoreboard**
- (stored to / loaded from local *tetrs_terminal_scores.json* if possible).
  
**Settings**
- Toggleable graphics (colored Unicode <-> oldschool, monochrome ASCII).
- Adjustable render rate and toggleable FPS counter.
- Rotation systems: *Ocular*, *Classic* and *Super*.
- Configurable controls.
  <details>
  
  <summary> Default Game Controls </summary>
  
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
  
   </details>


## Features of the Tetrs Engine

The frontend application is proof-of-concept;
Ultimately the tetrs engine tries to be modular and shifts the responsibility of detecting player input and chosen time of updates to the client.
Basic interaction with the engine could look like the following:

```rust
// Starting a game.
let game = tetrs_engine::Game::with_gamemode(gamemode, time_started);
// Application loop.
loop {
  // Updating the game with a new button state at a point in time.
  game.update(Some(new_button_state), update_time);
  // Updating the game with *no* change in button state (since the last).
  game.update(None, update_time_2);
  // Retrieving the game state (to render the board, active piece, next pieces, etc.).
  let GameState { board, .. } = game.state();
}
```

<details>

<summary> Using the engine in your rust project </summary>

Adding `tetrs_engine` as a [dependency from git](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html) to your project:
```toml
[dependencies]
tetrs_engine = { git = "https://github.com/Strophox/tetrs.git" }
```

</details>

<details>

<summary> Game Configuration Aspects </summary>

- Gamemodes: Marathon, Sprint, Ultra, Master; Custom, given a playing limit, start lvl, whether to increment level.
- Rotation Systems: *Ocular Rotation System*, *Classic Rotation System*, *Super Rotation System*.
- Tetromino Generators: *Bag*, *Recency-based*, *Uniformly random*.
- Piece Preview (default N = 1)
- Delayed Auto Shift (default DAS = 200ms)
- Auto Repeat Rate (default ARR = 50ms)
- Soft Drop Factor (default SDF = 15.0)
- Hard drop delay (default at 0.1ms)
- Line clear delay (default at 200ms)
- Appearance Delay (default ARE = 100ms)

Currently, drop delay and lock delay\* *(\*But not total ground time)* are a function of the current level:
- Drop delay (1000ms at lvl 1 to 0.833ms ("20G") at lvl 19).
- 'Timer' variant of Extended Placement Lockdown (step reset); The piece tries to lock every 500ms at lvl 19 to every 150ms at lvl 30, and any given piece may only touch ground for 2250ms in total. See also [Piece Locking](#piece-locking).

</details>

<details>

<summary> Game State Aspects </summary>

- Time: Game time is held abstract as "time elapsed since game started" and is not directly tied to real-world timestamps.
- Game finish: The game knows if it finished, and if session was won or lost. Game Over scenarios are:
  - Block out: newly piece spawn location is occupied.
  - Lock out: a piece was completely locked above the skyline (row 21 and above).
- Event queue: All game events are kept in an internal queue that is stepped through, up to the provided timestamp of a `Game::update` call.
- Buttons pressed state: The game keeps an abstract state of which buttons are currently pressed.
- Board state: Yes.
- Active piece: The active piece is stored as a (tetromino, orientation, position) tuple plus some locking data.
- Next Pieces: Are kept in a queue and can be viewed.
- Pieces played so far: Kept as a stat.
- Lines cleared: <sup>yeah</sup>
- (Speed) Level: Increases every 10 line clears and influences only drop/lock delay.
- Scoring: Line clears trigger a score bonus, which takes into account number of lines cleared, spins, combos, back-to-backs.
  <details>
  
  <summary>Scoring Details</summary>
  
  ```haskell
  score_bonus = 10
              * (lines ^ 2)
              * (if spin then 4 else 1)
              * (if perfect then 16 else 1)
              * combo
              * maximum [1, backToBack * backToBack]
    where lines = "number of lines cleared simultaneously"
          spin = "piece could not move up when locking occurred"
          perfect = "board is empty after line clear"
          combo = "number of consecutive pieces where line clear occurred"
          backToBack = maximum [1, "number of consecutive line clears where spin, perfect or quadruple line clear occurred"]
  ```
  A table of some bonuses is provided:
  | Score bonus | Action |
  | -: | :- |
  | +10 | Single |
  | +40 | Double |
  | +90 | Triple |
  | +160 | Quadruple |
  | +20 | Single (2.combo) |
  | +30 | Single (3.combo) |
  | +80 | Double (2.combo) |
  | +120 | Double (3.combo) |
  | +40 | ?-Spin Single |
  | +160 | ?-Spin Double |
  | +360 | ?-Spin Triple |

  </details>
  
</details>


<details>

<summary> Game Feedback Aspects </summary>

The game provides some useful feedback events upon every `update`, usually used to correctly implement visual effects:
- *Piece locked down*, *Lines cleared*, *Hard drop*, *Accolade* (score bonus info), *Message* (generic message, currently unused)

</details>

Also see documentation (`cargo doc --open`).


## Project Highlights

While the [2009 Tetris Guideline](https://tetris.wiki/Tetris_Guideline) served as good inspiration, I ended up having a lot of fun researching the variety of details of the game myself (thank you [Tetris Wiki](https://tetris.wiki/) and [HardDrop](https://harddrop.com/wiki)!) and also by asking people (thank you GrBtAce and KonSola5!).
The following detail various big and small concepts I tackled on my way to completing this game.


### Ocular Rotation System

> "[tetris has a great rotation system and is not flawed at all](https://www.youtube.com/watch?v=_qaEknA81Iw)"

— *[said no one ever, not even the creator of Tetris himself](https://youtu.be/6YhkkyXydNI?si=jbVwfNtfl5yFh9Gk&t=674).*

To be fair, donsidering the huge franchise and range of players coming from all sorts of niches and previous game version the *"Super Rotation System"* [gets its job done](https://www.youtube.com/watch?v=dgt1kWq2_7c)™.

But this doesn't change the fact that it was *the* mechanic I thought about revamping when thinking about starting this project:

- The system is **not** symmetric:
  - Symmetric pieces can look exactly the same in different rotation states, **[but have different behaviour](https://blog.battlefy.com/how-accidental-complexity-harms-the-tetris-community-93311461b2af)**.
  - Doing rotation, then Mirroring board and piece **≠** Mirroring board and piece, then Doing mirrored rotation.
- 'Piece elevators' exist (\*though I don't know how to disprove the existence of such in any given rotation system with kicks.)
- It's an [advanced system](https://harddrop.com/wiki/SRS) with things like different rotation points for different purposes, yet it re-uses the *exact same kicks* for 5 out of the 7 pieces, even though they have completely different symmetries.
- Not a hot take, but some rotations are just *weird* (to be chosen over other possibilities).

My criteria for a good rotation system thus would be:

1. Symmetric if the board and piece were mirrored.
2. Equal-looking states must have the same behaviour.
3. Don't overdo it with crazy kicks
4. But still allow fun things

The result of this was the *Ocular Rotation System*, which I made by... looking at each piece orientation and drawing the most visually sensible positions for a piece to land in after a rotation. By overlapping all the kicks that are test in sequential order, one gets a compact heatmap of where the piece will land, going from hottest color (bright yellow) to coldest (dark purple):

<details>

<summary> Ocular Rotation System Heatmap </summary>

![Ocular Rotation System Heatmap](Gallery/ocular-rotation-system_16px.png)

</details>

Now compare this to SRS and the difference is clear:

<details>

<summary> Super Rotation System Heatmap </summary>

![Super Rotation System Heatmap](Gallery/super-rotation-system_16px.png)

</details>

Although, one starts to see the vague rotational symmetries here.


### Tetromino Generation

Tetromino generation is at the heart of the game.

A trivial generator chooses tetrominos *uniformly at random*, and this actually already works decent.
But players tend to get frustrated when they don't get the pieces they need for a long time: there's even a name for not getting an `I`-piece for a long time: *Droughts*.

The modern *bag-based generation system* is standard and simply takes all 7 pieces once, gives them out in random order, repeat.
It's quite nice knowing an `I` piece will come after 12 pieces, every time. You can even start strategizing quite deterministically by counting where one bag ends and the next starts.

But to me it takes the fun out of the randomness, *somehow*; Maybe we can take uniform random generation and just tweak it a little so it doesn't have droughts anymore?

Presenting *recency-based generation system*: It remembers the last time it generated each piece and chooses the next piece randomly, but weighted by last seen so it more likely chooses pieces not seen in a while.

This preserves "possibly complete randomness" (*any* piece may be generated at any time), but *precisely* mitigating droughts.

One could make a point that it's even more handy than bag in *certain* ways: The player can have a mental image of the _order_ in which the next or next few pieces might appear. In bag, with each new refill the order within *is* completely random (although I'm aware this does not matter for games that have 6 piece preview and standard 7-bag).


### Piece Locking

The mechanic of locking a piece tends to be more complicated than it sounds.
My criteria for a good locking system are:

1. Keeps the player from stalling / forces the player to make a choice eventually.
2. Give the player enough flexibility to manipulate the piece even if it's on the ground.
3. Force the player to react _faster_ on higher levels, as speed is supposed to increase.
4. Implement all these limitations as naturally and/or simply as possible.

So I started researching and thinking about the locking system to use.

<details>

<summary> Piece Locking ... </summary>

*Classic lock down* is simple, but if one decreases the lock timer (3.) then it becomes insanely difficult for the player to actually have enough time to do adjustments (2.).

<details>

<summary> Classic Lock Down </summary>

> - If the piece touches a surface
>   - start a lock timer of 500ms (\**var with lvl*).
>   - record the lowest y coordinate the piece has reached.
> - If the lock timer runs out, lock the piece immediately as soon as it touches the next surface.
> - If the piece falls below the previously lowest recorded y coordinate, reset lock timer.

</details>

*Infinite lock down* mitigates the flexibility issue by saying, *"if the player manipulated his piece, give him some more time"*. 
It's very simple, but is disqualified due to breaking the make-a-decision-please-aspect of the game (1.) in the case where the player, well can't decide and keeps rotating his `T` piece forever.

<details>

<summary> Infinite Lock Down </summary>

> - If the piece touches a surface
>   - start a lock timer of 500ms (\**var with lvl*).
> - If the lock timer runs out, lock the piece immediately as soon as it touches the next surface.
> - If the piece moves/rotates (change in position), reset lock timer ('move reset').

</details>

The standard recommended by the guideline is therefore *extended placement lock down*.

<details>

<summary> Extended Placement Lock Down </summary>

> - If the piece touches a surface
>   - start a lock timer of 500ms (\**var with lvl*).
>   - start counting the number of moves/rotates the player makes.
>   - record the lowest y coordinate the piece has reached.
> - If the piece moves/rotates (change in position), reset lock timer ('move reset').
> - If the number of moves reaches 15, do not reset the lock timer anymore.
> - If the lock timer runs out, lock the piece immediately as soon as it touches the next surface.
> - If the piece falls below the previously lowest recorded y coordinate, reset counted number of moves.

*(\*This probably misses a few edge cases, but you get the gist.)*

</details>

Yeah.

It's pretty flexible (2.) yet forces a decision (1.), but I dislike the 'count to 15 moves' part of this lock down (also, after the 15 moves run out you can still manipulate the piece till lock down), it just seems so arbitrary (4.).

What if we limit the *total amount of time a piece may touch a surface* (1.) instead of number of moves/rotates (4.), though but at higher levels the piece *attempts* to lock down faster (3.), re-attempting later upon move/rotate; This still allows for plenty <sup>*\*technically arbitrarily many*</sup> piece manipulations (2.) while still fulfilling the other points :D

<details>

<summary> Timer Extended Placement Lock Down </summary>

*Let 'ground time' denote the amount of time a piece touches a surface*

> - If the piece touches a surface
>   - start a lock timer of 500ms (\**var with lvl*).
>   - start measuring the ground time.
>   - record the lowest y coordinate the piece has reached.
> - If the piece moves/rotates (change in position), reset lock timer ('move reset').
> - If the lock timer runs out *or* the ground time reaches 2.25s, lock the piece immediately as soon as it touches the next surface.
> - If the piece falls below the previously lowest recorded y coordinate, reset the ground time.

</details>

Although nice, it may *potentially* be abused by players keeping pieces in the air, only to occasionally touch down to reset the lock timer but hardly count any ground time (though this problem vanishes at high G anyway).

A small fix for this is to check the last time the piece touched the ground, and if it's less than 2×(drop delay) ago, act as if the piece had been touching ground all along. This way the piece is guaranteed to be counted as "continuously on ground" with upward kicks ≤ 2.

</details>

In the end, a timer-based extended placement lockdown (+ ground continuity fix) is what I used. I have yet to find a nicer system.


### Gamemodes

Initially a lot of architectural decisions were not clear; as such the question *what is the goal of this game?*
My findings were:
- There are several game stats one can keep track of, and
- Canonical / commonly found gamemodes can be approximated as a combination of `(stat which is limited so game can complete) + (stat which player aims to optimize)`.
Examples:
- *'Marathon':* limit Level to 20, try for highest score.
- *'Sprint'* / *'40 lines'*: limit lines to 40, try for lowest time.
- *'Ultra'* / *'Time Trial'*: limit time to 2-3min, try for highest score / most lines.
The real implementation additionally stores the (speed) level to start at, and whether clearing lines increments the level.
> [!NOTE]
> Given one stat, how do we know whether we want to maximize or minimize another arbitrary stat?
> I may be overlooking a simpler pattern, but it seems one can order all stats linearly, and given a stat to be fixed/limited, any other stat is maximized/minimized directly depending on whether it's further down/up the sequence:
> <details>
> 
> <summary>Gamemode Stat Relation Table</summary>
> 
> | name | finss | time  | piecs | lines | level | score |
> | ---- | ----- | ----- | ----- | ----- | ----- | ----- |
> |      |  fix  |  MAX  |       |       |       |
> |      |       |       |  MAX  |       |       |
> |      |  fix  |       |       |  MAX  |       |
> |      |  fix  |       |       |       |  MAX  |
> |      |  fix  |       |       |       |       |  MAX
> |      |  MIN  |  fix  |       |       |       |
> |      |       |  fix  |  MAX  |       |       |
> | *'Ultra*' |  |  fix  |       |  MAX  |       |
> |      |       |  fix  |       |       |  MAX  |
> |      |       |  fix  |       |       |       |  MAX
> |      |  MIN  |       |  fix  |       |       |
> |      |       |  MIN  |  fix  |       |       |
> |      |       |       |  fix  |  MAX  |       |
> |      |       |       |  fix  |       |  MAX  |
> |      |       |       |  fix  |       |       |  MAX
> |      |  MIN  |       |       |  fix  |       |
> | *'Sprint'* | |  MIN  |       |  fix  |       |
> |      |       |       |  MIN  |  fix  |       |
> |      |       |       |       |  fix  |  MAX  |
> |      |       |       |       |  fix  |       |  MAX
> |      |  MIN  |       |       |       |  fix  |
> |      |       |  MIN  |       |       |  fix  |
> |      |       |       |  MIN  |       |  fix  |
> |      |       |       |       |  MIN  |  fix  |
> | *'Marathon'* | |     |       |       |  fix  |  MAX
> |      |  MIN  |       |       |       |       |  fix
> |      |       |  MIN  |       |       |       |  fix
> |      |       |       |  MIN  |       |       |  fix
> |      |       |       |       |  MIN  |       |  fix
> |      |       |       |       |       |  MIN  |  fix
> 
> </details>


> So how does 'Puzzle Mode' work? - I can tell you how: with a pinch of state modeling jank and some not-so-secret internal state leakage via `Game::set_modifier`.

### Scoring

Coming up with a good score system is tough, and experience and playtesting helps, so the one I come up with probably sucks ("how many points should a 'perfect clear' receive?"). Even so, I went along and experimented, since I liked the idea of [rewarding all spins](https://harddrop.com/wiki/List_of_twists).

### Controls

Quick research on the 'best' or 'most ergonomic' game keybinds was [inconclusive](https://youtube.com/watch?v=6YhkkyXydNI&t=809). Upon sampling a few dozen on reddit posts it seems a 50/50 split on `←` `→` / `a` `d` or `z` `x` / `←` `→` for **move** / **rotate**. "Choose what feels best for you" some said - they're probably right. *(\*Even so, one should *not* hammer the spacebar for hard drops, the only button guideline suggests.)*


### Menu Navigation.

Modeling how the TUI goes between the menus was unclear initially. Luckily, I was able to look at how [Noita](https://noitagame.com/) did its menu navigation and saw that it needn't be a complicated system;
In essence, the menus form a graph (with menus as nodes and valid transitions as directed edges), and some menus allow one to _go back_ to where one came from last (or not).

<details>

<summary>
  Tetrs Menu Graph
</summary>

![tetrs menu graph](Gallery/tetrs_menu-graph.png)

</details>


### Miscellaneous Author Notes

This project allowed me to have first proper learning experience with programming a larger Rust project, an interactive game (in the console), and the intricacies of the Game mechanics themselves (see [Features of the Tetrs Engine](#features-of-the-tetrs-engine)).

Gamedev-wise I learned about the [modern](https://gafferongames.com/post/fix_your_timestep/) [game](http://gameprogrammingpatterns.com/game-loop.html) [loop](https://dewitters.com/dewitters-gameloop/) and finding the proper abstraction for `Game::update` (allow arbitrary-time user input, make updates decoupled from framerate).

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

All in all, Rust, known for its performance and safety, proved to be an very good choice for this project.

<sup>~~PS: I'm aware there are, like, a billion other [`tetrs`](https://github.com/search?q=%22tetrs%22&type=repositories)'s on GitHub, oops~~</sup>

*„Piecement Places.“* - [CTWC 2016](https://www.youtube.com/watch?v=RlnlDKznIaw&t=121).
