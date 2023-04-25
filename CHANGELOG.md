# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Add `Tree::walk` allowing a user to walk the tree [#21]
- Add example of generating zero hashes for `blake3`
- Add `EMPTY_SUBTREES` to `Aggregate` trait
- Derive `Debug`, `Clone`, `PartialEq`, `Eq`, and `Hash` for `Tree` and `Opening` [#13]

### Changed

- Change `Aggregate` trait to bind `Self` to be `Copy`
- Change `Tree::root` to return `&T` as opposed to `Option<&T>`
- Change `Tree` structure by removing `len` field

### Fixed

- Fix `CheckBytes` derivation in `Node` [#15]

<!-- ISSUES -->
[#21]: https://github.com/dusk-network/merkle/issues/21
[#15]: https://github.com/dusk-network/merkle/issues/15
[#13]: https://github.com/dusk-network/merkle/issues/13

<!-- VERSIONS -->
[Unreleased]: https://github.com/dusk-network/merkle/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/dusk-network/merkle/releases/tag/v0.1.0
