# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]



## [0.1.18] - 2023-01-15

### Added

- Add release builds for alpine/musl (#59).


## [0.1.17] - 2022-11-10

### Changed

- Upgrade to clap 4.0.


## [0.1.16] - 2022-11-10

### Added

- Github actions continuous integration.

### Changed

- Update all dependencies.
- Update README with installation from cargo.io.
- Migrate to git-graph 0.5.0.


## [0.1.15] - 2021-01-26

### Changed

- No line coloring for old/new file version.


## [0.1.14] - 2021-01-22

### Changed

- Upgrade to git-graph 0.4.3.

### Fixed

- Fix tags bug (#48).


## [0.1.13] - 2021-01-21

### Added

- Optional line wrapping in diffs.

### Changed

- Upgrade to git-graph 0.4.2.

### Fixed

- Prevent crash when trying to open a repo that is a shallow clone.


## [0.1.12] - 2021-01-17

### Added

- Jump to graph view when selecting a branch in full-screen mode.

### Changed

- Adapted help for interactive app.
- Brighten default foreground color of solarized theme.

### Fixed

- Fix missing newline.
- Don't reset scroll on toggle syntax highlighting.
- Prevent crash on utf-8 error.


## [0.1.11] - 2021-01-15

### Added

- Delayed display of commit diff files.
- Scroll margins for graph, files, and branches.
- Scroll indicators for graph, files, and branches.

### Fixed

- Space for line numbers in diff.

### Changed

- Switch to git-graph 0.4.1.


## [0.1.10] - 2021-01-13

### Added

- Search in graph view text.
- Jump to HEAD with Pos1/Home.
- Go into directory on Enter, if it is not a repo.
- Exit repo dialog with Ctrl+O
- Highlight directories that are repos.
- Add entry '..' to folder list to navigate upwards.

### Changed

- Migrate to git-graph version 0.4.0.


## [0.1.9] - 2021-01-12

### Added

- Add navigation hints to panel titles.


## [0.1.8] - 2021-01-11

### Added

- Adjust number of context lines in diff (+/-).
- Show file diff, old or new version (D/O/N).

### Changed

- Clear secondary selection when it equals primary selection
  (but not vice versa).


## [0.1.7] - 2021-01-10

### Added

- Horizontal scrolling for file list, branches and diff view.

### Changed

- Sort tags in inverse chronologic order.


## [0.1.6] - 2021-01-09

### Changed

- Branches panel, restrict dialog size.
- Reset secondary selection: changed from Enter to Backspace.

### Added

- Far left panel to show branches.


## [0.1.5] - 2021-01-09

### Fixed

- Minor errors.


## [0.1.4] - 2021-01-08

### Changed

- Better error messages when open repository fails.
- Disable mouse capture. This allows selection of text.
- Select previous folder when navigating upwards in file dialog.


## [0.1.3] - 2021-01-07

### Added

- Optional line numbers in diff.
- Fast scrolling.


## [0.1.2] - 2021-01-07

### Fixed

- Show diff for initial commit (without parents).


## [0.1.1] - 2021-01-06

### Added

- Show repository name.
- Open repository dialog.


## [0.1.0] - 2021-01-05

### Added

- Initial release with basic ui and commmit info.
