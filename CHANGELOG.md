# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
