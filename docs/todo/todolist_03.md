## 原始输入（原文，勿改）
- 1.选择配置文件夹的时候,文件夹中已经有之前的配置,需要能够反向导入到配置中,而不是不允许有配置的文件夹导入
- 2.同时配置文件也回显到配置编辑上

> 状态: ✅ 已完成 (2/2)
> 分支: `yxq-settings-adopt-existing-config`
> 更新: 2026-08-12
> 设计: `docs/plans/2026-08-12-adopt-existing-config-and-echo-matches-design.md`

- ✅ 1.选择配置文件夹的时候,文件夹中已经有之前的配置,需要能够反向导入到配置中,而不是不允许有配置的文件夹导入
- ✅ 2.同时配置文件也回显到配置编辑上

## 说明与上下文（完成后补）

- 做了什么：
  - 第 1 条：`...` 不再要求目录为空，改为按目录状态自动分流。新增 `AdoptService::plan`：空/不存在 → 沿用 `ScaffoldService` 生成默认配置；已有 espanso 配置（含 `config/` 且 `espanso_config::load` 通过）→ **接纳，一个字节都不写**；非空且不像配置 → 拒绝并说明缺什么。接纳时用 `AdoptService::inspect` 统计 config 数 / match 数，非致命错误降级为警告不阻断。
  - 第 2 条：新增 `inventory::scan_external_matches`，扫描 `match/**/*.{yml,yaml}`（排除自有的 `ui.yml`），把手写配置里的匹配全部回显。外部条目只读、带来源徽标与只读原因（regex / form / variables / image 分别给出具体原因），配 `Open file` 打开源文件。同时把仓库与活动路径改为可原地替换，切目录后编辑器立即刷新，不必重开 Settings。
- 关键决策：
  - **接纳路径零写入**，这是它能免掉确认框的依据：失败了原目录没有任何变化。
  - **外部文件只读，不回写**。调研三条硬证据：Rust 没有成熟的保注释 YAML 往返库（serde_yaml 停维、yaml-rust2 明确不保留注释、saphyr 未落地）；espanso 匹配支持 regex/form/vars/image，远超 `trigger+replace`；已有第三方 espanso GUI 就因为回写而在 README 里警告"保存会丢失既有 YAML 内容"。采用 VS Code 的 "Edit in settings.json" 逃生口模式而非 Home Assistant 的回写模式。
  - **两个列表在模型层分开**（`matches` / `external`）。保存只遍历 `matches`，所以"把 base.yml 误抄进 ui.yml"在数据结构上就不可能发生，而不是靠记得别犯。
  - **不让用户先声明意图**（对比 Cryptomator 的三入口）：espanso 配置目录的状态完全能由磁盘内容判定，不存在歧义，多问一步只是摩擦。
  - 接纳用的校验比 `EspansoConfigValidator` 宽松是有意为之：后者守的是我们刚写出的目录（有任何毛病都是我们的 bug），前者守的是用户手写了很久的目录（error 阻断、warning 放行）。
- 涉及文件：`espanso-settings/src/{adopt.rs, inventory.rs, app.rs, model.rs, lib.rs}`、`espanso-settings/ui/settings.slint`、`espanso-settings/tests/{adopt.rs, inventory.rs}`、`docs/plans/2026-08-12-adopt-existing-config-and-echo-matches-design.md`。
- 验证结果：`cargo fmt --all --check` 通过；`cargo clippy -p espanso-settings --all-targets` 在**含 UI 与 headless 两套 feature 下均零警告**；`cargo test -p espanso-settings --no-default-features` 29 项全过（新增 adopt 6 项、inventory 3 项）；`cargo check -p espanso` 通过。含 UI feature 时集成测试无法链接（slint 相关 crate 缺 rlib），是本机既有环境限制，与本次改动无关。
- 风险与后续：
  - 顺带修掉一个既有崩溃：`on_commit_deletion` 的 match 判别式临时量存活到整个 match 块，导致 `Ok` 分支里 `borrow_mut()` 撞上未释放的 `Ref`，点「Save deletion」必 panic。已用最小复现双向验证（原形态 panic、提到 `let` 后正常）。详见设计文档 §4.6。
  - 三端真实窗口下的观感与 `Open file` 行为仍需人工冒烟，未执行。
  - 外部匹配的原地编辑需要保注释的 YAML span 级修改（`yamlpath` / `noyalib`），或等 `saphyr` 的注释支持落地，见设计文档 §9。
  - 迁移流程（`Choose new directory...` → `Copy and switch`）完成后仍不就地刷新编辑器，原因是回调需 `Send`，见设计文档 §9。
