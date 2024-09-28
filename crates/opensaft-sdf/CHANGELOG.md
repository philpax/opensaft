# `opensaft-sdf` changelog

All notable changes to this project will be documented in this file.

## Unreleased

## 0.2.0 - 2023-08-23
- Bump minor version due to the `num_enum` dependency change that was released in 0.1.1.
  `num_enum` is part of the public api and a minor bump of it needs to be paired with a
  minor bump of `saft-sdf` version.

## 0.1.1 - 2023-08-21

### Changed 🔧

- Upgrade `num_enum` v0.6.0 -> v0.7.0
- Fix `speedy` Clippy lint

## 0.1.0 - 2023-05-24

- Initial release. Broke out `Interpreter` and signed distance field math from `saft`.
