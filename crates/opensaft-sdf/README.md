# 🧃 OpenSaft-sdf

Signed distance field interpreter and function library

This is a fork of Embark Studios' [`saft-sdf`](https://crates.io/crates/saft-sdf) with the intent of making its source code more accessible to the public.

Previously, the original library's source code was only available through its published crate. It was never published to a public code repository, and [directional changes](https://github.com/EmbarkStudios/rust-ecosystem/commit/61f0ec820350a9e107f92e2dc189217d313c75db) at Embark suggest that this is unlikely to occur in the near future.

Additionally, this fork removes the dependency on [`macaw`](https://crates.io/crates/macaw) in favour of using [`glam`](https://crates.io/crates/glam) directly. This should ease integration with other Rust projects, including [`bevy`](https://crates.io/crates/bevy).

## Contributing

We welcome community contributions to this project.

Please read our [Contributor Guide](CONTRIBUTING.md) for more information on how to get started.

## License

Licensed under either of

* Apache License, Version 2.0, [LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0)>
* MIT license [LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT)>

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
