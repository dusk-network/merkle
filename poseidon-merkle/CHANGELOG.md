# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.8.0] - 2025-02-07

### Changed

- Update `dusk-plonk` to v0.21
- Update `dusk-poseidon` to v0.41
- Update `dusk-bls12_381` to v0.14

## [0.7.0] - 2024-08-14

### Changed

- Update `dusk-plonk` to v0.20
- Update `dusk-poseidon` to v0.40

## [0.6.1] - 2024-08-28

### Added

- Implement `dusk_bytes::Serializable` for `Item<()>`

## [0.6.0] - 2024-05-22

### Changed

- Update `dusk-poseidon` to 0.39 [#85]
- Fix `ARITY` in the poseidon-tree to `4` [#85]

## [0.5.0] - 2024-01-03

### Changed

- Update `dusk-plonk` to 0.19
- Update `dusk-poseidon` to 0.33

## [0.4.0] - 2023-12-13

### Changed

- Update `dusk-bls12_381` to 0.13
- Update `dusk-plonk` to 0.18
- Update `dusk-poseidon` to 0.32

## [0.3.0] - 2023-10-12

### Changed

- Update `dusk-bls12_381` to 0.12
- Update `dusk-poseidon` to 0.31
- Update `dusk-plonk` to 0.16

## [0.2.1] - 2023-07-19

### Fixed

- Fix `rkyv-impl` feature

## [0.2.0] - 2023-06-28

### Changed

- Update `dusk-merkle` to `v0.5.0`

## [0.1.0] - 2023-06-28

### Added

- Add poseidon-merkle crate [#58]

<!-- ISSUES -->
[#85]: https://github.com/dusk-network/merkle/issues/85
[#58]: https://github.com/dusk-network/merkle/issues/58

<!-- VERSIONS -->
[Unreleased]: https://github.com/dusk-network/merkle/compare/poseidon-merkle_v0.8.0...HEAD
[0.8.0]: https://github.com/dusk-network/merkle/compare/poseidon-merkle_v0.7.0...poseidon-merkle_v0.8.0
[0.7.0]: https://github.com/dusk-network/merkle/compare/poseidon-merkle_v0.6.1...poseidon-merkle_v0.7.0
[0.6.1]: https://github.com/dusk-network/merkle/compare/poseidon-merkle_v0.6.0...poseidon-merkle_v0.6.1
[0.6.0]: https://github.com/dusk-network/merkle/compare/poseidon-merkle_v0.5.0...poseidon-merkle_v0.6.0
[0.5.0]: https://github.com/dusk-network/merkle/compare/poseidon-merkle_v0.4.0...poseidon-merkle_v0.5.0
[0.4.0]: https://github.com/dusk-network/merkle/compare/poseidon-merkle_v0.3.0...poseidon-merkle_v0.4.0
[0.3.0]: https://github.com/dusk-network/merkle/compare/poseidon-merkle_v0.2.1...poseidon-merkle_v0.3.0
[0.2.1]: https://github.com/dusk-network/merkle/compare/poseidon-merkle_v0.2.0...poseidon-merkle_v0.2.1
[0.2.0]: https://github.com/dusk-network/merkle/compare/poseidon-merkle_v0.1.0...poseidon-merkle_v0.2.0
[0.1.0]: https://github.com/dusk-network/merkle/releases/tag/poseidon-merkle_v0.1.0
