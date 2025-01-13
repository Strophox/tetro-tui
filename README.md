<div align="center"><img width="440" src="Gallery/tetrs_logo.png" /></div>


<div align="center" style="text-align: center; width: 100%">
<h1>Tetromino Game Engine + Terminal Application</h1>
</div>

*This repo hosts*
- `tetrs_engine`, a tetromino game engine implementing an API capable of handling some modern mechanics, and
- `tetrs_tui`, a simple but moderately polished Terminal User Interface for a typical cross-platform game experience.

<!--
---

**(Author's Note : Due to irl circumstances I cannot continue development right now - issues may be worked on at a later time)**

---
-->


## How to run

- [Download a release](https://github.com/Strophox/tetrs/releases) for your platform if available.
- Run the application in your favourite terminal

> Or compile it yourself:
> - Have [Rust](https://www.rust-lang.org/) (1.80.0+) installed.
> - Download / `git clone` this repository.
> - Navigate to `tetrs_tui/` and `cargo run`.

> [!IMPORTANT]
> Use a terminal like [kitty](<https://sw.kovidgoyal.net/kitty/>) (or any terminal with [support for progressive keyboard enhancement](https://docs.rs/crossterm/latest/crossterm/event/struct.PushKeyboardEnhancementFlags.html)) for smoother gameplay experience.
> **Note that otherwise DAS/ARR/Soft drop speed will be determined by Keyboard/OS/terminal emulator settings.** 
> 
> > <details>
> > 
> > <summary> Explanation. </summary>
> > 
> > Terminals do not usually send "key released" signals, which is a problem for mechanics such as "press left to move left repeatedly **until key is released**".
> > [Crossterm](https://docs.rs/crossterm/latest/crossterm/) automatically detects ['kitty-protocol'-compatible terminals]([https://docs.rs/crossterm/latest/crossterm/event/struct.PushKeyboardEnhancementFlags.html](https://sw.kovidgoyal.net/kitty/keyboard-protocol/#progressive-enhancement)) where this issue is solved, allowing for smooth, configurable gameplay controls.
> >
> > (\*This also affects holding Soft Drop locking pieces on ground instantly, as opposed to only upon press down -- for ergonomics this is explicitly mitigated by the 'No soft drop lock' configuration.)*
> > 
> > </details>


## Demo Gallery

**Classic game experience:**

![Unicode demo screenshot](Gallery/sample-unicode.png)


**Efficient rendering in consoles, configurable controls, etc.:**

![Unicode demo GIF](Gallery/sample-unicode.gif)


> [!TIP]
> Play **Puzzle Mode** with its 24 stages to test the custom [Ocular Rotation System](#ocular-rotation-system) (+ unlock the experimental **Descent Mode**).


**Various tileset + coloring combinations possible:**

<details>

<summary>ASCII demo (GIF)</summary>

![ASCII demo GIF](Gallery/sample-ascii.gif)

</details>


<details>

<summary>Electronika 60 demo (PNG)</summary>

![Electronika60 demo PNG](Gallery/sample-electronika60.png)

*\*For display just like in the screenshot set your terminal text color to green and font to a Unicode-compatible one (e.g. `DejaVu Sans Mono` works)*

</details>


# Features of the Terminal Application


### Gameplay

- Familiar stacker experience of moving, rotating, soft-/hard-dropping and holding Tetrominos and clearing completed rows.
- Colorful pieces.
- Next piece preview.
- Ghost piece.
- Animations: Hard drop, Line clears and Piece locking.
- Game stats: Current gravity level, lines cleared, points scored, time elapsed, pieces placed.

For more technical details see [Features of the Tetrs Engine](#features-of-the-tetrs-engine).


### Gamemodes

- Standard modes:
  - **40-Lines**: Clear 40-Lines as quickly as possible.
  - **Marathon**: Clear gravity levels 1-15 and achieve a highscore.
  - **Time Trial**: Get the highest score possible within three minutes.
  - **Master**: Clear 100 lines starting at the highest gravity.
- Special modes:
  - **Puzzle**: Advance through all 24 puzzle stages using perfect clears (and up to 5 attempts), enabled by piece acrobatics of the 'ocular' rotation system.
  - **Cheese**: Eat yourself through lines with random holes, with as few pieces as possible (default: 20).
  - **Combo**: Keep a line clear combo for as long as possible inside an infinite 4-wide well.
  - (**Descent**: Gather 'gems' as you navigate down (or up) an endless grid using an L or J piece - unlocked by completing Puzzle Mode)
- Custom mode: Change start gravity, toggle automatic gravity increase, set a game limit *(Time, Score, Pieces, Lines, Gravity, or No limit)*.
  

### Settings

- Look of the game:
  - Graphics (Unicode, ASCII, 'Electronika 60').
  - Colors (RGB Colors; 16 Colors (should work on all consoles), Monochrome).
  - Adjustable render rate and toggleable FPS counter.
- Play of the game:
  - Change controls.
  <details>
  
  <summary> Default In-Game Controls </summary>
  
  | Key | Action |
  | -: | :-: |
  | `←` | Move left |
  | `→` | Move right |
  | `A` | Rotate left |
  | `D` | Rotate right |
  | *(no def.)* | Rotate around (180°) |
  | `↓` | Soft drop |
  | `↑` | Hard drop |
  | *(no def.)* | Sonic drop |
  | `Esc` | Pause game |
  
  Special controls:

  | Key | Action |
  | `Ctrl`+`S` | *(experimental)* Take snapshot\* |
  | `Ctrl`+`D` | Forfeit game |
  | `Ctrl`+`C` | Exit program |

  \*This will remember the board and pieces so that configuration can be replayed in 'custom game'.
  
  </details>
  
  - Configure game.
    - Rotation system (Ocular, Classic, Super),
    - Piece generator (History, Uniform, Bag, Balance-Relative),
    - Preview count (0 - \<terminal width limit>),
    - DAS, ARR, hard drop delay, line clear delay, appearance delay,
    - soft drop factor, ground time max,
  - *Advanced*, No soft drop lock (Enables soft drop not instantly locking pieces on ground even if keyboard enhancements are off, for better experience on typical consoles (soft drops for piece spins)).
- **Keep Save File**: By default this program __won't store anything__ but just let you play the game. If you do want `tetrs_tui` to restore your settings and take record of past games upon startup then make sure this is set to **ON**!
  

### Local Scoreboard

- History of games played in the current session (or in the past, if "keep save file" is toggled on).
- *(\*Games where 0 lines have been cleared are auto-deleted on exit.)*

> [!NOTE]
> If "keep save file for tetrs" is toggled **ON** then your settings and games will be stored in `.tetrs_tui_savefile.json` under a directory that tries to follow OS conventions [[1](https://softwareengineering.stackexchange.com/questions/3956/best-way-to-save-application-settings), [2](https://softwareengineering.stackexchange.com/questions/299869/where-is-the-appropriate-place-to-put-application-configuration-files-for-each-p)]:
> | | Windows | Linux | macOS | other |
> | -: | - | - | - | - |
> | location | `%APPDATA%` | `~/.config/` | `~/Library/Application Support/` | (home directory) |
> 
> (If this fails it tries to store it locally, `./`.)


# Features of the Tetrs Engine

The frontend application is proof-of-concept;
Ultimately the tetrs engine tries to be modular and shifts the responsibility of detecting player input and chosen time of updates to the client.
Basic interaction with the engine could look like the following:

```rust
// Starting a game.
let game = tetrs_engine::Game::new(Gamemode::marathon());

// Application loop.
loop {
  // Updating the game with a new button state at a point in time.
  game.update(Some(new_button_state), update_time);
  // ...
  // Updating the game with *no* change in button state (since previous).
  game.update(None, update_time_2);

  // View game state
  let GameState { board, .. } = game.state();
  // (Render the board, etc..)
}
```

<details>

<summary> Using the engine in a Rust project </summary>

Adding `tetrs_engine` as a [dependency from git](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html) to a project:

```toml
[dependencies]
tetrs_engine = { git = "https://github.com/Strophox/tetrs.git" }
```

</details>

<details>

<summary> Game Configuration Aspects </summary>

- 'Standard' Gamemodes: Are encoded as a combination of *starting level* and *whether to increment level* and (one/several positive/negative) *limits*.
- Rotation Systems: *Ocular Rotation System*, *Classic Rotation System*, *Super Rotation System*. See [Ocular Rotation System](#ocular-rotation-system).
- Tetromino Generators: *Recency-based*, *Bag*, *Uniformly random*. Default is recency. See [Tetromino Generation](#tetromino-generation).
- Piece Preview (default 1)
- Delayed Auto Shift (default DAS = 167ms) *(\*Note: at very high levels [DAS and ARR equal lock delay - 1ms](https://www.reddit.com/r/Tetris/comments/cjkosd/tetris_effect_master_mode/).)*
- Auto Repeat Rate (default ARR = 33ms)
- Soft Drop Factor (default SDF = 15.0)
- Hard drop delay (default 0.1ms)
- Line clear delay (default 200ms)
- Appearance Delay (default ARE = 50ms)

Currently, drop delay and lock delay\* *(\*But not total ground time)* are a function of the current level:
- Drop delay (1000ms at lvl 1 to 0.833ms ("20G") at lvl 19, as per guideline)
- 'Timer' variant of Extended Placement Lockdown (step reset); The piece tries to lock every 500ms at lvl 19 to every 150ms at lvl 30, and any given piece may only touch ground for 3000ms in total. See [Piece Locking](#piece-locking).

Most default values inspired by [Guideline](https://tetris.wiki/Tetris_Guideline).

</details>

<details>

<summary> Game State </summary>

- Time: Game time is held abstract as "time elapsed since game started" and is not directly tied to real-world timestamps.
- Game finish: The game knows if it finished, and if session was won or lost. Normal Game Over scenarios are:
  - Block out: new piece spawn location is occupied.
  - Lock out: a piece was completely locked above the skyline (row 21 and above).
  - Forfeit: player stopped the current.
- Event queue: All game events are kept in an internal queue that is simulated through (up to the provided timestamp in the `Game::update` call).
- Buttons pressed state: The game keeps an abstract state of which buttons are currently pressed.
- Board state: (Yes).
- Active piece: The active piece is stored as a (tetromino, orientation, position) tuple plus some locking data.
- Next Pieces: Are polled from the generator, kept in a queue and can be viewed.
- Pieces played so far: A counter for each locked piece by type is stored.
- Lines cleared: (Yes)<sup>2</sup>.
- Gravity Level: Increases every 10 line clears and influences only drop/lock delay.
- Scoring: Line clears trigger a score bonus, which takes into account number of lines cleared, spins, combos, back-to-backs; See [Scoring](#scoring).
  
</details>


<details>

<summary> Game Feedback </summary>

The game provides some useful feedback events upon every `update`, usually used to correctly implement visual frontend effects:
- *Piece locked down*, *Lines cleared*, *Hard drop*, *Accolade* (score bonus info), *Message* (generic message, currently unused for base gamemodes)

</details>


# State of the Project

## Known Issues

- Buttons pressed in-game, held, and unpressed in the Pause menu do not register as unpressed in-game.


## Future Work

- The game snapshot functionality is limited to restoring the board and the pieces that are yet to be played - with notable exception of when taking a snapshot after pressing hold for the first time until a piece is locked. This bug should be fixed, or ideally, the functionality generalized to be able to take a snapshot of any game mode.
- Comprehensiveness of project README.
  - Many small details of the `tetrs_engine` are not properly explained (e.g. the initial rotation mechanic, which allows spawning a piece immediately rotated if a rotation button was held).
- FIXMEs in the code base.
- General improvements to the tetrs engine.
  - Better API documentation (`cargo doc --open`).
  - Better internal (implementation) documentation.
  - Refactor of complicated systems (e.g. locking).
  - *(Ideally)* Prove panic-freedom of `Game::update`.
- General improvements to the terminal application.
  - Make menu code less ad-hoc (code duplication, hidden panics).
  - Better internal documentation.

A personal goal would be to (at least partially) amend these problems, step-by-step.


# Project Highlights

While the [2009 Tetris Guideline](https://tetris.wiki/Tetris_Guideline) serves as good inspiration, I ended up doing a lot of amateur research into a variety of game details present in modern games online (thank you [Tetris Wiki](https://tetris.wiki/) and [HardDrop](https://harddrop.com/wiki)!) and also by getting some help from asking people.
I'd particularly like to thank the following people:
- **GrBtAce**, **KonSola5** and **bennxt** for helping me with my questions early in development :-)
- **Dunspixel** for the O-spin inspiration,
- and **madkiwi** for advice regarding 4wide 6res layouts.

In the following I detail various interesting concepts I tackled on my way to bringing this project to life - I was essentially new to the Tetris Community and couldn't remember playing it for more than a couple minutes (in the last decade), so I had to figure all this out from scratch!


## Tetromino Generation

[Tetromino generators are interesting](https://simon.lc/the-history-of-tetris-randomizers), and a core part of the game.

A trivial generator chooses tetrominos *uniformly at random*.
This already works decent for a playable game.

However, typically players tend to get very frustrated when they don't get "the pieces they need" for a while.
*(\*There's even a term for not getting an `I`-piece for an extended period of time: Drought.)*

In modern games, the *bag-based generation system* is ubiquitous;
The system simply takes all 7 pieces once, hands them out in random order, repeat.

It's quite nice knowing an `I` piece will come after 12 pieces, every time. One may even start strategizing and counting where one bag ends and the next starts.

It also takes a bit of the "fun" out of the randomness.

An alternative that seems to work well is *recency-based generation*:
Remember the last time each piece was generated and when choosing the next one randomly, do so weighted by when each one was last seen so it is more likely to choose a piece not played in a while.

This preserves "possibly complete randomness" in the sense that *any* piece may be generated at any time, while still mitigating droughts.

Unlike bag it possibly provides a more continuous "gut feeling" of what piece(s) might come next, where in bag the order upon refill really *is* completely random.


## Ocular Rotation System

> "[tetris has a great rotation system and is not flawed at all](https://www.youtube.com/watch?v=_qaEknA81Iw)"

— *said no one ever, not even [the creator of Tetris himself](https://youtu.be/6YhkkyXydNI?si=jbVwfNtfl5yFh9Gk&t=674).*

That's a bit harsh of course, considering the sheer size of the franchise and range of players coming from all sorts of niches and previous game versions, which the official *Super Rotation System* somehow [gets its job done](https://www.youtube.com/watch?v=dgt1kWq2_7c)™ - nevertheless it was *the* mechanic I wanted to redo even before starting this project.

<details>

<summary> Creating a Rotation System. </summary>

My personal gripes with SRS are:

- The system is not symmetric.
  - Symmetric pieces can look exactly the same in different rotation states, **[but have different behaviour](https://blog.battlefy.com/how-accidental-complexity-harms-the-tetris-community-93311461b2af)**.
  - For symmetrical pieces, rotation is different depending on whether it's right or left (even though it should be symmetric too).
- It's an [advanced system](https://harddrop.com/wiki/SRS) with things like different rotation points for different purposes, yet it re-uses the *exact same kicks* for 5 out of the 7 pieces, even though they have completely different symmetries.
- <sup>Not a hot take, but some rotations are just *weird* (to be chosen over other possibilities).</sup>
- Piece elevators.

Good general criteria for a rotation system I can think of would be:

1. Rotation must behave visually **symmetrically**;
    - equal-looking rotation states must behave the same,
    - and mirroring the board/pieces mirrors the rotation behaviour perfectly.
2. The kicks should be **intuitive**
    - the way pieces rotate should look 'feasible' to any given person;
    - e.g. the new position cannot be completely disjoint from previously.
3. Rotation should be fun! :D
    - but also any rotations should 'feel good' to a player.
    - (but don't overdo it - no teleporting pieces.)

The result of this was the *'Ocular' Rotation System*, which was made by... *looking* at each piece and orientation and drawing the 'best' position(s) for it to land in after rotating (following above points and gut feeling).

I present to you - the Ocular Rotation System Heatmap:

![Ocular Rotation System Heatmap](Gallery/rotation/ocular-rotation-system_16px.png)

*How to read it*:
This heatmap is created by considering each combination of (piece, orientation, rotate left or right).
By overlapping all the _new_ possible positions for a piece after rotation, one gets a compact visualization of where the piece will most likely land, going from brightest color (yellow, first position attempt) to darkest (purple, last attempt):

Here's a comparison with SRS - the Super Rotation System Heatmap:

![Super Rotation System Heatmap](Gallery/rotation/super-rotation-system_16px.png)

With SRS one starts to spot some rotational symmetries (you can always rotate back-and-forth between two positions), but I think it's overshadowed by all the asymmetrical kicks and very lenient (downwards *and upwards*) vertical kicks that contribute to SRS' unintuitiveness.

</details>

In the end I'm happy with how the custom rotation system turned out.
It vaguely follows the mantra "if the rotation looks like it could reasonably work visually, it should" (+ some added kicks for flexibility and fun :-), hence its name, *Ocular* rotation system.

<details>

<summary> Ocular Rotation System - Comments </summary>

The general rationale behind most kicks is, "these first kicks feel most natural, any additional kicks after that serve flexibility, cool tricks and skill ceiling". However, more concrete heuristics can be stated:
- A general trend with kicks is to *prevent needless upwarping* of pieces. This simply means we first prefer rotations into a position *further down* before trying further up (foregoing nonsensical upward kicks in the first place).
- New positions must look 'visually close' to the original position. One thing observed in this system is that there are no disjoint rotation states, i.e. the old and new position always overlap in at least one tile. This heuristic also influenced the **L**/**J** rotations, always incorporating all rotations where *two* of the new pieces overlap with the old.
- The **I**, **S** and **Z** pieces are more rotationally symmetric than **T**, **L** and **J**, yielding the same visual shape whether rotated left or right.
  However, they do not have a natural 'center' in a way that would allow them to be rotated precisely in this sense, forcing us to choose their new position to be more left or right. We use this to our advantage and allow the player to have direct control over this by inputting one of the two rotation directions. This may arguably aid both directional intuition as well as allow for better finesse. It should not hurt rotational intuition ("piece stays in place if rotated 360°") as a player in a sense would never need to rotate such symmetrical pieces (especially mid-air) more than once anyway. :P


*\*Notation*: `nTlr 0-3` describes kick positions `0` to `3` when rotating a `n`orth-facing `T`-piece to the `l`eft _or_ `r`ight.

![Ocular Rotation System Heatmap](Gallery/rotation/ocular-rotation-system+_16px.png)

- **O**-piece.
  - As the most symmetrical piece, having no kicks would be most natural, but also make it the only piece where rotation is 'useless'. Adding limited kicks however already turns out to be very useful to the player:
    - `nOl 0`: Simple sideways O-'roll', providing a second way to move O.
    - `nOl 1`: Allows for O-spins.
    - `nOl 2`: Also allows for O-spins and - more likely - 'rolling' the O out of a shallow well, possibly useful under high gravity.
    - `nOl 3`: While invisible on the chart, this technically allows the O to *always* successfully change its orientation upon rotation.
- **I**-piece.
  - As the longest piece, it has the most kicks to accommodate its visual range. What's special is that in its horizontal position it may rotate left or right into *any* reachable space around itself.
    - `nIl 0-7`: Rotating in it from a horizontal to a vertical position will very strongly prefer placing it in the desired direction of rotation first.
    - `nIl 8-9`: Fallback rotation states.
    - `eIl 5-6`: Allows tucking the I down flat.
    - Non-existent higher-positioned `eIl`: Intentionally left away to prevent upwarping.
- **S**/**Z**-piece.
  - The rotations for these pieces also allow for directional finesse.
    - `nSZlr`: Kept simple and symmetrical.
    - `eSr 2`|`eZl 2`: Useful and commonly applicable S/Z spin.
    - `eSl 2`|`eZr 2`: Tucking in an S/Z in this position might be handy.
- **T**-piece.
  - This piece has a relatively natural center of rotation, making for a good first rotation reposition.
    - `neswTlr 0`: 'Center' rotation.
    - `nTlr 4`: A T-'turn'.
    - `eTl 4`|`wTr 4`: A T-'insert', allowing it to warp down.
    - `sTlr 4`: A T-'turn'.
    - `sTlr 5`: This kick enables T-spin triples.
    - `wTl 3-4`|`eTr 3-4`: Two T-'turns'.
- **L**/**J**-piece.
  - Surprisingly, the most common center of rotation for L/J does not lead to a first rotation reposition where an overlap of two tiles with the original position is observed.
    - `neswLJlr 0`: 'Center' rotation.
    - `nLl 4`|`nJr 4`, `eLl 5`|`wJr 5`, `sLl 2`|`sJr 2`, `wLl 1`|`wJr 1`: Additional 'wall'-kicks that come after trying the 'center' rotation.
    - `nLr 4`|`nJl 4`, `eLr 5`|`wJl 5`, `sLr 4`|`sJl 4`, `wLr 2`|`eJl 2`: Somewhat weird kicks that are mostly included due to context and symmetry in rotation.
    - `wLl 6-7`|`eJr 6-7`: Two ways to specially tuck an L/J down.
    - `sLr 6`|`sJl 6`: Allows rotation into upright state even when resting on other blocks in a well.

</details>


## Piece Locking

The mechanics of locking down a piece on the grid can be more complicated than it might sound at first glance.

Good criteria for a locking system I can think of would be:

1. Keep players from stalling / force players to make a choice eventually.
2. Give players enough flexibility to manipulate the piece even if it's on the ground.
3. Force players to *react/input faster* on faster levels, as speed is supposed to increase.
4. Implement all these limitations as naturally/simply as possible.

So I started looking and deciding which locking system to implement;

<details>

<summary> Creating a Locking System. </summary>

*Classic lock down* is simple, but if one decreases the lock timer on faster levels (3.) then it might become exceedingly difficult for players to actually have enough time to do adjustments (2.).

<details>

<summary> Classic Lock Down </summary>

> - If the piece touches a surface
>   - start a lock timer of 500ms (\**var with lvl*).
>   - record the lowest y coordinate the piece has reached.
> - If the lock timer runs out, lock the piece immediately as soon as it touches the next surface.
> - If the piece falls below the previously lowest recorded y coordinate, reset lock timer.

</details>

*Infinite lock down* essentially mitigates the flexibility issue by saying, *"if the player manipulated his piece, give him some more time"*. 
It's very simple, but the fact that it lets players stall forever (1.) is less nice.

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

It's pretty flexible (2.) yet forces a decision (1.), but the 'count to 15 moves' part of this lock down seems somewhat arbitrary (4.)
<sup>*(\*Also note that after the 15 moves run out one can still manipulate the piece till lock down.)*</sup>

> **Idea.**
> 
> What if we limit the *total amount of time a piece may touch a surface* (1.) instead of number of moves/rotates (4.), but on faster levels the piece *attempts* to lock down faster (3.), re-attempting later upon move/rotate;
> This still allows for plenty <sup>*\*technically arbitrarily many*</sup> piece manipulations (2.) while still fulfilling the other points :D

<details>

<summary> 'Timer' Placement Lock Down </summary>

*Let 'ground time' denote the amount of time a piece touches a surface*

> - If the piece touches a surface
>   - start a lock timer of 500ms (\**var with lvl*).
>   - start measuring the ground time.
>   - record the lowest y coordinate the piece has reached.
> - If the piece moves/rotates (change in position), reset lock timer ('move reset').
> - If the lock timer runs out *or* the ground time reaches 2.25s, lock the piece immediately as soon as it touches the next surface.
> - If the piece falls below the previously lowest recorded y coordinate, reset the ground time.

Nice.

</details>

Although now it *may potentially* be abused by players which keep pieces in the air, only to occasionally touch down and reset the lock timer while hardly adding any ground time (note that this problem vanishes at 20G).

A small patch for this is to check the last time the piece touched the ground, and if that was, say, less than 2×(drop delay) ago, then act as if the piece had been touching ground all along. This way the piece is guaranteed to be counted as "continuously on ground" even with fast upward kicks of height ≤ 2.

</details>

In the end, a timer-based extended placement lockdown (+ ground continuity fix) is what I used.
Although there might be a nicer system somehow..


## Scoring

The exact scoring formula is given as follows:
<details>

<summary>Scoring Formula</summary>

```haskell
score_bonus = 10
            * (lines + combo - 1) ^ 2
            * maximum [1, backToBack]
            * (if spin then 4 else 1)
            * (if perfect then 100 else 1)
  where lines = "number of lines cleared simultaneously"
        spin = "piece could not move up when locking occurred"
        perfect = "board is empty after line clear"
        combo = "number of consecutive played pieces where line clear occurred"
        backToBack = "number of consecutive line clears where spin, perfect or quadruple line clear occurred"
```

</details>


<details>

<summary>Table of Example Bonuses</summary>

*A table of some example bonuses:*
| Score bonus | Action |
| -: | :- |
| +10 | Single |
| +40 | Double |
| +90 | Triple |
| +160 | Quadruple |
| +40 | ?-Spin Single |
| +160 | ?-Spin Double |
| +360 | ?-Spin Triple |
| +40 | Single (2.combo) |
| +90 | Single (3.combo) |
| +160 | Single (4.combo) |
| +90 | Double (2.combo) |
| +160 | Double (3.combo) |
| +250 | Double (4.combo) |
| +160 | Triple (2.combo) |
| +250 | Triple (3.combo) |
| +360 | Triple (4.combo) |
| +320 | Quadruple (2.B2B) |
| +480 | Quadruple (3.B2B) |
| +640 | Quadruple (4.B2B) |
| +1'000 | Perfect Single |
| +16'000 | Perfect L-Spin Double |

</details>

Coming up with a *good* [scoring system](https://tetris.wiki/Scoring#Recent_guideline_compatible_games) is easier with practical experience and playtesters.

I did actually try to come up with a new, simple, good formula, but it's tough to judge how much to reward the player for any given action *(how many points should a 'perfect clear' receive? - I've never achieved a single perfect clear in my life!)*.
The one I came up with, put mildly, *probably sucks*.

But I still allowed myself to experiment, because I really liked the idea of [rewarding all spins](https://harddrop.com/wiki/List_of_twists) (and don't understand modern Tetris' obsession with T-spins when S-, Z-, L- and J-spins are also so satisfying).


## Controls

A search for the 'best' / 'most ergonomic' game keybinds was [inconclusive](https://youtube.com/watch?v=6YhkkyXydNI&t=809).
In a sample of a few dozen opinions from reddit posts there was about a 50/50 split on

| move | rotate |
| - | - |
| `a` `d` | `←` `→` |
| `←` `→` | `z` `x` |

Frequent advice I saw was: "choose what feels best for you", which sounds about right. :P
*(\*though some mentioned one should **not** hammer ` spacebar ` for hard drops, though it's the only button the Guideline binds to this action.)*


## Menu Navigation

Modeling how a TUI should handle menus and move between them was unclear initially.
Luckily, I was able to look at how [Noita](https://noitagame.com/)'s menus are connected and saw that it was quite structured:
The menus form a graph (with menus as nodes and valid transitions as directed edges), with only some menus ('pop-ups') that allow backtracking to a previous menu.

<details>

<summary>Tetrs Terminal Menu Graph</summary>

![tetrs menu graph](Gallery/tui-menu-graph.svg)

</details>


## Combo Bot

[Screencast from 2024-09-08 22-21-45 combot-demo.webm](https://github.com/user-attachments/assets/a3d96655-7d96-4f87-80ff-b1c86840ced3)


### Background

The goal of 'Combo Mode' is to keep a combo going for as many pieces/lines as possible, where combo is maintained by clearing lines with consecutive pieces.

The fact that this is playable as its own gamemode is due to a special strategy known as the *4 wide (3 residual) combo setup*.
Here the board is completely filled except for a dedicated 4-wide vertical tunnel, at the bottom of which there are always 'residual' cells that help in clearing a line with the upcoming piece.

It turns out there are only a finite number of these configurations inside a 4-wide well are useful to keep a combo:

<details>

<summary>

**Image of 4-wide 3-residual combo setup configurations.**

</summary>

*Notice how due to the fact that a tetromino consists of 4 cells, clearing a line like this will always leave another 3 cells in the otherwise perfectly vertical 4-wide tunnel.*

![harddrop.com 4wide 3res combo continuations](/Gallery/combo/harddrop.com_4-Wide-Combo-Setups.png)

Graphic with modifications courtesy of [harddrop](https://harddrop.com/wiki/Combo_Setups#4-Wide_with_3_Residuals).

</details>


### Problem Approach

Given finite piece preview and armed with the knowledge of how these combo states transition into each other, how do we find the best way to continue a combo?

It turns out we can model this as a [graph](https://en.wikipedia.org/wiki/Graph_(abstract_data_type)) where we can see what paths we can take from our current state:

<details>

<summary>

**Graph of all states reachable with 4 preview pieces + hold.**

</summary>

![4-lookahead state graph](/Gallery/combo/combo_4-lookahead-graph.svg)

</details>

Different decisions can be made depending on whether we hold the current piece and use a different one, which can radically change the outcome.

What the bot does with this is to look at the farthest states it finds, and chooses the branch that maximizes depth and possibilities to make different decisions later on (states with many continuations).

While humans can vaguely do this for certain previews and also get a feeling for it, one can program a computer to do this automatically even for larger preview sizes (see a [12-lookahead state graph here](/Gallery/combo/combo_12-lookahead-graph.svg)).

The bot currently only supports lookahead up to 42 (number of bit triplets that fit into `u128`), although it already tends to get quite slow for values half of that.
As we'll see, it still does pretty okay for reasonable preview sizes.


### Results and Evaluation

<details>

<summary>
Sidenote: <i>weeeeeee</i>
</summary>

[Screencast from 2024-09-08 22-52-51 combot-weee.webm](https://github.com/user-attachments/assets/8506a73c-446b-431a-ad78-ce0617023a0a)

</details>

In short, the bot is pretty good and gets exponentially longer combos with larger preview, as can also be seen from the following chart.

| Samples | Randomizer | Lookahead | Median combo | Average combo | Maximum combo |
|-|-|-|-|-|-|
| 10000 | 'bag' | 0 |    8 |   11 |   114 |
| 10000 | 'bag' | 1 |   17 |   22 |   232 |
| 10000 | 'bag' | 2 |   29 |   40 |   426 |
| 10000 | 'bag' | 3 |   50 |   76 |   695 |
| 10000 | 'bag' | 4 |   82 |  129 |  1434 |
| 10000 | 'bag' | 5 |  150 |  244 |  2435 |
| 10000 | 'bag' | 6 |  300 |  502 |  5028 |
| 10000 | 'bag' | 7 |  540 |  985 | 10663 |
| 10000 | 'bag' | 8 | 1123 | 2126 | 24040 |
| 10000 | 'bag' | 9 | 2255 | 4199 | 54664 |

It is also interesting to note the differences depending on the [randomizer used to generate the next pieces](#tetromino-generation).

<details>

<summary>
Another table.
</summary>

| Samples | Randomizer | Lookahead | Median combo | Average combo | Maximum combo |
|-|-|-|-|-|-|
| 100000 | 'uniform'            | 3 |   15 |   23 |   287 |
| 100000 | 'balance-relative'   | 3 |   23 |   33 |   470 |
| 100000 | 'bag'                | 3 |   50 |   74 |   988 |
| 100000 | 'bag-2'              | 3 |   25 |   37 |   555 |
| 100000 | 'bag-3'              | 3 |   20 |   29 |   363 |
| 100000 | 'bag-2_restock-on-7' | 3 |   21 |   28 |   328 |
| 100000 | 'bag-3_restock-on-7' | 3 |   21 |   28 |   328 |
| 100000 | 'recency-0.0'        | 3 |   15 |   23 |   303 |
| 100000 | 'recency-0.5'        | 3 |   34 |   55 |   734 |
| 100000 | 'recency-1.0'        | 3 |   46 |   73 |   918 |
| 100000 | 'recency-1.5'        | 3 |   58 |   92 |  1238 |
| 100000 | 'recency-2.0'        | 3 |   68 |  107 |  1477 |
| 100000 | 'recency'            | 3 |   76 |  120 |  1592 |
| 100000 | 'recency-3.0'        | 3 |   83 |  129 |  1837 |
| 100000 | 'recency-7.0'        | 3 |  118 |  184 |  2715 |
| 100000 | 'recency-16.0'       | 3 |  223 |  374 |  4961 |
| 100000 | 'recency-32.0'       | 3 | 2583 | 4798 | 70998 |

</details>

Additionally, these benchmarks also produce visualization of the actual distribution of combos:

![combo distribution, recency, 0-lookahead, 1'000 samples](/Gallery/combo/combot-2024-09-08_19-01-11_L0_recency.svg)

<details>

<summary>
It is interesting to note how these distributions can spread out quickly for higher lookaheads.
</summary>

![combo distribution, recency, 4-lookahead, 2'500 samples](/Gallery/combo/combot-2024-09-08_19-02-47_L4_recency.svg)

</details>

<details>

<summary>
Running the bot on a uniform randomizer for 1'000'000 samples yields a much nicer, smooth curve.
</summary>

![combo distribution, uniform, 1-lookahead, 1'000'000 samples](/Gallery/combo/combot-2024-09-07_18-09-31_L1_uniform.png)

</details>

Basically all common randomizers I implemented will have a exponential-decay-looking curves smoothing out for many samples like this.

According to some [programming Okey\_Dokey did on 4wide combos](https://harddrop.com/forums/index.php?topic=7955), there are deadly sequences of 'bags' that will kill the combo (and needless to say, randomizers which allow truer forms of randomness can kill even more easily).
This somewhat explains that the curves are at least somewhat of exponential (decay) nature, as it gets more likely over time that the bot slips up and a random sequence appears that kills the combo.

<details>

<summary>
One more thing - I lied about all distributions looking the same.
</summary>

Look at this curious chart for the bag randomizer, which despite a million samples *does not* seem to smooth out:

![combo distribution, bag, 1-lookahead, 1'000'000 samples](/Gallery/combo/combot-2024-09-07_18-09-31_L1_bag.png)

What's happening with these obvious valleys and peaks?
Upon further inspection, the peaks are at: 6, 13, 20, 27, 34, 41, ... can you spot the pattern?

The answer is that the most of the runs die toward the end of a (7-piece) bag, because the bot optimizes paths expecting arbitrary pieces, but due to how this randomizer works every 7th piece is in fact *never* random, while every 7th+1 piece is *completely* random. That's why the bot is tripping over hurdles at those points, and shows that it is not actually making use of all the information that would be available to make the best decisions.

</details>

Overall this is an interesting topic in the overlap of tetromino randomizers, graph theory and a bit of statistics.
There's a larger space to be explored here - 4wide 3res is not the only combo setup (and this bot is pretty hardcoded in that respect). An interesting project would be to do something with 4wide 6res, which I heard is more resilient to combo killers, but that might have to wait for some other time :-)

> [!NOTE]
> The bot can be enabled to run in Combo Mode with a cmdline flag (`./tetrs_tui -e`).
> 
> To make it output the lookahead graphs, a feature is needed at compile time: `cargo run --release --features graphviz`.
> 
> To produce statistics, `cargo test <"simple"|"lookaheads"|"randomizers">` was used.


## Miscellaneous Author Notes

In the two very intense weeks of developing a majority of this project I had my first proper learning experience with building: a larger Rust project, an interactive game (in the console no less), and the intricacies of modern tetrs mechanics, all at the same time.

Gamedev-wise I can mention the [modern](https://gafferongames.com/post/fix_your_timestep/) [game](http://gameprogrammingpatterns.com/game-loop.html) [loop](https://dewitters.com/dewitters-gameloop/);
Finding a proper abstraction for `Game::update` (allowing arbitrary-time user input, making updates decoupled from framerate) wasn't so obvious at the beginning.

Frontend-wise I may as well have used [Ratatui](https://crates.io/crates/ratatui/), but decided to just do some basic menus using trusty [Crossterm](https://crates.io/crates/crossterm) for cross-platform terminal manipulation.
However, next time I should use a TUI crate so as to sleep more peacefully at night not having to take responsibility for the ~horrible ad-hoc code I wrote for the interface aaaAAAA .~

On the Rust side of things I read / learned about / read:
- some primers on [coding](https://docs.kernel.org/rust/coding-guidelines.html) [style](https://doc.rust-lang.org/nightly/style-guide/) [guidelines](https://github.com/rust-lang/rust-analyzer/blob/master/docs/dev/style.md#getters--setters) & `cargo fmt` (~`#[rustfmt::skip]`~),
- "[How to order Rust code](https://deterministic.space/how-to-order-rust-code.html)",
- an introduction to [writing](https://doc.rust-lang.org/book/ch14-02-publishing-to-crates-io.html) [documentation](https://rust-lang.github.io/api-guidelines/documentation.html) (like the fact they can even contain [tested examples](https://blog.guillaume-gomez.fr/articles/2020-03-12+Guide+on+how+to+write+documentation+for+a+Rust+crate#Hiding-lines)) & `cargo doc`,
- [`std` traits](https://rust-lang.github.io/api-guidelines/interoperability.html),
- using [serde](https://serde.rs/derive.html) (to [save some structured data locally](https://stackoverflow.com/questions/62771576/how-do-i-save-structured-data-to-file)),
- [conditional derives based on feature flags](https://stackoverflow.com/questions/42046327/conditionally-derive-based-on-feature-flag) & `cargo check --features serde`,
- [conditional compilation](https://doc.rust-lang.org/reference/conditional-compilation.html),
- basic [file system](https://doc.rust-lang.org/std/fs/index.html) shenanigans,
- [clap](https://docs.rs/clap/latest/clap/) to parse simple command line arguments (& passing them with `cargo run -- --descent-mode`),
- [formatting the time](https://docs.rs/chrono/latest/chrono/struct.DateTime.html#method.format) with [chrono](https://rust-lang-nursery.github.io/rust-cookbook/datetime/parse.html#display-formatted-date-and-time),
- [the `format!` macro](https://doc.rust-lang.org/std/fmt/#fillalignment) (the analogue to Python's f-strings! my beloved),
- [`debug_struct`](https://doc.rust-lang.org/std/fmt/struct.Formatter.html#method.debug_struct) (helpful to `impl Debug` for structs with weird fields),
- [the annoyances with terminal emulators](https://sw.kovidgoyal.net/kitty/keyboard-protocol/#progressive-enhancement) ~including how slow they can be~,
- [`BufWriter`](https://doc.rust-lang.org/std/io/struct.BufWriter.html) as an easy wrapper (diminished flickering!),
- [setting a custom panic hook](https://doc.rust-lang.org/std/panic/fn.set_hook.html) (TUI stuff tends to mess with console state upon crash),
- more practice with [Rust's module system](https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html),
- super handy multithreading with [`std::sync::mpsc`](https://doc.rust-lang.org/std/sync/mpsc/)
- [cargo workspaces](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html) (to fully separate frontend and backend),
- how [cargo git dependencies](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#specifying-dependencies-from-git-repositories) work (so one could reuse the backend from this repository),
- and [cross-compilation](https://blog.logrocket.com/guide-cross-compilation-rust/#how-rust-represents-platforms) (for releases).

All in all, Rust (known for its safety and performance while still providing ADTs) - proved to be an excellent choice for this project!

Also, can we appreciate how nice the name *tetrs* fits for a Rust game that does not infringe on TTC's copyright? <sup>~~though there were like a million other [`tetrs`](https://github.com/search?q=%22tetrs%22&type=repositories)'s on GitHub before me oof~~</sup>.

Other stuff:
- For the menu navigation graph I learned DOT (and used [graphviz](http://magjac.com/graphviz-visual-editor/)).
- For the combo bot graph I learned and generated simple [SVG](https://developer.mozilla.org/en-US/docs/Web/SVG/Tutorial) myself.
- For the terminal GIF recordings I used [asciinema](https://asciinema.org/) + [agg](https://github.com/asciinema/agg):
  ```bash
  agg --font-family="DejaVu Sans Mono" --line-height=1.17 --renderer=resvg --font-size=20, --fps-cap=30 --last-frame-duration=0  my_rec.cast my_rec.gif
  ```


\- If you find any typos in this readme you're more than welcome to keep them! ;D  *(\*actually you can open a PR if you really want to)*


*„Piecement Places!“* - [CTWC 2016](https://www.youtube.com/watch?v=RlnlDKznIaw&t=121).


<div  align="center">
  
`██ ▄▄▄▄ ▄█▀ ▀█▄ ▄█▄ ▄▄█ █▄▄`

</div>
