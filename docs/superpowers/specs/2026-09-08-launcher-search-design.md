# 启动器搜索（Launcher）设计

**日期**：2026-09-08
**状态**：待评审（决策已按推荐方案落定并逐条记录，用户可逐条否决）
**对应需求**：用户 2026-09-07 提出：

> 新增一个启动器搜索的功能，参考项目 https://gitee.com/ghost-him/ZeroLaunch-rs：通过快捷键展开一个搜索框，
> 支持搜索当前系统中的所有文件/文件夹/应用程序等；强大匹配算法：全称、拼音、首字母三重匹配与拼写纠错；
> 极致性能优化：数据结构优化、分层缓存、按需加载与并发处理，中低配设备毫秒级响应；
> 所有搜索与匹配均在本地完成，无需网络，零数据采集。算法原理参考 https://github.com/ghost-him/ZeroLaunch-rs/wiki/。

## 关于本文档的决策方式

自主运行会话，沿用既有 spec 先例：决策点以「问题 / 备选 / 采用 / 理由」记录，不逐个打断用户。

## 参考项目调研结论（2026-09-08 读 wiki 与 main 分支源码）

ZeroLaunch 的搜索是一条**管道**：数据源 → 内容处理器（关键字生成）→ 检索引擎（打分）→ 评分增强器（习惯加权）→ 执行器。要点：

**关键字生成**（`builtin_plugin/keyword_optimizer/`，按优先级链式执行、去重）：
- 归一化：小写、去符号（只留字母数字）、去空格 / 空格归一、去版本号（`(64-bit)`、` 2024` 等）；
- 拼音转换器：汉字→拼音（内置字典 `pinyin.json`），连续汉字以空格分隔，非汉字原样保留；
- 首字母提取器：**消费拼音转换器的输出**，取每个空格分隔词的首字母（`QQ音乐`→`qq yin le`→`qyl`；`Visual Studio Code`→`vsc`）；
- 大写字母提取器：纯 ASCII 名字取所有大写字母小写化（`VSCode`→`vsc`）；非 ASCII 直接放弃。
每个候选保留多个关键字，**取分最高的关键字作为该候选得分**。

**标准检索引擎**（`standard_search_model.rs`，逐关键字）：
1. 容错门槛：关键字长度 ≤2 严格匹配（`tolerance=0`），否则允许输入比关键字多 1 字符；
2. 编辑距离基础分：对关键字的**任意后缀**与输入求最短编辑距离 `d`，`value = 1 - d/n`，分 = `3·log2(n+1) · e^(3·value-2)`；
3. 长度比率调整（乘）：`ratio = min(1, len_in/len_kw)`，乘 `3·log2(ratio+1)`；
4. 溢出惩罚（乘）：输入长于关键字时 `max(0.7, 1 - 0.3·overflow_ratio)`；
5. 子集匹配分（加）：输入字符在关键字字符多重集中命中的个数；
6. KMP 分（加）：公共前缀长度 + （关键字包含输入时）输入长度；
7. 固定偏移（加）：用户设的稳定偏差。

**评分增强**（`history_booster.rs` / `query_affinity.rs`）：
- 历史分 `ln(1+launch_count)`×0.8；近 7 天按天计数加权 ×1.5；短期热度 `1/(1+Δt/(decay+1))`×0.5，`decay=10800s`；
- **抑制因子** `(base_score/15).clamp(0,1)`：基础匹配分低时习惯加成打折，防高频程序挤占真正匹配项；
- 查询亲和度：记录「查询词→最终启动项」，同词再输时加分（权重 3.0，时间衰减，15s 冷却）。

**数据源**（`program_source.rs`）：默认扫 `C:\ProgramData\Microsoft\Windows\Start Menu`、用户开始菜单、用户桌面；模式 `*.exe / *.lnk / *.url`；`max_depth=5`；排除关键字 `uninstall / 帮助 / help / 卸载`；`forbidden_paths`；UWP 由系统应用列表枚举；读 `desktop.ini` 本地化名。文件全盘搜索在参考项目里**依赖外部 Everything**，不是自建索引。

## 范围

### 做

| 分区 | 内容 |
|---|---|
| 呼出 | 全局快捷键（默认 `Alt+Space`，设置页可改；与面板快捷键互斥校验）呼出/隐藏搜索窗口；Esc 隐藏；失焦自动隐藏 |
| 窗口 | 光标所在屏居中偏上（顶部 22%），宽 640 逻辑，搜索框 + 最多 9 条结果；玻璃质感与 Inkling 主题一致；`Alt+数字` 直达 |
| 数据源 | ① 程序：开始菜单（公共 + 用户）、桌面，`*.exe/*.lnk/*.url`，深度 5，排除 uninstall/help/卸载/帮助；② UWP 应用：`Get-StartApps`（PowerShell，一次性、后台）；③ 文件与文件夹：用户配置的根目录列表（默认：用户主目录下 Documents / Downloads / Desktop / Pictures，深度 4，可增删、可设深度与排除关键字）；④ 内置命令：打开设置、重建索引、退出 |
| 关键字生成 | 小写 / 去符号 / 空格归一 / 去版本号 / 拼音 / 拼音首字母 / 大写缩写，链式生成、去重 |
| 匹配 | 标准引擎（上文 7 步逐项移植，含编辑距离容错）；短输入（1 字符）只走前缀/首字母 |
| 排序 | 引擎分 + 历史加权（含抑制因子）+ 查询亲和度；空输入显示最近/最常用 9 条 |
| 动作 | Enter 打开（explorer 代理，不成为子进程）；Ctrl+Enter 管理员运行（`runas`）；Tab / 右键：打开所在文件夹、复制路径 |
| 性能 | 索引常驻内存（`Vec<Candidate>` + 预生成关键字）；扫描在后台线程，`rayon` 并行打分，`select_nth_unstable` 取 Top-K；分层缓存：L1 查询结果 LRU（同一索引代际内）、L2 索引快照落盘（JSON，启动即用、后台增量重扫）、L3 图标缓存（按需提取到数据目录 PNG）；输入去抖 30ms；首屏 ≤ 50ms |
| 隐私 | 全部本地；索引与历史存 `app_data_dir/launcher/`；不出网 |

### 不做（及理由）

| 项 | 理由 |
|---|---|
| 全盘文件实时索引 | 参考项目自身也依赖外部 Everything；自建全盘 NTFS 索引（USN Journal / MFT）是一个独立的大项目，且会显著增加常驻内存与磁盘 IO。本期用「可配置根目录 + 深度」覆盖日常文件；预留 Everything（`es.exe` / SDK）适配点作为后续子项目 |
| 语义（AI）搜索、浏览器书签、自定义网址/命令、别名、稳定偏差 UI、插件 SDK | 参考项目的扩展生态，与「搜到并启动」核心无关；别名与偏差在数据模型预留字段，本期不做 UI |
| 窗口唤醒（Shift+Enter 置前已运行窗口）、游戏模式、安装监控 | 需要枚举系统窗口/进程，属高级维护功能，后续再议 |

## 决策记录

### D1：索引与匹配在哪一侧实现

**备选**：前端 JS / **Rust 后端**
**采用**：Rust。扫描、关键字生成、打分、历史全部在 Rust（`services/launcher/`）；前端只发 `launcher_search(query)` 拿 Top-K 结果与高亮区间。
**理由**：编辑距离 + 多关键字 × 上万候选是 CPU 密集操作，Rust + rayon 才能在中低配机器上稳定毫秒级；数据不离开进程。

### D2：拼音字典来源

**备选**：`pinyin` crate（带完整字典）/ 自带精简 JSON
**采用**：`pinyin = "0.10"` crate，`ToPinyin` 取首个读音的无调拼音（`Style::Plain`），多音字取第一读音（与参考项目一致，简单可预期）。
**理由**：字典维护交给 crate；体积约 1MB，对桌面应用可接受。

### D3：文件搜索的范围

**采用**：`Settings.launcher_roots`（JSON 数组字符串，每项 `{ path, depth, excludes }`），默认四个用户目录、深度 4；程序源固定内置。索引条目上限 200,000（超出停止扫描并日志告警），单条目内存约 200B → 上限 40MB。
**理由**：见「不做」表第一行；上限防止用户把 `C:\` 整盘加进来拖垮内存。

### D4：索引持久化与刷新

**采用**：
- 快照：`launcher/index.json`（条目 + 生成关键字 + 类型 + 路径），启动时先加载快照立即可搜，随后后台全量重扫并原子替换（`rename`），代际号 +1 使 L1 缓存失效；
- 定时：每 30 分钟后台重扫；设置页「立即重建索引」；
- 历史：`launcher/history.json`（启动次数、近 7 天按天计数、最后启动时间、查询亲和度），启动项以「路径」为键。
**理由**：与项目已有 SQLite 存储分离——索引是可再生缓存，不该进业务库；JSON 便于人工排查。

### D5：UWP 枚举方式

**备选**：Windows API（`PackageManager` + AppxManifest 解析）/ **PowerShell `Get-StartApps`**
**采用**：后台线程执行 `powershell -NoProfile -Command "Get-StartApps | ConvertTo-Json"`，解析 `Name/AppID`，以 `shell:AppsFolder\{AppID}` 启动；失败（无 PowerShell / 超时 10s）则跳过 UWP，不影响其他源。
**理由**：一次调用拿到系统「所有应用」列表且已本地化，无需解析 manifest；只在索引重建时执行，开销可接受。

### D6：启动方式

**采用**：`explorer.exe <path>`（普通启动，与参考项目一致，不成为子进程）；管理员：`powershell Start-Process -Verb RunAs`；`.lnk` 直接交给 explorer（由系统解析目标）；文件夹用 `explorer.exe /select,<path>`；UWP 用 `explorer.exe shell:AppsFolder\<AppID>`。全部经 `std::process::Command` 且 `DETACHED_PROCESS`。
**理由**：不引入额外 crate（`lnk` 解析、`windows` 大依赖）；explorer 代理在 Windows 上最稳。

### D7：图标

**采用**：本期**不提取可执行文件图标**；结果项以类型图标（程序 / 文件夹 / 文件类型 emoji + 扩展名标签）区分。数据模型预留 `icon_path`，L3 图标缓存的目录与接口先建好。
**理由**：图标提取要走 Shell API（`SHGetFileInfo` / `IShellItemImageFactory`）并转 PNG，实现与测试成本高，而对「搜到并启动」无影响；作为后续子项目。

### D8：快捷键

**采用**：`Settings.launcher_shortcut` 默认 `Alt+Space`，复用 `app/shortcut.rs` 的注册与改键机制；与面板快捷键相同则拒绝保存并提示。
**理由**：`tauri-plugin-global-shortcut` 已在用。

### D9：与灵动岛、面板的关系

**采用**：启动器是独立窗口 `launcher`，不进面板插件体系；灵动岛不展示启动器内容。
**理由**：交互模型不同（键盘驱动的一次性查询 vs 常驻信息展示）。

## 架构

```
Rust  src-tauri/src/services/launcher/
  mod.rs          LauncherState（Arc<RwLock<Index>>、历史、缓存代际）、start(app)：加载快照 → 后台重扫 → 30min 定时
  scan.rs         数据源扫描：程序（walkdir）、UWP（Get-StartApps）、文件根目录（walkdir+depth+excludes）、内置命令；上限 200k
  keyword.rs      关键字生成管道（纯函数，单测）
  score.rs        标准引擎（纯函数，单测）：edit_distance_suffix / subset / kmp / length_ratio / overflow
  history.rs      启动历史与查询亲和度（纯函数 + JSON 落盘，单测）
  search.rs       search(query, top_k) → rayon 并行打分 → 抑制因子加权 → select_nth → 高亮区间；L1 LRU
  launch.rs       打开 / 管理员 / 打开所在文件夹 / 复制路径
  ipc：launcher_search(query) / launcher_launch(id, mode) / launcher_rebuild / launcher_status / launcher_show / launcher_hide
前端
  launcher.html + src/windows/launcher.ts + src/windows/Launcher/LauncherApp.vue（输入框、结果列表、键盘导航、动作栏）
  src/service/tauri.ts  api.launcher.*
  src/windows/Main/SettingsView.vue 「启动器」分区：快捷键、根目录列表（增删/深度/排除）、立即重建、索引状态（条目数/上次重建时间）
  src/styles/launcher.css
```

### 数据模型

```rust
pub struct Candidate {
    pub id: u32,                 // 索引内序号（代际内稳定）
    pub kind: Kind,              // App | Uwp | File | Folder | Command
    pub name: String,            // 显示名（.lnk 去扩展名；desktop.ini 本地化名可选）
    pub path: String,            // 启动/定位用；UWP 为 shell:AppsFolder\AppID
    pub keywords: Vec<String>,   // 预生成、去重、小写
    pub bias: f64,               // 稳定偏差（预留，默认 0）
}
pub struct Hit { pub id: u32, pub name: String, pub path: String, pub kind: Kind, pub score: f64, pub matched_keyword: String }
```

### 搜索请求时序

输入 → 前端去抖 30ms → `launcher_search(query)` → 小写 + 空格归一 → L1 命中直接返回 → 否则 rayon 对全部候选逐关键字打分（提前剔除 `len_kw + tol < len_in`）→ 历史加权 → `select_nth_unstable_by` 取 Top-9 → 写 L1 → 返回。空查询返回按历史分排序的前 9。

### 性能预算（中低配：4 核 8G，索引 5 万条）

- 单次打分：每候选 3–6 个关键字，编辑距离 O(m·n) 且 n（输入）≤ 20 → 约 5e6 基本操作，rayon 4 线程 < 10ms；
- 首次快照加载：5 万条 JSON 约 15MB，< 200ms，且在窗口第一次呼出前完成；
- 内存：5 万条 × ~250B ≈ 12.5MB。

## 错误处理与日志

- 扫描单个目录失败（权限）跳过并计数；`[launcher] 扫描完成 程序=N UWP=N 文件=N 用时=ms`。
- UWP 枚举超时 10s 放弃并日志。
- 启动失败 Toast「启动失败：<系统错误>」。
- 快照损坏 → 忽略并重扫。

## 测试

- Rust 单测：关键字生成（中英混合、版本号、缩写）、编辑距离后缀算法、子集/KMP、完整打分排序用例（`微信` 用 `wx`/`weixn` 命中优先于 `weixin_uninstall`）、历史加权抑制因子、快照读写、根目录扫描（临时目录）。
- 前端：`vue-tsc`；实机：呼出/隐藏、输入即搜、键盘上下 + Enter、Ctrl+Enter、Tab 动作、设置根目录后重建、空查询常用项。

## 未决

无。
