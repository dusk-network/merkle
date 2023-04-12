# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Add `Aggregate::NULL` associated constant
- Derive `Debug`, `Clone`, `PartialEq`, `Eq`, and `Hash` for `Tree` and `Opening` [#13]
- Add `blake3` feature, implementing `Aggregate` for `blake3::Hash` [#11]

### Fixed

- Fix `CheckBytes` derivation in `Node` [#15]

<!-- ISSUES -->
[#15]: https://github.com/dusk-network/merkle/issues/15
[#13]: https://github.com/dusk-network/merkle/issues/13
[#11]: https://github.com/dusk-network/merkle/issues/11

<!-- VERSIONS -->
[Unreleased]: https://github.com/dusk-network/merkle/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/dusk-network/merkle/releases/tag/v0.1.0
