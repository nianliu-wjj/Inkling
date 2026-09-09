# 启动器全盘文件索引设计

**日期**：2026-09-09　**状态**：待评审（决策已给推荐值，逐条可否决）
**对应需求**：用户 2026-09-08 提出——「启动器可以搜索当前系统中所有的应用程序、文件夹、文件，将搜索的数据按照匹配度进行排序（应用程序、文件夹、文件），按下回车时，默认选中结果中的第一个并打开」。
**已定方向**（用户本会话拍板）：搜索范围 = **全盘**；索引机制 = **SQLite 落地索引**。

## 现状（已实现，不动）

- 打分引擎 `score.rs`（ZeroLaunch 移植，拼音+模糊+编辑距离），历史加权 `history.rs`。
- 扫描 `scan.rs`：程序（开始菜单/桌面 `*.exe/*.lnk/*.url` 深度5）、UWP（Get-StartApps）、**配置的文件根目录**（文档/下载/桌面/图片，深度4）、内置命令 → 全部进**内存** `Vec<Candidate>`。
- 关键字管道 `keyword.rs`（名称 → 拼音全拼/简拼等）。
- 前端 `LauncherApp.vue`：默认选中第一项（`active=0`），Enter 执行选中动作。**「回车打开第一项」已满足，不改。**

## 目标与缺口

1. 文件/文件夹搜索覆盖**全盘固定磁盘**（当前仅 4 个用户目录）。
2. 结果**类别优先**：应用 > 文件夹 > 文件（当前纯按分数，无类别倾向）。
3. 内存预算：需求 3.1「空闲常驻 ≤120MB」——全盘条目数可达百万级，**不能全进内存**（这正是选 SQLite 落地的原因）。

## 决策记录

### D1：索引存储

**采用**：独立 SQLite 库文件 `<应用数据目录>/launcher-index.db`（与业务库分离，避免索引写入干扰笔记库、便于单独重建/删除）。

- 表 `file_entry(path TEXT PRIMARY KEY, name TEXT, name_lower TEXT, pinyin TEXT, initials TEXT, kind INTEGER, mtime INTEGER, gen INTEGER)`。
- FTS5 虚表 `file_fts(name_lower, pinyin, initials)`（`content=file_entry`，`tokenize=trigram`）——trigram 支持子串/错字的粗筛，命中后再用 `score.rs` 精排。
- `gen`（代际）用于增量：每轮重建递增全局代，扫到的行写当前代，扫描结束删除非当前代的行（即已消失的文件）。

**理由**：FTS5 trigram 粗筛在磁盘完成、只返回候选，内存只承载命中的 ≤N 行；分离库文件让「重建索引」= 删文件重扫，简单可靠。

### D2：索引构建（后台线程，复用现有重建线程）

**采用**：

- 枚举固定磁盘：`GetLogicalDrives` + `GetDriveTypeW == DRIVE_FIXED`（`windows-sys` 已是依赖）。
- 逐盘 `WalkDir`（已依赖），批量事务 upsert（每 2000 行一提交），写 name/拼音（复用 keyword.rs）/kind/mtime/gen。
- **内置排除**：`Windows`、`$Recycle.Bin`、`System Volume Information`、`ProgramData\Package Cache`、`AppData\Local\Temp`、`node_modules`、`.git`、`target`、`.cache` 等（可在设置里追加）。
- 首次索引在后台跑，**不阻塞启动器**：程序/UWP/命令即时可搜，文件随索引推进逐步可搜；托盘/日志提示进度。
- 增量：默认**每次应用启动 + 每 N 小时**做一次全盘 re-walk（mtime 比对跳过未变，代际删除已消失项）。不接 USN（那需管理员），用轻量周期重扫。

**理由**：纯 Rust、免管理员；周期重扫对便签场景足够，避免 USN 的权限与复杂度。

### D3：查询流程（前端 120ms 防抖不变）

**采用**：

1. 内存候选（程序/UWP/命令）：照现有 `score.rs` 全量打分。
2. 文件候选：`file_fts MATCH`（把输入切 trigram）粗筛，`LIMIT 800` 取回行 → 用**同一** `score.rs` 在内存精排 → 取分数 top-K。
3. 合并两路 → 统一最终排序（D4）→ 截断每类上限。

**理由**：磁盘粗筛把百万级压到百级，内存只精排候选，既全盘可搜又守住内存预算与 3.1「全库搜索 ≤150ms」。

### D4：类别优先排序

**采用**：最终排序主键仍是**匹配分**，但按分数分档（如四舍五入到 0.5 档）后，**同档内按类别优先** App/UWP > Folder > File，再按历史加权。等价「按匹配度排序，同等匹配下应用在前、文件夹次之、文件最后」，与用户括号里的顺序一致，又不会让一个烂匹配的应用压过一个完美匹配的文件。

**理由**：既满足「按匹配度」主序，又实现「应用>文件夹>文件」的类别倾向。

### D5：设置项

**采用**：设置页启动器分区新增——「全盘文件索引」开关（默认**开**）、「额外排除目录」列表、「重建索引」按钮（删库重扫）。关闭时回退到现有「配置根目录」模式。保留现有根目录配置作为「关闭全盘索引时」的来源。

### D6：不做

- USN/MFT 实时监控（需管理员/unsafe，见机制评审已排除）。
- 文件内容全文检索（只索引路径与文件名，符合「搜索文件/文件夹」而非「搜内容」）。
- 网络驱动器 / 可移动盘（只固定盘，避免离线卡顿）。

## 架构

```
Rust  src-tauri/src/services/launcher/
  index_db.rs   新增：SQLite 打开/建表(FTS5 trigram)/批量 upsert/代际 prune/查询(MATCH+LIMIT)
  drives.rs     新增：枚举固定磁盘（windows-sys DRIVE_FIXED）
  scan.rs       改：文件扫描目标从「内存 Vec」改为「写 index_db」（全盘 + 排除），程序/UWP/命令仍走内存
  search.rs     改：文件路数改为 index_db 粗筛→score 精排；合并；D4 类别优先最终排序
  mod.rs        改：后台线程建/刷 index_db；查询编排两路合并
  model.rs      不变（Kind 已含 File/Folder/App/Uwp/Command）
前端  基本不动（结果结构不变）；SettingsView 启动器分区加 D5 三项
```

## 内存与性能

- 空闲：索引在磁盘，内存只常驻程序/UWP/命令候选（数百~数千）→ 守住 ≤120MB。
- 查询：FTS 粗筛 ≤800 行 + 内存精排 → 目标 ≤150ms（3.1 指标6）。
- 首次索引：后台数十秒~数分钟（盘大小而定），不阻塞交互。

## 测试

- Rust 单测：index_db 建表/upsert/代际 prune/MATCH 查询；drives 枚举（mock）；D4 排序（应用>文件夹>文件同档）。
- 实机：全盘搜到深层文件、拼音/错字命中、回车打开第一项、重建索引、关开全盘索引开关。

## 未决

无（三项方向已定）。评审后转 writing-plans 出实现计划，再分批实现提交。
