# Contribution Guidelines

1. If you are unsure whether a change is desired, open an issue and ask first;
   nobody wants to waste time working on something that won't get merged anyway!
2. Make sure your local environment is setup correctly as explained in [Local Setup](./README.md#local-setup).
3. Rust has [great tooling](https://doc.rust-lang.org/book/appendix-04-useful-development-tools.html).
   Use it! `cargo clippy` and `cargo +nightly fmt` will be your best friends.

Before committing, you should run `just check` to make sure your code
   * compiles correctly
   * doesn't violate any linter rules
   * is formatted correctly
   * is documented properly
