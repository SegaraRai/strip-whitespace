# Rust fuzzing (cargo-fuzz)

This repo’s Rust implementation lives in the workspace member at `core/`.
The `wasm/` crate is a thin wrapper for wasm-bindgen / JS consumers.

Fuzzing is set up via `cargo-fuzz` under `fuzz/`.

## Prereqs

- Install `cargo-fuzz`:

  ```bash
  cargo install cargo-fuzz
  ```

- You typically want a **nightly** toolchain for best sanitizer support:

  ```bash
  rustup toolchain install nightly
  ```

## Targets

- `strip_no_sourcemap`: fuzzes `strip_astro_whitespace_no_sourcemap`
- `strip_sourcemap_create`: fuzzes `strip_astro_whitespace_sourcemap_create`
- `strip_sourcemap_roundtrip`: creates a sourcemap then rewrites it

## Run

From repo root:

```bash
cargo +nightly fuzz run strip_no_sourcemap
```

You can also pass a corpus directory:

```bash
cargo +nightly fuzz run strip_no_sourcemap fuzz/corpus/strip_no_sourcemap
```

Artifacts/crashes will be written under `fuzz/artifacts/` (ignored by git).

## Windows notes (MSVC)

On Windows with the MSVC toolchain, `cargo-fuzz` enables AddressSanitizer (ASan) by default.
The fuzz target EXE will fail to launch with `0xc0000135 (STATUS_DLL_NOT_FOUND)` unless the ASan runtime DLL is available on `PATH`.

If you have Visual Studio installed, you can usually fix this for the current PowerShell session by adding the directory that contains `clang_rt.asan_dynamic-x86_64.dll` to `PATH`:

```powershell
$asanDll = Get-ChildItem "$env:ProgramFiles\Microsoft Visual Studio" -Recurse -ErrorAction SilentlyContinue -Filter "clang_rt.asan_dynamic-x86_64.dll" |
  Select-Object -First 1

if (-not $asanDll) {
  throw "Could not find clang_rt.asan_dynamic-x86_64.dll under $env:ProgramFiles\\Microsoft Visual Studio"
}

$env:PATH = "$($asanDll.Directory.FullName);$env:PATH"
cargo +nightly fuzz run strip_no_sourcemap -- -runs=1
```

Note: `--sanitizer none` is currently not a good option on Windows/MSVC for this setup; it can fail to link due to missing sanitizer-coverage symbols.

Alternatively, run the helper:

```powershell
./scripts/fuzz-win.ps1 -Target strip_no_sourcemap -Runs 1
```
