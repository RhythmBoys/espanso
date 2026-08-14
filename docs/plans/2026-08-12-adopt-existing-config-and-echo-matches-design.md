# 导入已有配置目录与配置回显设计

> 日期: 2026-08-12
> 风险: medium（触及配置目录切换与 match 文件读取，但**不新增任何对外部文件的写入**）
> 关联: `docs/todo/todolist_03.md`、`docs/plans/2026-08-05-settings-theme-and-config-bootstrap-design.md`、`docs/plans/2026-07-20-cross-platform-settings-design.md`

## 1. 目标

对应 `todolist_03.md` 两条：

| # | 原始输入 | 归类 |
|---|---------|------|
| 1 | 选择配置文件夹的时候,文件夹中已经有之前的配置,需要能够反向导入到配置中,而不是不允许有配置的文件夹导入 | 目录选择：接纳已有配置 |
| 2 | 同时配置文件也回显到配置编辑上 | 配置回显：已有 match 显示到编辑器 |

两条是同一条用户旅程的前后半段：**选中一个已有配置目录 → 立即在 Configuration 页看到里面的内容**。分开做会得到一个"导入成功但编辑器空白"的半成品，所以合并为一次设计。

## 2. 现状与根因

### 2.1 第 1 条：`...` 只接受空目录

`ScaffoldService::preflight`（2026-08-05 引入）对非空目录直接 `bail!`：

```rust
if fs::read_dir(&destination)?.next().is_some() {
    bail!("the selected directory is not empty; create a new empty folder and select it");
}
```

当时的判断是对的——那一条需求的字面意思是"选择(必须是空)完成后,自动生成默认配置"，且**在空目录里写文件**这个动作确实必须保证目录为空。但它把"我要在这里新建配置"和"我已经有配置，指过去就行"两种意图压成了一个入口，于是后者被误伤。

关键区别：**接纳已有目录根本不需要写任何文件**，因此"目录必须为空"这个前置条件对它不成立。

### 2.2 第 2 条：编辑器只读 `match/ui.yml`

`UiMatchRepository::new(config_root)` 把路径钉死为 `config_root/match/ui.yml`，`load()` 在文件不存在时返回空数组：

```rust
if !self.path.exists() {
    return Ok(Vec::new());
}
```

所以任何一个由用户手写的既有配置目录（只有 `base.yml`、`personal.yml`……）在 Configuration 页永远是空的。

还有第二个断点：`UiMatchRepository` 在 `app::run` 启动时构造一次，`bind_location` 里切换目录后**不会重建**，因此即使目录切过去了，编辑器仍然读旧目录。

## 3. 调研（deep 三轮，见 §8）

本机 SearXNG 与私有 firecrawl 均不可达，降级至 WebSearch + 官方文档抓取（与 2026-08-05 同样的降级路径）。

三条直接决定方案的结论：

1. **接纳已有目录是成熟桌面模式，且不应改动目录内容。** Obsidian 的 "Open folder as vault" 明确支持非空目录，原有文件原地保留、不复制不移动。Cryptomator 更进一步，把 Open Existing / Create New / Recover 做成三个显式入口。
2. **GUI 不该回写自己无法表达的结构。** VS Code 对 GUI 无法呈现的复杂类型不给控件，改为 "Edit in settings.json" 跳转到源文件。反面教材是 Home Assistant：可视化编辑器回写 YAML 会清掉它不理解的模板内容。
3. **Rust 目前没有可用的保注释 YAML 往返方案。** `serde_yaml` 已停止维护并拒绝了该特性，`yaml-rust2` 明确不保留注释，后继者 `saphyr` 的注释支持仍在计划中。本仓库用的 `serde_norway` 同样是"解析成值再重新序列化"，注释、空行、键顺序一律丢失。

三条合起来指向一个硬结论：**Settings 可以读全部 match 文件，但只能回写自己创建的那一个。** 这不是保守，是有前车之鉴——已有的第三方 espanso GUI 在 README 里就写着"保存会丢失部分既有 YAML 内容，请先备份 match 目录"。我们不能重蹈。

## 4. 方案

### 4.1 第 1 条：`...` 按目录状态自动分流

不给用户加一道"你要新建还是导入"的选择题。用户选目录时心里已经有答案，目录本身也已经把答案写在磁盘上了；再问一遍是把系统能自己判断的事推给用户。改为**探测 → 分流 → 明确告知做了什么**：

| 选中目录的状态 | 动作 | 是否写入磁盘 | 是否需要确认 |
|---|---|---|---|
| 不存在 / 空 | 生成默认配置（沿用 `ScaffoldService`） | 是（仅新建文件） | 否，目录为空，破坏性为零 |
| 含 `config/` 且 `espanso_config::load` 通过 | **接纳**：只把它记为当前配置目录 | **否** | 否，零写入就没有可后悔的事 |
| 非空但不像 espanso 配置 | 拒绝，说明原因与下一步 | 否 | — |

"零写入"是这套流程能免掉确认框的**依据**，不是巧合：接纳路径全程只读磁盘，失败了原目录一个字节都没变。

入口文案同步改成描述真实行为的 "Choose configuration folder..."。调研里 KeePassDX #1714 就是"open existing vault"名不副实导致的困惑，值得避开。

`...` 与既有 `Choose new directory...`（复制迁移）仍是两个入口，语义现在更清楚了：**`...` = 指过去，`Choose new directory...` = 搬过去**。

### 4.2 校验：error 与 warning 分开

接纳一个真实用户配置时，`EspansoConfigValidator::validate` 现有的"有任何 `NonFatalErrorSet` 就整体拒绝"过于严苛——那是给**我们自己刚生成的**目录用的标准，对它成立（我们生成的东西不该有任何毛病），对用户手写了两年的目录不成立。

沿用 2026-08-05 已确立的 error/warning 取舍（NN/g：error 阻断、warning 可继续）：

- `espanso_config::load` 本身失败 → **error**，阻断，不切换。
- `load` 成功但带非致命错误 → **warning**，照常切换，把受影响文件数量提示给用户。

因此新增 `AdoptService::inspect`，而不是复用 `ConfigValidator::validate`。两者的严格度不同是**有意为之**，各自服务于不同来源的目录。

### 4.3 第 2 条：全量回显，分级可编辑

Configuration 页同时展示两类条目：

```
match/ui.yml   → Settings 自有   → 可编辑、可删除      （现状不变）
match/*.yml    → 外部文件        → 只读 + [Open file]  （新增）
```

外部条目带来源徽标（相对配置根的路径，如 `match/base.yml`）和只读原因。对 regex / form / vars / image 这类 UI 表达不了的匹配，原因写得更具体（"regex trigger"、"form"…），这样用户看到的不是一句笼统的"不支持"，而是知道自己该去改哪里。`Open file` 直接用系统默认编辑器打开源文件——即 §3 里 VS Code 的 "Edit in settings.json" 逃生口。

**两个列表在模型里严格分开**（`SettingsModel.matches` 与 `SettingsModel.external`）。这不只是展示上的区分：保存路径只会遍历 `matches`，外部条目在数据结构层面就没有机会被写进 `ui.yml`。如果混在一个 `Vec` 里，一次疏忽的 `save` 就会把整个 `base.yml` 复制进 `ui.yml`，触发重复 trigger、甚至丢内容。分开存放让这类错误不可能发生，而不是靠记得别犯。

搜索框对两类同时生效。

### 4.4 切目录后重新加载

`UiMatchRepository` 在 `app.rs` 里改为 `Rc<RefCell<UiMatchRepository>>`，目录切换成功后原地替换，然后统一走一个 `reload_matches` 助手：重新 `load()` 自有匹配 + 重新扫描外部匹配 + 刷新列表。启动时的首次加载走同一个助手，避免两条初始化路径漂移。

### 4.5 扫描的容错取舍

`inventory::scan_external_matches` 遇到解析不了的 `.yml` **跳过而不报错**。理由：它只服务于展示，而真正会阻断使用的配置错误已经由 §4.2 的 `inspect` 在切换时把过一道关；因为某个无关文件写坏就让整个编辑器打不开，是把展示功能的故障放大成了主功能的故障。

### 4.6 顺带修掉的既有崩溃（`Save deletion`）

把 `UiMatchRepository` 换成 `Rc<RefCell<...>>` 时发现 `on_commit_deletion` 原本就会 panic：

```rust
match commit_repository.save(commit_model.borrow().matches()) {
    Ok(()) => {
        commit_model.borrow_mut().reduce(ModelMessage::Saved);  // panic
```

match 判别式里产生的临时量存活到**整个 match 块结束**，因此 `borrow()` 得到的 `Ref` 在 `Ok` 分支里仍然活着，`borrow_mut()` 撞上它 → `RefCell already borrowed`。因为 `commit_model` 是闭包捕获的 `Rc`，生命周期足够长，编译器不会报错，只在用户点「Save deletion」时崩窗口。

已用两个最小复现验证：`Rc<RefCell<_>>` 版本运行即 panic；把判别式提到 `let outcome = ...;`（临时量在分号处析构）后正常。本次改动采用后者，顺带修复。这不是本轮需求，但它就在被改的那几行里，留着等于明知有雷不排。

## 5. 影响面

| 文件 | 改动 |
|---|---|
| `espanso-settings/src/adopt.rs` | 新增：`AdoptService`、`AdoptPlan`、`ConfigFolderPlan` |
| `espanso-settings/src/inventory.rs` | 新增：`ExternalMatch`、`scan_external_matches` |
| `espanso-settings/src/model.rs` | 新增 `external` 列表与 `filtered_external` |
| `espanso-settings/src/app.rs` | 仓库改可替换、`browse-config` 改分流、新增 `reload_matches` 与 `open-match-file` |
| `espanso-settings/ui/settings.slint` | `MatchRow` 增加来源/只读字段，外部条目改只读渲染，文案更新 |
| `espanso-settings/src/lib.rs` | 导出新类型 |
| `espanso-settings/tests/adopt.rs`、`tests/inventory.rs` | 新增测试 |

不改：数据库、网络协议、匹配引擎、YAML 格式、`espanso` 二进制 crate、迁移流程。

## 6. 验收标准

- [ ] 选中一个含既有 espanso 配置的目录：切换成功，不向该目录写入任何文件，提示统计信息与需要重启。
- [ ] 选中空目录 / 不存在的目录：行为与 2026-08-05 一致，生成默认配置。
- [ ] 选中非空但非 espanso 配置的目录：拒绝，文案说明原因与下一步，且不写入任何文件。
- [ ] 切换后 Configuration 页立即显示新目录的匹配，无需重开 Settings。
- [ ] 外部文件的匹配以只读形式显示，带来源徽标；`Open file` 能打开源文件；没有 Edit / Delete 按钮。
- [ ] 保存自有匹配不会把外部匹配写进 `ui.yml`。
- [ ] `cargo fmt --check`、espanso-settings 严格 Clippy、crate 测试全绿。

## 7. 回滚方案

- 目录接纳：`AdoptService::plan` 直接返回 `Scaffold` 分支即回到"仅接受空目录"。
- 回显：`reload_matches` 里不调用 `scan_external_matches` 即回到只显示 `ui.yml`。
- 两者都不改磁盘格式，回滚无数据迁移成本。

## 8. 调研依据

深度调研（三轮）。SearXNG-public 与私有 firecrawl 本机不可达，降级至 WebSearch。

| # | 结论 | 可信度 | 来源 |
|---|------|--------|------|
| 1 | Obsidian "Open folder as vault" 支持非空目录，原有文件原地保留、不复制不移动，仅添加 `.obsidian` 标记 | ⭐⭐⭐ | [Manage vaults, Obsidian Help](https://obsidian.md/help/manage-vaults)、[Opening an Existing Folder as a Vault](https://app.studyraid.com/en/read/46589/2185753/opening-an-existing-folder-as-a-vault) |
| 2 | Cryptomator 用 Open Existing / Create New / Recover 三个显式入口区分意图 | ⭐⭐ | [Adding Vaults, Cryptomator Docs](https://docs.cryptomator.org/desktop/adding-vaults/) |
| 3 | 入口命名必须描述真实行为，否则造成困惑 | ⭐⭐ | [KeePassDX #1714](https://github.com/Kunzisoft/KeePassDX/issues/1714) |
| 4 | GUI 对无法表达的复杂类型不提供控件，改为跳转源文件编辑 | ⭐⭐⭐ | [VS Code Settings](https://code.visualstudio.com/docs/configure/settings)、[vscode#67908](https://github.com/microsoft/vscode/issues/67908) |
| 5 | 可视化编辑器回写 YAML 会清掉它不理解的内容（反面教材） | ⭐⭐ | [HA Community 566338](https://community.home-assistant.io/t/editing-yaml-automations-editor-reverts-code-to-visual-editor-version/566338)、[Automations in YAML](https://www.home-assistant.io/docs/automation/yaml/) |
| 6 | Rust 无成熟保注释 YAML 往返库：`serde_yaml` 停维、`yaml-rust2` 不保留注释、`saphyr` 计划中未落地 | ⭐⭐⭐ | [Libraries, yaml.org](https://yaml.org/libraries/)、["Respectful" YAML patching in Rust](https://verrchu.github.io/blog/2-respectful-yaml-patching-in-rust/) |
| 7 | 已有第三方 espanso GUI 明确警告"保存会丢失部分既有 YAML 内容，请先备份 match 目录" | ⭐⭐⭐ | [Pebkac03/espanso_gui](https://github.com/Pebkac03/espanso_gui) |
| 8 | espanso match 支持 regex / form / vars / image / global vars，远超 trigger+replace | ⭐⭐⭐ | [Forms](https://espanso.org/docs/matches/forms/)、[Regex triggers](https://espanso.org/docs/matches/regex-triggers/)、[Organizing matches](https://espanso.org/docs/matches/organizing-matches/) |
| 9 | error 阻断、warning 可继续；错误文案要说明原因与下一步 | ⭐⭐⭐ | [NN/g: Hostile Error Messages](https://www.nngroup.com/articles/hostile-error-messages/) |
| 10 | espanso 配置目录在进程启动时解析，切换后需重启 | ⭐⭐⭐ | [Configuration Basics](https://espanso.org/docs/configuration/basics/) |

结论 2 与本次取舍部分相反（Cryptomator 让用户先声明意图）。保留记录：espanso 的配置目录状态可以完全由磁盘内容判定，不存在 Cryptomator 那种"同一个目录既可能是新建也可能是恢复"的歧义，因此自动分流的信息量足够，多问一步只是摩擦。

## 9. 后续（本次不做）

- **迁移流程切换后不重新加载编辑器**：`on_migrate` 在后台线程完成，回调必须是 `Send`，因而拿不到主线程的 `Rc<RefCell<PathBuf>>`。迁移本来就以"请重启 Espanso"收尾，这里维持原状；若将来要让它也就地刷新，需要把活动路径改成跨线程可共享的形式。

- **外部匹配的原地编辑**：需要保注释的 YAML span 级修改（`yamlpath` / `noyalib` 路线），或等 `saphyr` 的注释支持落地。在那之前只读是唯一安全解。
- **`...` 与 `Choose new directory...` 的收敛**：两个入口的语义已经清楚，但仍是两个按钮；若实际使用中仍混淆，按 2026-08-05 §3.2 的备选方案收敛。
- 三端（Windows / macOS / Linux）真实窗口、DPI、IME 人工冒烟，沿用既有待办。
