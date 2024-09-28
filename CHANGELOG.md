# `opensaft` changelog

All notable changes to this project will be documented in this file.

## Unreleased

## 0.34.1 - 2024-02-21

- Require Rust 1.76.0

## 0.34.0 - 2023-08-23

- Bump minor version due to the `num_enum` dependency change that was released in 0.33.1.
  `num_enum` is part of the public api and a minor bump of it needs to be paired with a
  minor bump of `saft` version.

## 0.33.1 - 2023-08-21

### Changed 🔧

- Upgrade `rust-gpu` v0.7 -> v0.8
- Upgrade `tiny-bench` to v0.3
- Fix markdown lint warnings
- Require [Rust 1.71.1+](https://blog.rust-lang.org/2023/08/03/Rust-1.71.1.html)
- Fix `speedy` Clippy lint
- Fix ambiguous shadowing in imports/exports
- Fix clippy tuple to array warning
- Upgraded `num_enum` v0.6 -> v0.7
- Enable publishing `saft` to crates.io by not relying on wildcard deps

## 0.33.0 - 2023-04-28

- Require Rust 1.69.0
- Remove dependency on `puffin`
- Upgraded `rust-gpu` to v0.6

## 0.32.0 - 2023-05-24

- Require Rust 1.67.0
- Broke out `Interpreter` and sdf math to separate crate called `saft-sdf`.
- `Interpreter` now operates on a slice of opcodes and constants rather than a `Program`.

## 0.31.0 - 2022-12-16

- Require Rust 1.66.0
- Updated to `macaw` 0.18.0

## 0.30.1 - 2022-09-16

- Require Rust 1.63
- Added new functionality for sphere tracing and converting a triangle mesh to an obj-file.

## 0.30.0 - 2022-04-05

### Added 🔧

- Added optional `arbitrary` feature that derives the `Arbitrary` trait allowing to generate arbitrary values for each type, useful for fuzz testing

### Changed 🔧

- Updated to `macaw` 0.17.0

## 0.29.0 - 2022-02-21

- Update `puffin` to 0.13.1

## 0.28.0 - 2022-01-14

- Update `glam` to 0.20
- Update to `macaw` 0.16.0

## 0.27.0 - 2021-11-25

- Update `puffin` to 0.12.0

## 0.26.0 - 2021-11-15

- Update `puffin` to 0.11.0

## 0.25.0 - 2021-11-05

- Upgrade to `macaw` 0.15.0

## 0.24.0 - 2021-10-25

- Upgrade to `puffin` 0.10.0

## 0.23.0 - 2021-10-18

- Upgrade to `macaw` 0.14.0

## 0.22.0 - 2021-10-12

- Upgrade to `macaw` 0.13.0

## 0.21.0 - 2021-10-04

- Upgrade to `macaw` 0.12.0

## 0.20.1 - 2021-09-14

- Upgrade to `macaw` 0.11.2

## 0.20.0 - 2021-08-24

- Upgrade to `puffin` 0.8

## 0.19.0 - 2021-08-24

- Upgrade to `puffin` 0.7

## 0.18.1 - 2021-08-23

- Upgrade to `macaw` 0.11.1

## 0.18.0 - 2021-08-02

- Upgrade to `macaw` 0.11 and `glam` 0.17

## 0.17.1 - 2021-08-02

- First public published version
- Use public published `macaw` 0.10.6

## 0.17.0 - 2021-07-08

- Renamed `SaftError` to `Error`
