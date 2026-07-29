//! Disables the wxWidgets graphics-context subsystem on Windows.
//!
//! `src/msw/gdiplus.cpp` implements wrappers for the GDI+ flat API (`GdipAlloc`,
//! `GdiplusStartup`, ...). Those definitions collide with the import library
//! shipped by the `windows` crates that the Slint settings UI pulls in, which
//! makes the final link fail with `LNK2005`/`LNK1169`.
//!
//! Turning off only `wxUSE_GRAPHICS_GDIPLUS` is not enough: in wxWidgets 3.1.5
//! `src/msw/graphics.cpp` is the only MSW translation unit defining
//! `wxGraphicsRenderer::GetDefaultRenderer()`, and it is guarded by that same
//! setting, so `src/common/graphcmn.cpp` is left with an unresolved external
//! (`LNK2019`). The whole graphics-context subsystem is therefore disabled,
//! which is a configuration wxWidgets supports and keeps internally consistent.
//!
//! `wxUSE_ACTIVITYINDICATOR` has to follow, because `include/wx/chkconf.h`
//! raises `#error "wxUSE_ACTIVITYINDICATOR requires wxGraphicsContext"` when it
//! stays enabled.

use std::fs;
use std::path::Path;

/// `(original, patched, label)` markers applied to `include/wx/msw/setup.h`.
///
/// Each patched marker must not appear anywhere in the pristine header, so that
/// an already-patched tree is recognized as an idempotent no-op. `setup.h`
/// already contains a plain `#define wxUSE_GRAPHICS_CONTEXT 0` in the
/// non-MSVC branch, hence the trailing comment on that replacement.
const SETUP_MARKERS: [(&str, &str, &str); 2] = [
    (
        "#define wxUSE_GRAPHICS_CONTEXT 1",
        "#define wxUSE_GRAPHICS_CONTEXT 0 // espanso: avoid GDI+ symbol collision",
        "wxUSE_GRAPHICS_CONTEXT",
    ),
    (
        "#define wxUSE_ACTIVITYINDICATOR 1",
        "#define wxUSE_ACTIVITYINDICATOR 0",
        "wxUSE_ACTIVITYINDICATOR",
    ),
];

fn patch_marker(
    source: &str,
    original: &str,
    patched: &str,
    label: &str,
) -> Result<(String, bool), String> {
    let original_count = source.matches(original).count();
    let patched_count = source.matches(patched).count();

    match (original_count, patched_count) {
        (1, 0) => Ok((source.replacen(original, patched, 1), true)),
        (0, 1) => Ok((source.to_owned(), false)),
        _ => Err(format!(
            "include/wx/msw/setup.h: {label}: expected exactly one original or patched marker, \
             found {original_count} original and {patched_count} patched"
        )),
    }
}

/// Applies every setup marker, returning the patched header and whether it changed.
pub fn patch_setup(setup: &str) -> Result<(String, bool), String> {
    let mut current = setup.to_owned();
    let mut changed = false;

    for (original, patched, label) in SETUP_MARKERS {
        let (next, marker_changed) = patch_marker(&current, original, patched, label)?;
        current = next;
        changed |= marker_changed;
    }

    Ok((current, changed))
}

pub fn patch_tree(root: &Path) -> Result<bool, String> {
    let setup_path = root.join("include").join("wx").join("msw").join("setup.h");
    let setup = fs::read_to_string(&setup_path)
        .map_err(|error| format!("unable to read {}: {error}", setup_path.display()))?;
    let (patched, changed) = patch_setup(&setup)?;

    if changed {
        fs::write(&setup_path, &patched)
            .map_err(|error| format!("unable to write {}: {error}", setup_path.display()))?;
    }

    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mirrors the relevant shape of `include/wx/msw/setup.h`, including the
    /// non-MSVC `#else` branch that already defines the setting to `0`.
    const ORIGINAL_SETUP: &str = "\
#if defined(_MSC_VER)
#define wxUSE_GRAPHICS_CONTEXT 1
#else
#define wxUSE_GRAPHICS_CONTEXT 0
#endif
#define wxUSE_ACTIVITYINDICATOR 1 // wxActivityIndicator
";

    const PATCHED_SETUP: &str = "\
#if defined(_MSC_VER)
#define wxUSE_GRAPHICS_CONTEXT 0 // espanso: avoid GDI+ symbol collision
#else
#define wxUSE_GRAPHICS_CONTEXT 0
#endif
#define wxUSE_ACTIVITYINDICATOR 0 // wxActivityIndicator
";

    #[test]
    fn patches_both_setup_markers() {
        let (patched, changed) = patch_setup(ORIGINAL_SETUP).unwrap();

        assert_eq!(patched, PATCHED_SETUP);
        assert!(changed);
    }

    #[test]
    fn accepts_an_already_patched_tree_without_changes() {
        let (patched, changed) = patch_setup(PATCHED_SETUP).unwrap();

        assert_eq!(patched, PATCHED_SETUP);
        assert!(!changed);
    }

    #[test]
    fn rejects_a_missing_graphics_context_marker() {
        let error = patch_setup("#define wxUSE_ACTIVITYINDICATOR 1\n").unwrap_err();

        assert!(error.contains("wxUSE_GRAPHICS_CONTEXT"));
        assert!(error.contains("found 0 original and 0 patched"));
    }

    #[test]
    fn rejects_a_missing_activity_indicator_marker() {
        let error = patch_setup("#define wxUSE_GRAPHICS_CONTEXT 1\n").unwrap_err();

        assert!(error.contains("wxUSE_ACTIVITYINDICATOR"));
    }

    #[test]
    fn rejects_repeated_markers() {
        let repeated = format!("{ORIGINAL_SETUP}{ORIGINAL_SETUP}");
        let error = patch_setup(&repeated).unwrap_err();

        assert!(error.contains("found 2 original and 0 patched"));
    }

    #[test]
    fn reports_the_patched_file_in_every_error() {
        let error = patch_setup("unrecognized setup").unwrap_err();

        assert!(error.contains("include/wx/msw/setup.h"));
    }
}
