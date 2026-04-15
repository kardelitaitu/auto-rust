# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.0.30] - 2026-03-27

### Added

- Enhanced setup wizard with native module rebuild and pre-flight checks
- New documentation structure under `/docs/` folder

### Changed

- Updated better-sqlite3 to v12.8.0

### Removed

- Various unused files cleaned up (logs, temp scripts, lock files)

### Fixed

- Resolved clipboard race conditions and text clipping in concurrent sessions

## [0.0.29] - 2026-03-26

### Added

- Integration test coverage improvements
- Enhanced agent workflow tests

### Changed

- Improved decision.js coverage

## [0.0.28] - 2026-03-27

### Added

- Edge case tests for decision.js
- Improved test coverage (67%)

### Removed

- Generated files from git tracking

## [0.0.27] - 2026-03-25

### Added

- Test helpers unit tests
- Enhanced code coverage reporting

## [0.0.26] - 2026-03-24

### Added

- CI workflow for automated testing

### Changed

- Updated lint configuration

## [0.0.25] - 2026-03-23

### Added

- Git workflow helpers (pnpm commit, pnpm amend)

### Fixed

- Various bug fixes and improvements

## [0.0.20] - 2026-03-20

### Added

- Multi-browser automation framework
- AI agent for autonomous decisions
- Human-like behavior patterns
- Session isolation with AsyncLocalStorage

## [Unreleased]

### Added

- `/docs/` folder with modern documentation structure
- Updated README with industry-standard format
- CHANGELOG.md

### Removed

- Duplicate documentation files
- Unused scripts and temporary files

### Changed

- Consolidated documentation into `/docs/`

---

## Upgrade Notes

### 0.0.30+

- Run `setup_windows.bat` to rebuild native modules
- Check `.env` for new configuration options

### 0.0.20+

- Requires Node.js 18+
- Uses pnpm instead of npm

---

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) for details on how to contribute.
