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

### [2026-07-29] 别把 `\\?\` verbatim 路径交给 espanso-config

- **问题**: `espanso_validator_rejects_non_fatal_yaml_errors_before_publish` 在 Windows release 构建下 panic：`end byte index 18446744073709551613 is out of bounds for string of length 170`。
- **根因**: `espanso-settings` 用了 `std::path::Path::canonicalize()`，Windows 上返回 `\\?\C:\...` verbatim 路径。espanso-config 的 `STANDARD_INCLUDES` 是 `"../match/**/[!_]*.yml"`，含 `..`。glob 0.3.0 这样算根偏移：
  ```rust
  let rest = components.map(|s| s.as_os_str()).collect::<PathBuf>();      // 空起点，不折叠 ..
  let normalized_pattern = Path::new(pattern).iter().collect::<PathBuf>(); // verbatim 起点，折叠 ..
  let root_len = normalized_pattern.len() - rest.len();                    // 下溢 3 字节
  Some(Path::new(&pattern[..root_len]))                                    // panic
  ```
  `PathBuf::push` 只在 self 带 verbatim 前缀时移除 `..`，两边长度因此差 3（`\..`）。release 构建整数溢出回绕成 `usize::MAX - 2` = 18446744073709551613，与报错数字精确吻合。
- **解决**: 全部改用 `dunce::canonicalize`（espanso-config 早就这么做，dunce 存在的意义就是不返回 verbatim 路径）。`tests/location.rs` 里对 `Path::canonicalize()` 的断言同步改掉，并加了「解析结果不得以 `\\?\` 开头」的守卫。
- **预防**:
  - **本仓库任何要交给 `espanso_config::load` 的路径，一律 `dunce::canonicalize`，不要用 `Path::canonicalize`。**
  - 断言别用被测代码换过的那个 API 去反推期望值——`assert_eq!(resolved, x.canonicalize())` 把 verbatim 形式固化成了契约，改用 dunce 时才暴露。
  - 残留风险：路径 ≥260 字符时 dunce 仍会返回 verbatim 形式。
  - 只在 Windows + release 复现：debug 下整数溢出会 panic 在减法处，报错完全不同。
- **关键词**: glob 0.3.0, verbatim path, `\\?\`, dunce, canonicalize, root_len underflow, release overflow wrap

### [2026-07-29] 文件锁竞争的错误码不跨平台统一

- **问题**: `only_one_settings_instance_can_hold_the_runtime_lock` 在 Windows 报 `The process cannot access the file because another process has locked a portion of the file. (os error 33)`，第二次 `try_acquire` 返回 `Err` 而不是预期的 `Ok(None)`。
- **根因**: `single_instance.rs` 用 `error.kind() == ErrorKind::WouldBlock` 判断"锁已被占用"。Unix 的 flock 返回 `EWOULDBLOCK`（确实映射为 `WouldBlock`），Windows 的 `LockFileEx` + `LOCKFILE_FAIL_IMMEDIATELY` 返回 `ERROR_LOCK_VIOLATION`（33），而 std 不把它映射为 `WouldBlock`，于是 guard 不匹配、错误穿透。
- **解决**: 改用 fs2 文档指定的方式比较 `error.raw_os_error() == fs2::lock_contended_error().raw_os_error()`（保留 `WouldBlock` 分支作冗余）。`try_lock_exclusive` 的文档原文就是 "returns an error if the file is currently locked (see `lock_contended_error`)"。
- **预防**:
  - **不要用 `io::ErrorKind` 判断平台相关的失败原因**。std 的 `decode_error_kind` 映射表是不完整且平台特定的。库若提供了「哨兵错误」构造函数（如 `lock_contended_error()`），用它。
  - 参照物：`espanso/src/lock.rs` 用的是 `try_lock_exclusive().is_ok()`，绕开了该问题（代价是把真实 IO 错误也当成"已被锁"）。
- **关键词**: fs2, LockFileEx, ERROR_LOCK_VIOLATION, os error 33, ErrorKind::WouldBlock, lock_contended_error

### [2026-07-29] espanso-settings 整个 crate 从未在 Windows 上跑过

- **现象**: 连续三轮 CI，每轮暴露一个此前从未触发的 Windows 专属缺陷（`sync_all` 只读句柄 → glob verbatim 路径 → 文件锁错误码），三者互不相关，且都在 `espanso-settings`。
- **判断**: 不是巧合。这个 crate 的测试此前只在 Linux/macOS 验证过；`ci.yml` 的 windows job 虽然有 `cargo test -p espanso-settings --no-default-features`，但这些用例显然从未在该平台绿过。
- **预防**: 新增跨平台 crate 时，**首次提交前就要在每个目标平台跑完整测试套件**，而不是靠 CI 一轮抓一个——每轮往返成本极高（Windows job 含 wxWidgets 构建）。本地 Windows 一次性跑 `cargo test -p espanso-settings --no-default-features` 可在一次内暴露全部。
- **关键词**: 跨平台验证, 平台覆盖缺口, CI 往返成本

### [2026-07-29] `ci.yml` 绿不等于发布工作流绿

- **问题**: 上述 bug 由 `dev-release.yml` 暴露，但 `ci.yml` 的 windows job 同样跑 `cargo test -p espanso-settings --no-default-features`，本应更早红。
- **根因**: 两个工作流的测试矩阵不同。`ci.yml` 用 debug + 单 crate；`dev-release.yml` 用 `--release --workspace --exclude espanso-modulo --exclude espanso-ipc --no-default-features --features native-tls`，**不排除 `espanso-settings`**。把「CI 门禁」等同于 `ci.yml` 会漏掉发布路径。
- **预防**: 门禁清单要覆盖 `ci.yml` **和** `dev-release.yml` / `create-release-draft.yml` 的对应平台 job。见 `.claude/skills/yxq-espanso-build-test/references/ci-commands.md`。
- **关键词**: CI parity, dev-release, release workflow, test matrix drift
