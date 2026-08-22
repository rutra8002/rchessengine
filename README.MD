# rchessengine

A UCI-compliant chess engine written in Rust.
It is deployed on Oracle Cloud Server, and playable on [lichess](https://lichess.org/@/rchessengine).

## Building

### Using Cargo

To build a release binary:

```bash
cargo build --release
```

### Using Nix

To build for Linux:

```bash
nix build
```

To build for Windows:

```bash
nix build .#windows
```

## Benchmarking

To run the benchmark suite:

```bash
cargo bench
```