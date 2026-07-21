# Windows wxWidgets GDI+ Link Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Link the settings-enabled `espanso.exe` on Windows without duplicate wxWidgets and windows-rs GDI+ symbols.

**Architecture:** A pure Rust build-support helper patches the extracted wxWidgets configuration and GDI+ source guard before `nmake` runs. The Windows build path recompiles a reused wxWidgets tree when the patch changes it, and Windows CI runs the focused helper tests before the full workspace build.

**Tech Stack:** Rust 1.97, Cargo build scripts, wxWidgets 3.1.5, MSVC `nmake`/`link.exe`, GitHub Actions.

## Global Constraints

- Preserve Slint 1.17.1 and its current renderer features.
- Preserve wxWidgets Direct2D graphics-context support.
- Support x86_64 Windows 10 and Windows 11; do not add pre-Direct2D compatibility.
- Do not change Linux or macOS build behavior.
- Do not add `/FORCE:MULTIPLE` or another linker-error suppression flag.
- Do not replace or upgrade `espanso-modulo/vendor/wxWidgets-3.1.5-patched-version-3.zip`.
- Apply the patch idempotently and fail when the vendored source matches neither the original nor patched state.

## File Structure

- Create `espanso-modulo/build_support/wx_gdiplus.rs`: pure marker transformation, filesystem adapter, and standalone unit tests.
- Modify `espanso-modulo/build.rs`: call the patch helper and rebuild stale wxWidgets output.
- Modify `.github/workflows/ci.yml`: run the focused helper test on the Windows runner before `cargo build`.
- Create `C:/Users/ZQ/.Codex/skills/knowledge-base/lessons/rust-windows-linking.md`: record the reusable linker lesson after CI proves the fix.

---

### Task 1: Build a guarded, idempotent wxWidgets source patcher

**Files:**

- Create: `espanso-modulo/build_support/wx_gdiplus.rs`

**Interfaces:**

- Consumes: an extracted wxWidgets root directory containing `include/wx/msw/setup.h` and `src/msw/gdiplus.cpp`.
- Produces: `pub fn patch_tree(root: &Path) -> Result<bool, String>`, returning `true` only when it writes a patch.
- Produces: `pub fn patch_contents(setup: &str, gdiplus: &str) -> Result<PatchedSources, String>` for focused tests.

- [ ] **Step 1: Write the failing transformation tests**

Create `espanso-modulo/build_support/wx_gdiplus.rs` with the test module first:

```rust
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
```

- [ ] **Step 2: Run the tests and verify RED**

Run from a Windows Rust developer shell:

```powershell
rustc --edition 2021 --test espanso-modulo/build_support/wx_gdiplus.rs -o "$env:TEMP\wx-gdiplus-patch-tests.exe"
```

Expected: compilation fails because `patch_contents` is undefined.

- [ ] **Step 3: Implement the minimal transformation and filesystem adapter**

Insert this code before the test module:

```rust
use std::fs;
use std::path::Path;

const ORIGINAL_SETUP_MARKER: &str =
    "#define wxUSE_GRAPHICS_GDIPLUS wxUSE_GRAPHICS_CONTEXT";
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
```

- [ ] **Step 4: Run the focused tests and verify GREEN**

```powershell
rustc --edition 2021 --test espanso-modulo/build_support/wx_gdiplus.rs -o "$env:TEMP\wx-gdiplus-patch-tests.exe"
& "$env:TEMP\wx-gdiplus-patch-tests.exe"
```

Expected: `5 passed; 0 failed`.

- [ ] **Step 5: Format and commit the helper**

```powershell
rustfmt --edition 2021 espanso-modulo/build_support/wx_gdiplus.rs
git check-ignore -v espanso-modulo/build_support/wx_gdiplus.rs
git status --short
git diff --stat
git add espanso-modulo/build_support/wx_gdiplus.rs
git commit -m "fix(windows): add guarded wx GDI+ patcher"
```

Expected: `git check-ignore` prints nothing; the commit contains only the helper.

---

### Task 2: Apply the patch before compiling wxWidgets

**Files:**

- Modify: `espanso-modulo/build.rs:20-125`
- Modify: `.github/workflows/ci.yml:178-184`

**Interfaces:**

- Consumes: `wx_gdiplus::patch_tree(&Path) -> Result<bool, String>` from Task 1.
- Produces: an extracted wxWidgets tree with GDI+ disabled and Direct2D retained.
- Produces: a Windows CI regression gate that runs the helper tests before the full link.

- [ ] **Step 1: Confirm the failing integration test**

Use the recorded Windows Actions job:

```powershell
gh run view 29796605341 --job 88563009978 --log | Select-String 'LNK2005|LNK1169|GdipAlloc'
```

Expected: the log contains `GdipAlloc already defined`, `LNK2005`, and `LNK1169`.

- [ ] **Step 2: Register the build-support module and shared `Path` import**

Replace the imports at the top of `espanso-modulo/build.rs` with:

```rust
use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
#[path = "build_support/wx_gdiplus.rs"]
mod wx_gdiplus;
```

Remove the old conditional `use std::path::Path;` declaration.

- [ ] **Step 3: Add an `nmake` runner without changing its command contract**

Add this function above the Windows `build_native` function:

```rust
#[cfg(target_os = "windows")]
fn run_nmake(out_wx_dir: &Path, vcvars_path: &Path, target: Option<&str>) {
    let mut command = std::process::Command::new("cmd");
    command
        .current_dir(out_wx_dir.join("build").join("msw"))
        .arg("/k")
        .arg(vcvars_path)
        .args([
            "&",
            "nmake",
            "/f",
            "makefile.vc",
            "BUILD=release",
            "TARGET_CPU=X64",
        ]);
    if let Some(target) = target {
        command.arg(target);
    }
    let mut handle = command
        .args(["&", "exit"])
        .spawn()
        .expect("failed to execute nmake");

    if !handle
        .wait()
        .expect("unable to wait for nmake command")
        .success()
    {
        panic!("nmake {} returned non-zero exit code!", target.unwrap_or("build"));
    }
}
```

- [ ] **Step 4: Separate extraction, patching, and compilation decisions**

Replace the current `if !out_wx_dir.is_dir()` block at lines 50-113 with:

```rust
    let extracted = !out_wx_dir.is_dir();
    if extracted {
        let wx_archive =
            std::fs::File::open(&wx_archive).expect("unable to open wxWidgets source archive");
        let mut archive =
            zip::ZipArchive::new(wx_archive).expect("unable to read wxWidgets archive");
        archive
            .extract(&out_wx_dir)
            .expect("unable to extract wxWidgets source dir");
    }

    let patch_changed = wx_gdiplus::patch_tree(&out_wx_dir)
        .unwrap_or_else(|error| panic!("unable to patch wxWidgets GDI+ backend: {error}"));
    let compiled_dir = out_wx_dir.join("build").join("msw").join("vc_mswu_x64");
    let needs_build = extracted || patch_changed || !compiled_dir.is_dir();

    if needs_build {
        let tool = cc::Build::new().get_compiler();
        assert!(
            tool.is_like_msvc(),
            "The tool found is not of the MSVC family, did you install Visual Studio?"
        );

        let mut vcvars_path = None;
        let mut current_root = tool.path();
        while let Some(parent) = current_root.parent() {
            let target = parent
                .join("VC")
                .join("Auxiliary")
                .join("Build")
                .join("vcvars64.bat");
            if target.exists() {
                vcvars_path = Some(target);
                break;
            }
            current_root = parent;
        }

        let vcvars_path = vcvars_path.expect("unable to find vcvars64.bat file");
        println!("vsvars folder: {}", vcvars_path.display());

        if patch_changed && !extracted {
            run_nmake(&out_wx_dir, vcvars_path, Some("clean"));
        }
        run_nmake(&out_wx_dir, vcvars_path, None);
    }
```

Replace the repeated output-directory construction at lines 118-122 with `compiled_dir.is_dir()`:

```rust
    if !compiled_dir.is_dir() {
        panic!("wxWidgets is not compiled correctly, missing 'build/msw/vc_mswu_x64' directory")
    }
```

- [ ] **Step 5: Add the focused Windows CI gate**

Insert this step after `Format` and before `Build` in `.github/workflows/ci.yml`:

```yaml
      - name: Test wxWidgets GDI+ patch
        run: |
          test_binary="${RUNNER_TEMP}/wx-gdiplus-patch-tests.exe"
          rustc --edition 2021 --test \
            espanso-modulo/build_support/wx_gdiplus.rs \
            -o "$test_binary"
          "$test_binary"
```

- [ ] **Step 6: Run focused and static verification**

```powershell
rustc --edition 2021 --test espanso-modulo/build_support/wx_gdiplus.rs -o "$env:TEMP\wx-gdiplus-patch-tests.exe"
& "$env:TEMP\wx-gdiplus-patch-tests.exe"
cargo fmt --all -- --check
git diff --check
```

Expected: five helper tests pass, formatting passes, and `git diff --check` prints nothing.

- [ ] **Step 7: Build the complete Windows workspace**

```powershell
cargo build
cargo check -p espanso-settings
cargo test -p espanso-settings --no-default-features
cargo clippy -- --deny warnings
```

Expected: every command exits `0`; the link produces `target/debug/espanso.exe` without `LNK2005` or `LNK1169`.

- [ ] **Step 8: Verify the reused-output path**

Run twice with a dedicated build output directory:

```powershell
$env:WX_WIDGETS_BUILD_OUT_DIR = Join-Path $env:TEMP 'espanso-wx-gdiplus-verification'
cargo clean -p espanso-modulo
cargo build
cargo clean -p espanso-modulo
cargo build
Remove-Item Env:WX_WIDGETS_BUILD_OUT_DIR
```

Expected: the first build patches and compiles wxWidgets; the second build accepts the patched tree and links without cleaning or rebuilding wxWidgets for another source change.

- [ ] **Step 9: Run the three required pre-commit checks and commit**

```powershell
git check-ignore -v espanso-modulo/build.rs .github/workflows/ci.yml
git status --short
git diff --stat
git diff -- espanso-modulo/build.rs .github/workflows/ci.yml
git log --oneline origin/dev..HEAD
git add espanso-modulo/build.rs .github/workflows/ci.yml
git commit -m "fix(windows): avoid wxWidgets GDI+ symbol collision"
```

Expected: no modified file is ignored; the diff contains only the approved build and CI changes. The post-fix lesson is handled in Task 3.

---

### Task 3: Prove the fix in Actions and preserve the engineering lesson

**Files:**

- Create: `C:/Users/ZQ/.Codex/skills/knowledge-base/lessons/rust-windows-linking.md`
- Verify: `docs/src/ch02-01-platform-tiers.md:21-31`

**Interfaces:**

- Consumes: the two implementation commits from Tasks 1 and 2.
- Produces: a successful GitHub Actions Windows job and a reusable global lesson.

- [ ] **Step 1: Invoke completion verification before making a success claim**

Use `superpowers:verification-before-completion`. Confirm the local focused tests and all available Cargo commands were run after the final code edit. Do not treat a timeout or an in-progress job as a pass.

- [ ] **Step 2: Push the scoped branch and open a new PR to `dev`**

```powershell
git status --short
git diff --stat origin/dev...HEAD
git log --oneline origin/dev..HEAD
git push origin yxq-windows-gdiplus-link-fix
gh pr create --base dev --head yxq-windows-gdiplus-link-fix --title "fix: resolve Windows GDI+ link collision" --body "Fixes the Windows LNK2005/LNK1169 failure caused by duplicate wxWidgets and windows-rs GDI+ symbols. Disables only the wxWidgets GDI+ fallback, preserves Direct2D, adds guarded patch tests, and verifies the complete Windows build."
```

Expected: the diff contains the existing Rust 1.97 Clippy fix, approved design and plan documents, the patch helper, `build.rs`, and the Windows workflow change. No unrelated commit or file appears.

- [ ] **Step 3: Watch the complete CI run**

```powershell
$pr = gh pr view --json number,url,headRefOid | ConvertFrom-Json
gh pr checks $pr.number --watch --interval 30
gh pr checks $pr.number
```

Expected: `linux-x11`, `linux-wayland`, `macos`, and `windows` all report `pass`. The Windows job's focused test, full build, settings tests, and Clippy steps pass.

- [ ] **Step 4: Inspect the Windows job for direct link evidence**

```powershell
$run = gh run list --branch yxq-windows-gdiplus-link-fix --workflow CI --limit 1 --json databaseId | ConvertFrom-Json
gh run view $run.databaseId --log | Select-String 'Test wxWidgets GDI\+ patch|Compiling espanso v|Finished `dev` profile|LNK2005|LNK1169'
```

Expected: the log contains the focused test and successful Espanso build; it contains no `LNK2005` or `LNK1169`.

- [ ] **Step 5: Complete the mandatory three-part session self-check**

Record these conclusions:

1. **Meta-lesson:** Static C/C++ libraries may define delayed-load wrappers for system APIs that collide with generated Rust import libraries. Resolve ownership at the native-library build boundary; do not suppress duplicate symbols globally.
2. **Manual to tooling:** The exact-marker helper and Windows CI step automate the checks that would otherwise require repeated archive inspection and manual linker-log review.
3. **Assumption revision:** The project support document already limits Windows to 10/11, so removing the pre-Direct2D GDI+ fallback does not require a platform-support edit.

- [ ] **Step 6: Add the global lesson**

Create `C:/Users/ZQ/.Codex/skills/knowledge-base/lessons/rust-windows-linking.md` with this content:

```markdown
# Rust Windows Linking Lessons

## 2026-07-21 — Assign one owner for system API import symbols

### Symptom

MSVC reports `LNK2005` and `LNK1169` when a Rust GUI dependency and a statically linked C++ GUI library both provide symbols such as `GdipAlloc` and `GdiplusStartup`.

### Lesson

A native library may implement delayed-load wrappers for a Windows system API while `windows-rs` supplies a generated import library for the same API. Adding an unrelated GUI crate can change archive extraction order and expose the duplicate definitions.

Fix symbol ownership at the native build boundary. Disable the obsolete wrapper backend when the supported operating systems provide the modern backend. Never use `/FORCE:MULTIPLE` as the default response; it hides which implementation wins.

### Guardrail

Patch vendored source with exact, idempotent marker checks. Fail when the vendor layout changes, and run the patch test before the full Windows link in CI. When a reusable native build directory is supported, force a clean rebuild after the patch first changes it.
```

- [ ] **Step 7: Report the PR and stop before merge**

Report the PR URL, successful CI jobs, implementation commits, and the global lesson path. Do not merge the PR until the user authorizes the merge.
