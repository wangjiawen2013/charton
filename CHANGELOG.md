# Changelog

All notable changes to the `charton` library are documented here. The project
follows [Semantic Versioning](https://semver.org/). Releases before 0.7.0
predate this file.

## [0.7.0]

### Added

- `ThemeMode` (`Light` / `Dark`), with `Theme::light()`, `Theme::dark()`,
  `Theme::with_mode()`, and `Theme::is_dark()`. A mode only changes colors, so
  switching between light and dark leaves layout and typography untouched.

### Changed

- Tightened the default canvas margins — `top` 0.05 → 0.025, `right` 0.03 →
  0.02, `bottom` 0.08 → 0.03, `left` 0.06 → 0.025 — so charts use their space
  better. The plot area is now larger; set margins explicitly to restore the
  previous look.
- Reduced the per-axis edge buffer from 10px to 4px. The canvas margins already
  provide the outer breathing room, so the buffer is now only a small safety
  gap.

### Fixed

- Legend symbols for size-only and shape-only legends now use the theme's
  legend ink instead of a hard-coded `#333333`, which was nearly invisible on
  dark backgrounds.

[0.7.0]: https://github.com/wangjiawen2013/charton/releases/tag/v0.7.0
