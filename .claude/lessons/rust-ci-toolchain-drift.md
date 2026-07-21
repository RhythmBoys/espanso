# Rust CI 工具链漂移

### [2026-07-21] 浮动 stable 下目标 crate 通过不代表 workspace 通过

- **问题**: Settings 相关 crate 的严格 Clippy 已通过，但 CI 升级到 Rust 1.97 后，workspace 在多个旧模块连续触发新增格式 lint，并因 `--deny warnings` 失败。
- **根因**: `rust-toolchain.toml` 跟随 `stable`，Clippy 规则会随工具链变化；此前验证只覆盖目标 crate，没有执行 CI 中实际的 workspace 命令。
- **解决**: 使用与 CI 一致的 stable toolchain、features 和 `--all-targets` 运行完整 Clippy，按机器可应用建议统一修复，再运行格式检查与 workspace 测试。
- **预防**: 修改 workspace 或依赖图后，以 `.github/workflows/ci.yml` 中的原始命令作为发布门禁；定向检查只能用于快速反馈，不能替代最终 workspace 验证。
- **关键词**: Rust, Clippy, stable, toolchain drift, workspace, deny warnings, CI parity
