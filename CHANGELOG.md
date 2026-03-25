# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [Unreleased]

-


## [2.1.0] - 2026-03-25

### Added
- Experimental: **New lineclear effect!**
- When loading savefile fails on startup, the exact error can be viewed in the Advanced Settings menu.

### Changed
- Palette colors are now stored formatted as hex in savefile.

### Fixed
- Cheese mode bugs fixed.
- Enabling override-enhanced-keyboard-events on Windows shouldn't lead to spurious error on program exit.
- Faulty replay data should not lead to infinite load screen anymore.
- Kitty: menu texts can now be properly selected and copied.

### Removed
-



## [2.0.1] - 2026-03-23

### Changed
- Tweaked gameplay settings names.
- Bumped dependencies.

### Fixed
- Made it possible to copy+paste text from menus (e.g. ⇝'Advanced Settings'⇝'Savefile location')


## [2.0.0] - 2026-03-23

### Added
- Settings menu:
    * Quick-switch between Graphics/Keybinds/Gameplay settings slots!
    * Added more default slots for each of the sub-settings: In particular Gameplay from old tetromino stackers.
- Gameplay settings:
    * Experimental **Double-tap movement finesse** toggle: Allow move double-taps to be converted to teleports.
    * **Ensure move delay less than lock delay** toggle.
    * Option to **adjust Bag size** or **Recency weighting** (⇝'Tetromino generation'⇝`Alt`+`←`/`→`)
    * Rotation system: 'Classic' properly split into 'ClassicL' and 'ClassicR'.
- **Advanced settings** (gathers special options from previous menus):
    * Save-contents granularity.
    * Override assume-enhanced-key-events (reminder: useful for some Windows terminals which support this but aren't auto-recognized).
    * Blindfolded gameplay (reminder: toggle in-game using `Ctrl`+`Alt`+`B`).
    * **Renderertype**: Default / Halfcell / Braille
- **Statistics** menu: As with settings, optionally stored in savefile.
- In-Game:
    * **Quick-load** savepoints using `Ctrl`+`L` now! (reminder: save using `Ctrl`+`S`).
    * Game over will now show subtle effects depending on it happened: In particular 'Block out' shows how piece overlapped.
- Replay:
    * **Reset replay speed** using `-`. 
    * Skip **forward one state change** using `Alt`+`.`

### Changed
- Savefile is now versioned using only `MAJOR.MINOR` (e.g. `/home/meee/.config/.tetro-tui_v2.0_savefile.json`) (i.e. should not break on simple `.PATCH` releases).
- List of some renames:
    * '16-Color' palette -> 'ANSI' palette
    * '40-Lines' gamemode -> 'Swift' gamemode
    * 'Marathon' -> 'Classic'
    * 'Custom' mode -> dynamic: 'Lines-40'/'Score-900'/'Limitless'/...
    * 'Single', 'Double', 'Triple', 'Quadruple', 'Quintuple' ... -> 'Mono', 'Duo', 'Tri', 'Tetra', 'Penta' ...
- 'Master' mode is now only unlocked if 'Classic' has been completed. (secret command for the experienced/impatient: `Ctrl`+`U`.)
- Certain special controls that used `[Shift+?]` now use `[Alt+?]` (for better compatibility).
- Animated Game Complete menu; and Title menu is fancier now, too (even with its different design in ASCII mode)!
- Many, many small TUI tweaks: text/labels, menu navigation...

### Fixed
- Replay:
    * Modded games now also generate state anchors for efficient arbitrary time skips (reminder: using `0`..`9`).
    * Fixed rendering logic to make it flicker less on 'very low-end' terminals.
    * More efficient when game is paused.
- Scores-and-Replays menu:
    * Now remembers which game you had selected when you return from watching a replay.
    * Switching between scoreboard ordering automatically keeps selection on same game as well.
- Changes/refactors of in the game engine (falling-tetromino-engine v1.4) should make it more robust now; bugs fixed/prevented.
- `Ctrl`+`C` should now consistently exit program + save (if applicable) from all menus.
- 'ASCII' graphics should now properly only use ASCII graphics even for small tetromino previews, button state icons.

### Removed
- 'Time Trial' gamemode: Use 'Custom'⇝'Limit: Some(Time(180s))' instead.
- `b` menu keybind (was alias for `Backspace`/`q`/`Esc`). 


## [1.1.0] - 2026-02-16

### Fixed
- 'Blindfold' toggle functions again (using [Ctrl+Alt+B])
- Turning on enhanced-keyboard-events override no longer crashes on Windows (enabling actual use cases despite kitty protocol not being detected)
- Framerate in live games should be more stable
- Display correct total replay length/time
- Mark games derived from replay with an apostrophe (')
- Restored modded games (replays, savepoints) no longer contain empty print_warn_msg modifiers


## [1.0.0] - 2026-02-16

### Added
- Initial release
- main tetro-tui application

### Changed
-

### Fixed
-

### Removed
-
