# Cross-Platform Settings and Match Editor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 Espanso 增加 Windows、macOS、Linux 三端一致的 Settings 窗口，支持安全选择和迁移配置目录，并通过图形界面管理触发文本与长替换文本。

**Architecture:** 新增 `espanso-settings` crate，使用 Slint 1.17.1 构建同一套跨平台界面，并由现有 `espanso` 主程序通过隐藏 `settings` 子命令启动。路径选择与迁移、YAML 规则读写、单实例控制均放在可独立测试的 Rust 服务层；UI 只消费状态并发出命令。配置目录优先级固定为 CLI > 环境变量 > 持久化选择 > 平台默认值。

**Tech Stack:** Rust 2021、Slint 1.17.1、slint-build 1.17.1、rfd 0.17.2、serde/serde_json、serde_norway、tempdir、fs_extra、现有 espanso-config/espanso-ipc。

**Implementation status (2026-07-21):** 原始三项功能已实现；Rust 1.97 Linux ARM64 环境下，CI 等价的 workspace Clippy、格式检查、workspace 测试、Settings 检查与测试均已通过。Windows/macOS/Linux 原生 UI、IME、DPI、主题和安装包仍需在对应平台执行发布前人工验收。下方清单保留原始细粒度范围，未实现的增强项不作虚假勾选。

## Global Constraints

- Windows、macOS、Linux 首版功能与信息架构一致；平台差异只允许存在于目录选择器、窗口激活、配置定位和安装包依赖。
- 沿用项目 GPL-3.0；Slint 按 GPLv3 兼容方式使用，不采用要求额外署名的 Royalty-Free 路径。
- 不引入 Node.js、WebView、HTML/CSS 或第二套前端构建链。
- 不修改、格式化或重写用户已有的任意 YAML；图形界面首版只写 `match/ui.yml`。
- 配置迁移只复制，不自动删除旧目录；验证新目录成功后才切换持久化路径。
- 迁移过程中发生任何错误时，当前运行路径与旧目录保持不变。
- 所有持久化写入采用“同目录临时文件 → flush/sync → rename”的原子模式。
- 禁止把配置内容、trigger、replace、完整路径或用户文本写入遥测；错误日志只记录阶段和脱敏路径类别。
- 长替换文本至少支持 100,000 个 Unicode scalar values，保留换行、emoji 和中日韩字符。
- UI 最小窗口 820×560，默认 1040×720；键盘可完成选项卡切换、表单编辑、保存和取消。

---

## Research Basis and Decisions

- 当前 `espanso-ui` 只提供托盘、通知和菜单抽象，没有设置窗口或表单框架。
- 核心已支持隐藏参数 `--config_dir` 和环境变量 `ESPANSO_CONFIG_DIR`；子进程也会传播显式路径覆盖。
- `espanso-config` 已负责加载 `config/` 与 `match/`，迁移后验证必须复用它，不能另写一套 YAML 校验器。
- Slint 官方文档明确支持 Windows、macOS 与 Linux 桌面；1.17.1 提供 Rust 生成组件和多行 `TextEdit`。
- rfd 0.17.2 提供 Windows、macOS、Linux 原生文件夹选择；Linux 默认采用 XDG Portal 后端，避免强绑定 GTK。
- 外部社区经验仅用于发现风险；架构结论以本仓库代码和官方文档为准。

Primary references:

- https://docs.slint.dev/latest/docs/slint/guide/platforms/desktop/
- https://docs.slint.dev/latest/docs/slint/reference/std-widgets/views/textedit/
- https://docs.slint.dev/latest/docs/rust/slint/
- https://docs.rs/rfd/0.17.2/rfd/
- https://github.com/espanso/espanso/issues/2223

## File Map

| Path | Action | Responsibility |
|---|---|---|
| `Cargo.toml` | Modify | 注册 `espanso-settings` workspace member 和共享依赖 |
| `Cargo.lock` | Modify | 锁定 Slint/rfd 依赖图 |
| `espanso-settings/Cargo.toml` | Create | Settings crate 依赖与 features |
| `espanso-settings/build.rs` | Create | 编译 `ui/settings.slint` |
| `espanso-settings/ui/settings.slint` | Create | 窗口、选项卡、设置页、规则编辑器与状态提示 |
| `espanso-settings/src/lib.rs` | Create | 对外 `run(SettingsLaunchOptions)` 入口 |
| `espanso-settings/src/model.rs` | Create | UI 状态、消息与可测试 update reducer |
| `espanso-settings/src/location.rs` | Create | 持久化配置位置与优先级解析 |
| `espanso-settings/src/migration.rs` | Create | 预检、复制、验证、切换与回滚 |
| `espanso-settings/src/matches.rs` | Create | `match/ui.yml` 领域模型、校验与原子读写 |
| `espanso-settings/src/single_instance.rs` | Create | 设置窗口锁与聚焦请求 |
| `espanso-settings/tests/location.rs` | Create | 路径优先级和持久化测试 |
| `espanso-settings/tests/migration.rs` | Create | 迁移故障注入与回滚测试 |
| `espanso-settings/tests/matches.rs` | Create | YAML、Unicode、重复 trigger 和原文保护测试 |
| `espanso/src/main.rs` | Modify | 注册隐藏 `settings` 子命令并解析路径 |
| `espanso/src/path/mod.rs` | Modify | 在既有路径解析中接入持久化配置目录优先级 |
| `espanso/src/ipc.rs` | Modify | 增加 `OpenSettings` 事件 |
| `espanso/src/cli/worker/ipc.rs` | Modify | 将 `OpenSettings` 转给 worker engine |
| `espanso/src/cli/worker/engine/mod.rs` | Modify | 注入 Settings launcher |
| `espanso/src/cli/worker/engine/dispatch/executor/context_menu.rs` | Modify | 托盘菜单动作启动/聚焦 Settings |
| `espanso-engine/src/event/ui.rs` | Modify | 定义 Settings 菜单动作 |
| `.github/workflows/ci.yml` | Modify | 三平台构建与测试 Settings crate |
| `docs/todolist_01.md` | Modify | 保留原始需求并追踪实施状态 |

## Public Interfaces

```rust
pub struct SettingsPaths {
    pub config: PathBuf,
    pub packages: PathBuf,
    pub runtime: PathBuf,
}

pub struct SettingsLaunchOptions {
    pub paths: SettingsPaths,
    pub config_override_source: ConfigPathSource,
}

pub fn run(options: SettingsLaunchOptions) -> anyhow::Result<()>;

pub enum ConfigPathSource {
    Cli,
    Environment,
    Persisted,
    PlatformDefault,
}

pub struct ConfigLocationStore {
    bootstrap_path: PathBuf,
}

impl ConfigLocationStore {
    pub fn load(&self) -> anyhow::Result<Option<PathBuf>>;
    pub fn save_atomic(&self, path: &Path) -> anyhow::Result<()>;
    pub fn clear(&self) -> anyhow::Result<()>;
}

pub struct MigrationPlan {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub file_count: usize,
    pub byte_count: u64,
    pub conflicts: Vec<PathBuf>,
}

pub trait ConfigValidator {
    fn validate(&self, root: &Path) -> anyhow::Result<()>;
}

pub struct UiMatch {
    pub id: String,
    pub trigger: String,
    pub replace: String,
    pub enabled: bool,
}

pub trait MatchRepository {
    fn load(&self) -> anyhow::Result<Vec<UiMatch>>;
    fn save(&self, matches: &[UiMatch]) -> anyhow::Result<()>;
}
```

## Task 1: Scaffold the cross-platform Settings crate

**Files:** `Cargo.toml`, `Cargo.lock`, `espanso-settings/Cargo.toml`, `espanso-settings/build.rs`, `espanso-settings/src/lib.rs`, `espanso-settings/ui/settings.slint`.

- [ ] Add a workspace member named `espanso-settings`.
- [ ] Pin `slint = "=1.17.1"`, `slint-build = "=1.17.1"`, and `rfd = { version = "=0.17.2", default-features = false, features = ["xdg-portal"] }`.
- [ ] Add path dependencies on `espanso-config` and `espanso-ipc`; reuse workspace serde, serde_json, anyhow, thiserror, tempdir, and fs_extra. Keep `espanso-settings` independent from the binary crate by mapping `espanso::path::Paths` into the public `SettingsPaths` DTO in `espanso/src/main.rs`.
- [ ] Write a failing smoke test that calls the public crate API and proves `SettingsLaunchOptions` is constructible without opening a window.
- [ ] Implement the smallest crate API and a placeholder Slint window with the title `Espanso Settings`.
- [ ] Run `cargo test -p espanso-settings`; expect all crate tests to pass.
- [ ] Run `cargo check -p espanso-settings` on Windows and the existing Linux CI target.

## Task 2: Add persisted configuration-location resolution

**Files:** `espanso-settings/src/location.rs`, `espanso-settings/tests/location.rs`, `espanso/src/path/mod.rs`, `espanso/src/main.rs`.

- [ ] Write tests for the exact precedence: CLI > `ESPANSO_CONFIG_DIR` > persisted file > platform default.
- [ ] Store the persisted selection in the stable default config parent, not inside the movable directory: `settings-location.json` containing `{"version":1,"config_dir":"..."}`.
- [ ] Reject empty, relative, nonexistent parent, and non-directory paths; canonicalize with the project-compatible path normalization strategy.
- [ ] Implement atomic save using a sibling `settings-location.json.tmp` and rename.
- [ ] Change path resolution to accept persisted location only when neither CLI nor environment override is present.
- [ ] Display the active source in the UI: `命令行覆盖`, `环境变量覆盖`, `自定义目录`, or `系统默认`.
- [ ] When CLI/environment override is active, disable the permanent-directory Save action and explain why.
- [ ] Run targeted tests and existing path-resolution tests.

## Task 3: Build a transactional migration engine

**Files:** `espanso-settings/src/migration.rs`, `espanso-settings/tests/migration.rs`.

- [ ] Define `MigrationPlan` and write tests for file count, byte count, source equals destination, nested destination, unwritable destination, symlink/reparse-point handling, and filename conflicts.
- [ ] Reject a destination inside the source and a source inside the destination.
- [ ] Treat symlinks/reparse points as conflicts in v1; do not follow them silently.
- [ ] Require destination either empty or newly created; do not merge with existing unrelated files.
- [ ] Copy into `<destination>.espanso-migration-<uuid>` while the current worker keeps using the old directory.
- [ ] Flush copied files, then call `espanso_config::load()` against the staged root.
- [ ] Rename the staged root to the chosen destination only after validation succeeds.
- [ ] Save the new persistent location atomically; if saving fails, keep the application on the old path and retain the validated copy for recovery.
- [ ] Request a controlled stop/start instead of relying on `restart`; show progress stages and a final restart-required state.
- [ ] Never delete the source. Show `旧目录已保留` with an `在文件管理器中打开` action.
- [ ] Add fault-injection tests after directory creation, mid-copy, validation, rename, and persistent-location save.

## Task 4: Implement the Settings window shell and navigation

**Files:** `espanso-settings/ui/settings.slint`, `espanso-settings/src/model.rs`, `espanso-settings/src/lib.rs`.

- [ ] Write reducer tests for tab switching, dirty-state confirmation, migration progress, error dismissal, and window-close behavior.
- [ ] Build a two-tab layout: `设置` and `配置`; preserve selected tab while the window stays open.
- [ ] Provide a consistent header with current config path, Espanso status, and non-blocking error banner.
- [ ] Use responsive breakpoints: two-column content at ≥960 px, one-column at smaller sizes.
- [ ] Add visible focus styling, logical Tab order, `Ctrl/Cmd+S` save, `Ctrl/Cmd+F` search, and `Esc` cancel/close dialog.
- [ ] Add unsaved-changes confirmation before tab switch, path migration, reload, or window close.
- [ ] Keep long operations off the Slint event loop; send progress back with `invoke_from_event_loop` and weak component handles.
- [ ] Verify Chinese, English, emoji, 125%/150% DPI, light theme, and dark theme rendering.

## Task 5: Implement the configuration-directory UX

**Files:** `espanso-settings/ui/settings.slint`, `espanso-settings/src/lib.rs`, `espanso-settings/src/migration.rs`.

- [ ] Show current path in a selectable read-only field with `打开目录` and `复制路径` actions.
- [ ] Use `rfd::FileDialog::pick_folder()` for native directory selection.
- [ ] After selection, run preflight without mutating disk and show source, destination, file count, size, conflicts, and warnings.
- [ ] Require explicit `复制并切换` confirmation; do not label the operation merely `移动`.
- [ ] Render progress as named phases: 检查、复制、验证、切换、等待重启.
- [ ] On failure, show the exact safe state: current path unchanged, source retained, staged-copy location if available.
- [ ] On success, show new active path and old retained path, then offer controlled restart.
- [ ] Add manual test cases for canceling the picker, destination permission denial, full disk simulation, invalid YAML, and process interruption.

## Task 6: Implement the UI-managed match repository

**Files:** `espanso-settings/src/matches.rs`, `espanso-settings/tests/matches.rs`.

- [ ] Define the only writable file as `<active-config>/match/ui.yml`.
- [ ] Use a stable document shape:

```yaml
# Managed by Espanso Settings. Advanced rules may be edited in other files.
matches:
  - trigger: ":hello"
    replace: |-
      Hello from Espanso
```

- [ ] Write failing round-trip tests for multiline text, CRLF/LF, quotes, colon prefixes, emoji, Chinese, 100,000-character replacements, and empty final lines.
- [ ] Reject blank triggers, triggers containing newline/NUL, exact duplicates, and duplicate normalized triggers inside `ui.yml`.
- [ ] Load all match files through `espanso-config` for global duplicate warnings, but never rewrite other files.
- [ ] Preserve the last valid `ui.yml` when parsing or validation fails.
- [ ] Serialize to a sibling temporary file, validate the complete config root, flush, and atomically rename.
- [ ] Add a backup `ui.yml.backup` before replacing a pre-existing valid file.
- [ ] Verify the daemon watcher observes one final rename event rather than a stream of partial writes.

## Task 7: Build the match editor UX

**Files:** `espanso-settings/ui/settings.slint`, `espanso-settings/src/model.rs`, `espanso-settings/src/lib.rs`.

- [ ] Show a searchable list with trigger, replacement preview, enabled state, and source badge.
- [ ] Make rules outside `ui.yml` read-only and provide `打开源文件` instead of editing them.
- [ ] Provide `新增规则`, `编辑`, `复制`, and `删除` for UI-managed rules.
- [ ] Use a single-line trigger field and a resizable multiline replacement editor.
- [ ] Show character count, newline count, duplicate warnings, and a literal preview that preserves whitespace.
- [ ] Save only when validation succeeds; show inline field errors and retain the unsaved draft after failure.
- [ ] Confirm deletion by trigger name; support undo until the next successful save.
- [ ] Filter 1,000 rules within 100 ms on a representative release build.
- [ ] Add reducer tests for draft editing, filter, duplicate error, save failure, delete/undo, and dirty navigation.

## Task 8: Connect tray menu, IPC, and single-instance behavior

**Files:** `espanso/src/main.rs`, `espanso/src/ipc.rs`, `espanso/src/cli/worker/ipc.rs`, `espanso/src/cli/worker/engine/mod.rs`, `espanso/src/cli/worker/engine/dispatch/executor/context_menu.rs`, `espanso-engine/src/event/ui.rs`, `espanso-settings/src/single_instance.rs`.

- [ ] Add a `Settings…` item to the default tray menu before `Open Config Folder`.
- [ ] Add `IPCEvent::OpenSettings`; test serialization compatibility and exhaustive matching.
- [ ] Add hidden `espanso settings` subcommand with required paths but without loading worker-only engine state.
- [ ] On first launch, acquire a lock in the runtime directory and open the Slint window.
- [ ] On repeated launch, send a focus request to the existing Settings process and exit 0.
- [ ] Remove stale locks only after proving no Settings IPC endpoint is reachable.
- [ ] Propagate active config/package/runtime overrides to the Settings child process.
- [ ] Test click → event → process launch and click-again → focus behavior without opening a real window.

## Task 9: Cross-platform hardening

**Files:** `.cargo/config.toml` if absent and required, `espanso-settings/Cargo.toml`, platform packaging files, `.github/workflows/ci.yml`.

- [ ] On Windows, suppress the release console window and apply Slint's documented 8 MB stack only if a debug stack-overflow reproduction proves it necessary.
- [ ] On macOS, verify Settings launches as a foreground window without creating an extra persistent Dock identity after close.
- [ ] On Linux, test X11 and Wayland; use XDG Portal for the folder picker and document required D-Bus/runtime packages.
- [ ] Verify IME composition for Chinese/Japanese/Korean on all three platforms.
- [ ] Verify native file picker cancellation and parent-window focus restoration.
- [ ] Add CI jobs for `cargo check -p espanso-settings`, unit tests, Clippy, and Slint compilation on all supported OS runners.
- [ ] Measure release binary delta and cold-open time; record results in the implementation report without inventing a hard size target.

## Task 10: End-to-end acceptance and rollout

**Files:** `docs/todolist_01.md`, implementation report under `plan/`, release notes if the project uses them.

- [ ] Create a temporary Espanso config containing basic, multiline, Unicode, and invalid rules.
- [ ] Verify Settings opens from tray and CLI on Windows, macOS, Linux.
- [ ] Create/edit/delete a UI-managed rule and confirm actual text expansion after watcher reload.
- [ ] Migrate the temporary config, confirm all rules still expand, and confirm the old directory remains untouched.
- [ ] Inject migration failure and confirm the active path stays on the source.
- [ ] Verify CLI and environment overrides visibly win and disable persistent switching.
- [ ] Run `cargo fmt --check`.
- [ ] Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo test --workspace`.
- [ ] Run platform packaging builds and three-platform smoke tests.
- [ ] Update the task file only after evidence proves the feature works; append changed files, decisions, test output, risks, and rollback instructions.

## User Experience Acceptance Criteria

- A first-time user can find Settings from the tray without knowing the CLI or filesystem layout.
- The UI always distinguishes “current directory”, “selected destination”, and “old retained directory”.
- No migration button is enabled until preflight succeeds and the user reviews the summary.
- An interruption before final path save cannot change the active configuration location.
- A user can create a basic trigger/replacement rule without learning YAML.
- Advanced/manual YAML remains available and untouched.
- Invalid input produces an actionable inline message and never destroys the last valid file.
- Closing or switching tabs with an unsaved draft always asks before discarding.
- Keyboard, IME, scaling, long text, and Unicode work consistently across all three operating systems.

## Rollback

1. Disable the `Settings…` menu item and hidden subcommand without changing the text-expansion engine.
2. Clear `settings-location.json` to return to normal platform path resolution.
3. Retain the user-selected directory and original directory; never delete either during rollback.
4. Restore `match/ui.yml.backup` if a rules-file regression is confirmed.
5. Remove `espanso-settings` from the workspace and lockfile only after confirming no persisted path depends on the new bootstrap file.
