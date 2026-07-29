# Windows 文件持久化与只读句柄

### [2026-07-29] `sync_all()` 在只读句柄上必然失败（os error 5）

- **问题**: `dev-release.yml` 的 Windows job 里 `espanso-settings` 的 4 个 `tests/matches.rs` 集成测试全部失败，报 `called Result::unwrap() on an Err value: Access is denied. (os error 5)`，且都停在第一次 `repository.save()`。Linux/macOS 全绿。
- **根因**: `espanso-settings/src/migration.rs` 的 `copy_tree` 用 `File::open(&target)?.sync_all()?` 做落盘。`File::open` 拿到的是**只读句柄**；Windows 上 `sync_all` 映射到 `FlushFileBuffers`，该 API 要求句柄具备 `GENERIC_WRITE`，否则返回 `ERROR_ACCESS_DENIED`。Unix 的 `fsync` 允许在 O_RDONLY fd 上调用，所以只有 Windows 暴露。
- **实证**: P/Invoke 直接验证，只读句柄 `FlushFileBuffers -> ok=False lastError=5`，写句柄 `ok=True lastError=0`。
- **解决**: 改用 `File::options().write(true).open(&target)`，并给 open 和 sync_all 都加 `with_context`。
- **预防**:
  - 任何 `sync_all()` / `sync_data()` 之前，确认句柄是写句柄。`fs::copy` 之后必须重新以写模式打开，不能用 `File::open`。
  - **裸 `?` 会吃掉现场**。这次 panic 只有一行 `Access is denied. (os error 5)`、没有 `Caused by`，正是因为该 `?` 没有 context——反过来也成了定位线索：有 context 的 `?` 可以直接排除。文件系统操作一律加 `with_context` 并带上路径。
  - 遗留风险：`fs::copy` 在 Windows 会连只读属性一起复制。若用户的配置文件本身是只读的，重开写句柄仍会 EACCES。现在错误会指名具体文件，不再是裸报错。
- **关键词**: Windows, sync_all, FlushFileBuffers, GENERIC_WRITE, ERROR_ACCESS_DENIED, os error 5, fsync, anyhow context

### [2026-07-29] `ci.yml` 绿不等于发布工作流绿

- **问题**: 上述 bug 由 `dev-release.yml` 暴露，但 `ci.yml` 的 windows job 同样跑 `cargo test -p espanso-settings --no-default-features`，本应更早红。
- **根因**: 两个工作流的测试矩阵不同。`ci.yml` 用 debug + 单 crate；`dev-release.yml` 用 `--release --workspace --exclude espanso-modulo --exclude espanso-ipc --no-default-features --features native-tls`，**不排除 `espanso-settings`**。把「CI 门禁」等同于 `ci.yml` 会漏掉发布路径。
- **预防**: 门禁清单要覆盖 `ci.yml` **和** `dev-release.yml` / `create-release-draft.yml` 的对应平台 job。见 `.claude/skills/yxq-espanso-build-test/references/ci-commands.md`。
- **关键词**: CI parity, dev-release, release workflow, test matrix drift
