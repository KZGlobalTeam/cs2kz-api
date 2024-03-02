# Contribution Guidelines

1. Make sure your dev environment is correctly setup as explained in [the README](./README.md#dev-setup).
2. If you want to implement a new feature or refactor an existing one, please open an issue first.
3. Rust has
   [great tooling](https://doc.rust-lang.org/book/appendix-04-useful-development-tools.html).
   Use it. Lint your code with `cargo clippy` and format it with `cargo +nightly fmt` when
   contributing code. The `justfile` has helpers for these: `just check` and `just format`.
4. Write [good commit messages](https://cbea.ms/git-commit)!
