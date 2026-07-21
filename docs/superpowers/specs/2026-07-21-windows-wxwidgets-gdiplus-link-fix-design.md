# Windows wxWidgets GDI+ Link Fix

## Context

The Windows CI build fails while linking `espanso.exe` with `LNK2005` and
`LNK1169`. The duplicate symbols include `GdipAlloc`, `GdipFree`,
`GdiplusStartup`, and image-loading functions.

The collision has two participants:

- Espanso statically builds the vendored wxWidgets 3.1.5 archive. Its
  `src/msw/gdiplus.cpp` defines wrappers for the GDI+ flat API.
- The new Slint settings UI adds Windows dependencies whose generated import
  library also defines the same `Gdip*` symbols.

Espanso supports Windows 10 and Windows 11. Both platforms provide Direct2D,
so wxWidgets does not need its legacy GDI+ graphics-context fallback.

## Goal

Build the settings-enabled `espanso.exe` on Windows without duplicate GDI+
symbols while retaining wxWidgets Direct2D support and the Slint UI.

## Non-goals

- Do not downgrade Slint or change its renderer.
- Do not change Linux or macOS builds.
- Do not suppress linker errors with `/FORCE:MULTIPLE`.
- Do not replace or upgrade the vendored wxWidgets archive.

## Decision

Patch the extracted wxWidgets source before `nmake` compiles it. Disable only
the wxWidgets GDI+ graphics backend; retain `wxUSE_GRAPHICS_CONTEXT` and its
Direct2D implementation.

The build must apply two exact transformations:

1. Change the Windows setup value for `wxUSE_GRAPHICS_GDIPLUS` from
   `wxUSE_GRAPHICS_CONTEXT` to `0`.
2. Guard `src/msw/gdiplus.cpp` with `wxUSE_GRAPHICS_GDIPLUS` instead of the
   broader `wxUSE_GRAPHICS_CONTEXT` setting.

This removes wxWidgets' competing `Gdip*` definitions. The Direct2D graphics
context remains available.

## Components

### Patch helper

Add a small, Windows-independent Rust helper under `espanso-modulo` build
support. It receives the two source texts, verifies each original or patched
marker occurs exactly once, and returns the patched texts plus a changed flag.
The filesystem wrapper reads and writes the extracted wxWidgets files.

The helper treats already-patched input as a successful no-op. It must fail with
a specific error when neither state matches or a marker is repeated. This
converts a future vendor-source change into an early build error instead of
silently producing a partial patch.

### Build integration

Call the helper in the Windows branch of `espanso-modulo/build.rs` after the
source tree exists and before `nmake`. Other platforms do not compile or run
this path.

Run `nmake` when the source tree was extracted, the patch changed either file,
or the expected build output is absent. If the patch changes a previously built
tree supplied through `WX_WIDGETS_BUILD_OUT_DIR`, clean that wxWidgets build
before recompiling it. This prevents an old static library from surviving in a
shared build directory.

### Tests

Keep pure text-transformation tests beside the helper. The tests cover:

- both expected substitutions;
- already-patched input as an idempotent no-op;
- a missing setup marker;
- a missing source guard;
- repeated markers.

Run the helper tests as a standalone Rust test binary in Windows CI before the
full Cargo build. This avoids coupling the focused test to wxWidgets linking.

## Verification

The Windows CI job must complete these checks in order:

1. focused patch-helper tests;
2. `cargo build` for the complete workspace and default features;
3. `cargo check -p espanso-settings`;
4. `cargo test -p espanso-settings --no-default-features`;
5. `cargo clippy -- --deny warnings`.

The final link command must produce `espanso.exe` without `LNK2005` or
`LNK1169`.

## Risks

Disabling the GDI+ backend removes the graphics-context fallback used on
pre-Direct2D Windows releases. This does not reduce the repository's declared
Windows 10/11 support. Exact marker checks protect against applying the patch to
an incompatible future wxWidgets archive.

## Acceptance Criteria

- Windows CI links the settings-enabled `espanso.exe` successfully.
- The focused helper tests pass.
- Settings checks and Clippy pass.
- Linux and macOS behavior remains unchanged.
- No linker suppression flag or dependency downgrade is introduced.
