# watchso

[![Crates.io](https://img.shields.io/crates/v/watchso.svg)](https://crates.io/crates/watchso) [![Documentation](https://docs.rs/watchso/badge.svg)](https://docs.rs/watchso/) [![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](https://github.com/acheroncrypto/watchso/blob/master/LICENSE)

Hot reload [Solana](https://solana.com) programs.

## Installation

Install with [cargo](https://www.rust-lang.org/learn/get-started):

```sh
cargo install watchso --locked
```

## Usage

Run in the directory of your project:

```sh
watchso
```

This will:

1. Check whether the necessary tools are installed e.g [solana-cli-tools](https://docs.solana.com/cli/install-solana-cli-tools).
2. Start a Solana test validator if it's not already running.
3. Update program id(s) if there is a mismatch between the keypair files and the source code.
4. Build the program(s).
5. Deploy the program(s).
6. Hot reload on changes.

### Supported frameworks

- [Native Solana](https://github.com/solana-labs/solana)
- [Anchor](https://github.com/coral-xyz/anchor)
- [Seahorse](https://github.com/ameliatastic/seahorse-lang)

## License

[Apache-2.0](https://github.com/acheroncrypto/watchso/blob/master/LICENSE)
