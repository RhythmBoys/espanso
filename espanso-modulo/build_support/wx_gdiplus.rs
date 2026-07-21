use std::fs;
use std::path::Path;

const ORIGINAL_SETUP_MARKER: &str = "#define wxUSE_GRAPHICS_GDIPLUS wxUSE_GRAPHICS_CONTEXT";
const PATCHED_SETUP_MARKER: &str = "#define wxUSE_GRAPHICS_GDIPLUS 0";
const ORIGINAL_SOURCE_MARKER: &str = "#if wxUSE_GRAPHICS_CONTEXT";
const PATCHED_SOURCE_MARKER: &str = "#if wxUSE_GRAPHICS_GDIPLUS";

#[derive(Debug, Eq, PartialEq)]
pub struct PatchedSources {
    pub setup: String,
    pub gdiplus: String,
    setup_changed: bool,
    gdiplus_changed: bool,
}

impl PatchedSources {
    pub fn changed(&self) -> bool {
        self.setup_changed || self.gdiplus_changed
    }
}

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
            "{label}: expected exactly one original or patched marker, \
             found {original_count} original and {patched_count} patched"
        )),
    }
}

pub fn patch_contents(setup: &str, gdiplus: &str) -> Result<PatchedSources, String> {
    let (setup, setup_changed) = patch_marker(
        setup,
        ORIGINAL_SETUP_MARKER,
        PATCHED_SETUP_MARKER,
        "include/wx/msw/setup.h",
    )?;
    let (gdiplus, gdiplus_changed) = patch_marker(
        gdiplus,
        ORIGINAL_SOURCE_MARKER,
        PATCHED_SOURCE_MARKER,
        "src/msw/gdiplus.cpp",
    )?;

    Ok(PatchedSources {
        setup,
        gdiplus,
        setup_changed,
        gdiplus_changed,
    })
}

pub fn patch_tree(root: &Path) -> Result<bool, String> {
    let setup_path = root.join("include").join("wx").join("msw").join("setup.h");
    let gdiplus_path = root.join("src").join("msw").join("gdiplus.cpp");
    let setup = fs::read_to_string(&setup_path)
        .map_err(|error| format!("unable to read {}: {error}", setup_path.display()))?;
    let gdiplus = fs::read_to_string(&gdiplus_path)
        .map_err(|error| format!("unable to read {}: {error}", gdiplus_path.display()))?;
    let patched = patch_contents(&setup, &gdiplus)?;

    if patched.setup_changed {
        fs::write(&setup_path, &patched.setup)
            .map_err(|error| format!("unable to write {}: {error}", setup_path.display()))?;
    }
    if patched.gdiplus_changed {
        fs::write(&gdiplus_path, &patched.gdiplus)
            .map_err(|error| format!("unable to write {}: {error}", gdiplus_path.display()))?;
    }

    Ok(patched.changed())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGINAL_SETUP: &str =
        "before\n#define wxUSE_GRAPHICS_GDIPLUS wxUSE_GRAPHICS_CONTEXT\nafter\n";
    const PATCHED_SETUP: &str = "before\n#define wxUSE_GRAPHICS_GDIPLUS 0\nafter\n";
    const ORIGINAL_GDIPLUS: &str = "before\n#if wxUSE_GRAPHICS_CONTEXT\nafter\n";
    const PATCHED_GDIPLUS: &str = "before\n#if wxUSE_GRAPHICS_GDIPLUS\nafter\n";

    #[test]
    fn patches_both_wxwidgets_markers() {
        let patched = patch_contents(ORIGINAL_SETUP, ORIGINAL_GDIPLUS).unwrap();

        assert_eq!(patched.setup, PATCHED_SETUP);
        assert_eq!(patched.gdiplus, PATCHED_GDIPLUS);
        assert!(patched.changed());
    }

    #[test]
    fn accepts_an_already_patched_tree_without_changes() {
        let patched = patch_contents(PATCHED_SETUP, PATCHED_GDIPLUS).unwrap();

        assert_eq!(patched.setup, PATCHED_SETUP);
        assert_eq!(patched.gdiplus, PATCHED_GDIPLUS);
        assert!(!patched.changed());
    }

    #[test]
    fn rejects_a_missing_setup_marker() {
        let error = patch_contents("unrecognized setup", ORIGINAL_GDIPLUS).unwrap_err();

        assert!(error.contains("include/wx/msw/setup.h"));
    }

    #[test]
    fn rejects_a_missing_gdiplus_guard() {
        let error = patch_contents(ORIGINAL_SETUP, "unrecognized source").unwrap_err();

        assert!(error.contains("src/msw/gdiplus.cpp"));
    }

    #[test]
    fn rejects_repeated_markers() {
        let repeated_setup = format!("{ORIGINAL_SETUP}{ORIGINAL_SETUP}");
        let error = patch_contents(&repeated_setup, ORIGINAL_GDIPLUS).unwrap_err();

        assert!(error.contains("found 2 original and 0 patched"));
    }
}
