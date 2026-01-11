# strip-whitespace-fuzz

Fuzzing harness for the `strip-whitespace` core crate.

This crate is intended to be run via `cargo fuzz` and is not published.

## Targets

- `strip` (`fuzz_targets/strip.rs`)
- `strip_no_sourcemap` (`fuzz_targets/strip_no_sourcemap.rs`)

## Running

Prerequisites:

- `cargo-fuzz` installed (`cargo install cargo-fuzz`)
- A nightly toolchain (cargo-fuzz requires nightly)

From the repo root:

- `cargo +nightly fuzz run strip`
- `cargo +nightly fuzz run strip_no_sourcemap`

On Windows with ASAN, the repo includes a helper script:

- `./scripts/fuzz-win.ps1 -Target strip -Runs 1000`
