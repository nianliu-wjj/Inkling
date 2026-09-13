# 原型对齐 · 阶段四 C「浮窗启动台」实机验收记录

**日期**：2026-09-13
**分支 / 提交**：`feature/tauri-vue`，验收对象 `82addab` + 验收补丁（见 §4）
**对照文档**：`../specs/2026-09-13-prototype-realign-phase4c-launcher-design.md`（§6 十项）、`2026-09-13-prototype-realign-phase4c-launcher.md`（Task 7）
**环境**：Windows 11 Enterprise 10.0.26300；主屏 2560×1600 @150%（`\\.\DISPLAY1`）、副屏 2560×1440 @100%；主题 `typewriter`（打字机）；`pnpm tauri:dev` 后台实例
**工具**：`.tmp/win.ps1`（list/show/shot/screen/keys/click/move/wheel/paste/fg/hide）、`.tmp/acc-paste.ps1`（写入剪贴板 → 读回校验 → 重试 → Ctrl+V，本次新增）、`.tmp/acc-browser-recent.mjs`（浏览器库近时访问查询，本次新增）
**方法**：键鼠自动化（已获用户批准）。所有键入走剪贴板粘贴，不用 SendKeys 打字母/中文；发键前先确认前台窗口。

---

## 1. 自动化校验

| 项 | 结果 |
|---|---|
| `pnpm sync:styles --check` | 通过（生成层与 `docs/styles.css` 一致） |
| `pnpm typecheck` | 通过 |
| `pnpm test` | 脚本 6 / 6、单元 30 / 30 |
| `cargo test` | **87 / 87**（含本阶段新增 `v8_migration_creates_browser_history_and_is_idempotent`、`webkit_timestamp_matches_known_values`、`search_matches_title_or_url_and_orders_desc`、`prune_drops_rows_outside_retention_window`、`import_file_reads_chromium_urls_table`、`builtin_commands_cover_nine_ids`、`read_rows_keeps_most_recent_within_limit`） |
| `cargo fmt --check` / `cargo check` | 通过（仅 2 条既有 `dead_code` 警告：`launcher/mod.rs` 的 `candidate()`、`index_db.rs` 的 `count()`） |
| `pnpm format:check` | 通过 |
| `vite build` | ✓ built in 9.74s；入口 `launcher-*.js` **3.2 KB**、`main-*.js` **62.7 KB**（结果源随共享 chunk） |
| 残留 grep | `useLauncherSearch` / `LAUNCHER_KIND_ICON` / `LAUNCHER_KIND_LABEL` / `styles/launcher.css` / `launcher-actions` / `launcher-search-icon` 在 `src/` 下**无命中** |
| 常量 | `LAUNCHER_WIDTH = 560.0`、`LAUNCHER_HEIGHT = 420.0` |

---

## 2. 逐项比对（spec §6 十项）

| # | 项 | 结果 | 证据 |
|---|---|---|---|
| 1 | 浮窗结构 / 尺寸 / 入场 / 页脚两态 | **通过** | 窗口物理 840×630 = 逻辑 **560×420** ×1.5（主屏 150%）；DOM 为 `.glass > .launcher-input-row(.launcher-ico 🚀 + #launcherInput) + .launcher-list + .launcher-footer`；有结果页脚「↑↓ 导航 · ↵ 执行 · **Esc 关闭** · 共 N 项」、无结果「↵ 回车搜索 · Esc 退出」；右栏＝动作芯片〔打开 / 管理员 / 打开所在文件夹〕+「Tab 切换 · Ctrl+Enter 管理员 · Alt+1..9」，**560 逻辑宽下单行不折行**（GDI+ 预估最坏 ≈494 / 上限 516 px，实测吻合）。入场动效未逐帧验证（`.tmp/win.ps1` 的 `keysshot` 支持抓中间帧，本轮未用） |
| 2 | 六类数据源检索 + 范围开关 | **通过（应用 / 命令 / 笔记 / 待办 / 历史 / 计算器六类均命中；范围开关的「关掉后分类消失」未逐项点验，见 §5）** | 应用：`writeback`/`listary`/`你好呀` 均返回应用行（模糊 + 拼音）；笔记：`writeback` → 行 `📝 笔记: pinned writeback ok`，副文案 `今天 15:14`（`formatStamp` 口径）、徽章 `笔记`；待办：`remindertest` → `✅ 待办: phase remindertest` / `darkremindertest`，副文案 `2026-09-10 · 优先级 中`、徽章 `待办`；历史：`listary` → 4 行 `🌐 标题 + URL 副文案 + 历史`；内置命令 `📚 历史归档` 同屏出现；空查询只列应用 / 命令（9 项） |
| 3 | `=` 与「计算器」 | **通过** | 查询 `=` → 首条 `🧮 计算器`、副文案 `打开系统计算器`、徽章 `计算器`；回车后日志 `[launcher] 执行内置命令 cmd:calc → calc.exe`，任务管理器出现 **`CalculatorApp.exe`**（审查者担心的「Win11 `calc.exe` 是 UWP 激活存根、启动成功但应用起不来」**不成立**），窗口可见标准计算器 UI |
| 4 | 无结果回车 → 默认浏览器搜索 | **未验证** | 后端模糊匹配几乎总能命中（`zzqq-inkling-acceptance-nohit` 仍返回 9 项），本轮未能构造「无结果」态；该分支代码路径已核（`useLauncherResults.ts` Enter 分支对 `floating` 与非 `floating` 一致，空关键词有守卫） |
| 5 | 键盘 ↑↓ / Enter / Tab / Alt+N / Ctrl+Enter / Esc | **部分验证** | ↑↓ 循环 + `scrollIntoView` **通过**（列表随选中项滚动，正是本阶段补的那条）；Enter 执行 **通过**（计算器、历史条目各一次）；Esc 收起 **通过**（窗口 Visible → False）。Tab / Alt+N / Ctrl+Enter 受输入法/焦点干扰未能可靠发送，见 §5 |
| 6 | 9 条内置命令 | **部分验证** | `cmd:calc` 通过（见 #3）；`📚 历史归档` 在结果中渲染正确（图标与 `命令` 徽章）；其余 7 条未逐条执行，见 §5 |
| 7 | 呼出时面板收起、主窗口保留 | **通过** | 先 `Ctrl+Shift+Space` 唤面板（日志 `[panel] 面板已显示`，窗口 Visible=True）→ 再 `Alt+Space`：**Panel Visible=False、Launcher Visible=True**，主窗口（`Inkling 念头捕手`）**Visible=True 且几何未变** |
| 8 | 浏览器历史采集 / 计数 / 保留天数 / 打开 | **通过** | 首扫（启动 +30 s）：`[history] 导入 1576 条 ← Chrome Default`、`导入 110 条 ← Edge Default`、`本轮写入 1686 条，清理过期 0 条，保留 100 天`；10 分钟后第二轮同样执行（周期循环正常）；**Chrome 1576 与事前独立勘察该库行数完全一致**；库内 1659 行（1686 − 27 条跨浏览器同 URL 被 `INSERT OR REPLACE` 合并）、42 条空标题（NULL 标题降级，Task 2 的修复点）、最新一条 14:40 与 Chrome 库 mtime 分秒吻合、时间戳换算全部落在 2026-07-01 → 09-13 窗口内；设置页「已记录历史地址 **1659 条**（保留近 100 天 · 无痕访问不记录）」与库一致；历史条目回车 → **Chrome 于 21:02:11 记录该 URL 访问**（`link.juejin.cn/?target=…listary.com`） |
| 9 | 启动台页与浮窗结果一致 | **通过** | 同一查询在两处的分类顺序与条数一致（`writeback` 均为 10 项：9 后端 + 1 笔记）；页面页脚为**页面版文案**「↑↓ 导航 · ↵ 执行 · 点击直接执行 · 共 N 项」（未被误统一成浮窗的「Esc 关闭」）；页面七行设置齐备、五个范围开关全勾选、图例已含新数据源 |
| 10 | 回归 | **部分验证** | 面板呼出 / 收起 **通过**（#7 顺带）；灵动岛仍正常显示与轮播；提醒卡片在本轮多次照常弹出（`09-10 phase remindertest`、`island alert probe`、`reminder probe` 三张逾期卡先后出现）；置顶浮窗双击编辑、删除确认浮层未测，见 §5 |

---

## 3. 发现的问题

### 3.1 实机确认的缺陷（已修，见 §4）

**换查询时列表不回到顶部**（两位审查者均标为「可记」，本轮**在真机复现**，升级为必修）：
`results` 变化时 `search()` 把 `active` 复位为 0，但 `watch(active)` 只在值**变化**时触发——原本就是 0 时不会滚动，于是出现「选中项是第 0 条（回车会执行它），视野里却停在一屏看不到它的中间行」。
复现：在启动台页滚到列表中段 → 换成查询 `=` → 首条应为「🧮 计算器」但列表仍停在中间的应用行；按 ↓ 再按 ↑ 触发一次 `scrollIntoView` 后该行才出现。原型每次输入都重建 `innerHTML`（`docs/app.js:3310` / `:3383`），滚动位置天然归零。

### 3.2 采集与工具链的实测结论（供后续参考）

- **`Set-Clipboard` 可能静默失败**：首次粘贴「writeback」时剪贴板实际仍是旧内容（5 小时前的 `island alert probe`），`clipboard_entries` 里查无该词。推测与应用的剪贴板监听周期性占用剪贴板有关。→ 本轮改为「写入 → 读回校验 → 重试 ≤6 次 → 再 Ctrl+V」（`.tmp/acc-paste.ps1`），此后未再失败。
- **中文输入法会吞掉合成按键，不止字母**：`x` 触发候选窗（输入框变成 `writebackx`），**方向键 / Esc 也会被候选窗截走**，表现为「窗口状态没变、列表没滚」。凡是需要发键的验收步骤，先确认无候选窗或先点窗口空白处取消组合。
- **`node:sqlite` 读 Chromium 的 `last_visit_time`**（约 1.34e16，超出 JS 安全整数）必须在 statement 上 `setReadBigInts(true)`，否则 `.all()` 抛 `ERR_OUT_OF_RANGE`；时间窗条件写在 SQL 里算，不要把该值当参数绑定。应用后端用 `i64` 不受影响。

### 3.3 本轮产生的副作用（已清理 / 待确认）

- 验收期间用剪贴板粘贴了 8 个测试词，**均被应用的剪贴板监听记录进 `clipboard_entries`**（20:30–21:15）：`inkling-acc-token-7731`、`writeback`、`你好呀`、`=`、`listary`、`remindertest`、`zzqq-inkling-acceptance-nohit`、`思维导图`。
- 为验证「历史条目回车」，默认浏览器新开了一个标签页（`link.juejin.cn` 跳转）。
- 验证计算器时拉起过 `CalculatorApp.exe`，已 `Stop-Process` 关闭。
- 未新增 / 删除任何笔记、待办、标签数据。

---

## 4. 补丁清单

| 文件 | 改动 | 原因 |
|---|---|---|
| `src/windows/Main/LauncherPageView.vue` | 新增 `watch(query)` → `list.scrollTop = 0` | §3.1 实机复现的缺陷 |
| `src/windows/Launcher/LauncherApp.vue` | 同上（浮窗侧同源问题） | 同上 |

补丁为 2026-09-13 实机验收发现，随本记录一并提交；校验见 §1。

---

## 5. 未验证项

| 项 | 原因 | 建议 |
|---|---|---|
| spec §6 第 4 项「无结果回车 → 浏览器搜索」 | 后端模糊匹配几乎总能命中，未能构造无结果态 | 可在设置页关掉「检索范围：应用与命令」后复验，或代码走查（已核） |
| spec §6 第 5 项 Tab 循环动作 / Alt+N 直达 / Ctrl+Enter 管理员 | 输入法候选窗与前台抢占导致合成按键不可靠送达 | 用真实键盘复验最快；四个分支的代码路径已核（`actionIndex`、`floating` 守卫、`admin` 仅在多动作条目） |
| spec §6 第 6 项其余 7 条内置命令 | 同上（需在结果中选中对应命令条目后回车） | 至少补 `cmd:mindmap`（唯一建窗者，走 async 命令路径） |
| spec §6 第 1 项入场动效 | 静态截图看不到 200 ms 位移 | 用 `.tmp/win.ps1 -Action keysshot -Delay <ms>` 抓中间帧 |
| spec §6 第 10 项置顶浮窗双击编辑 / 删除确认浮层 | 本阶段未触及这两处代码（4B / 4A 已各自验收） | 可跳过；如要回归需真实键盘 |
| 深色轮 | 本轮只跑了 `typewriter`（打字机）主题；切换主题需进设置页操作 | 用户在设置页切到深色主题后，重点看浮窗 `.glass` 底色可读性与页脚对比度 |
| 多 Profile 的浏览器历史 | 本机 Chrome / Edge 均只有 `Default` | 已由单测 `read_rows_keeps_most_recent_within_limit` 等覆盖；`MAX_PROFILES = 8` 分支未实测 |
| I2「导入超限取最新」 | 本机 `urls` 仅 1576 行（≪ 20000 上限） | 只能靠代码审查 + 新增单测把关，已在 `8fe5046` 修复并加用例 |

---

## 6. 结论

本阶段七个任务的核心路径在真机上可用：**v7→v8 真实迁移**、**Chrome / Edge 历史采集与周期循环**、**浮窗 560×420 的原型结构与两态页脚**、**六类数据源的检索与执行**（应用 / 命令 / 笔记 / 待办 / 历史 / 计算器）、**呼出时收起面板而保留主窗口**、**启动台页七行设置与历史计数** 均已验证；发现并修复了 1 处实机缺陷（换查询不回到顶部）。未验证项集中在「需要真实键盘输入」的交互分支（详见 §5），不影响本阶段主要交付。
