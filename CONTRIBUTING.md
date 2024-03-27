# Contribution Guidelines

1. If you are unsure whether a change is desired, open an issue and ask first;
   nobody wants to waste time working on something that won't get merged anyway!
2. Make sure your local environment is setup correctly as explained in
   [Local development setup](#local-development-setup).
3. Rust has [great tooling](https://doc.rust-lang.org/book/appendix-04-useful-development-tools.html).
   Use it! `cargo clippy` and `cargo +nightly fmt` will be your best friends.

# Local development setup

As described in the [README](./README.md), you should have both
[rustup](https://www.rust-lang.org/tools/install) and
[Docker](https://www.docker.com) installed on your machine. Make sure to
install the nightly toolchain in addition to the stable one
(`rustup toolchain install nightly`) for use with `rustfmt`; it has a bunch of
unstable rules that are used in this project. If you want a nice command
runner, install [just](https://github.com/casey/just) as well, and have a look
at the [Justfile](./Justfile).

Before committing, you should run `just precommit` to make sure your code
   * compiles correctly
   * doesn't violate any linter rules
   * is formatted correctly
   * is documented properly
