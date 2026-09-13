# 原型对齐 · 阶段四 C「浮窗启动台」设计

**日期**：2026-09-13
**前置**：阶段四 A（D22–D28）、阶段四 B（D29–D34）、阶段二 Task 9（启动台页骨架）
**原型**：`docs/index.html` `#launcherWindow`（438–447）、启动台页（185–208）；`docs/app.js` `LAUNCHER_APPS` / `collectLauncherResults` / `renderLauncherResults` / `openLauncher`（3174–3206、3209–3300）、键盘与页脚（3316–3360）、设置桥接（3360–3434）；`docs/styles.css` 772–800；生成层 `src/styles/components.css` 已含 `#launcherWindow` / `.launcher-input-row` / `.launcher-ico` / `#launcherInput` / `.launcher-list` / `.launcher-item` / `.li-ico` / `.li-name` / `.li-cat` / `.launcher-footer` / `.launcher-empty` 全部规则

## 1. 范围

把浮窗启动台对齐原型并补全检索能力：原型 DOM 与页脚、统一结果源（应用 / 命令 / 笔记 / 待办 / 计算器 / 浏览器历史）、内置命令扩到 9 条、浏览器历史采集（Chrome / Edge）、启动台页补 5 个检索范围开关与历史保留设置、浮窗尺寸改 560×420、呼出时隐藏面板。

**不在 4C**：启动台窗口的页面内搜索框行为（阶段二已对齐，仅随共享结果源获得新数据源）；浏览器扩展方案（D35 明确不采用）。

## 2. 决策记录（本阶段新增，均已确认）

| # | 问题 | 采用 | 理由 |
|---|---|---|---|
| D35 | 浏览器历史来源 | 只扫描 **Chrome 与 Edge** 的 `History` SQLite（同一套 Chromium 表结构；`User Data/Default` 与 `User Data/Profile *`），**复制到临时文件后只读打开**，导入自有表；无痕访问天然不落库 | 中等工作量、无外部安装步骤；Firefox 的 places.sqlite 另需一套解析，暂不做 |
| D36 | 内置命令集合 | 原型 6 条（历史归档 / 待办清单 / 剪贴板历史 / 新建思维导图 / 统计报表 / 偏好设置）＋保留现有「重建启动器索引」「退出 Inkling」，另加「计算器」（见 D38），共 9 条 | 对齐原型功能入口，同时不丢现有运维命令 |
| D37 | 浮窗尺寸 | 宽 560（原型 `min(92vw, 560px)`）、高 420 | 对齐原型；列表高度按内容撑满 |
| D38 | 计算器 | **不内置表达式求值**：查询以 `=` 开头或匹配「计算器 / calc」时，首条结果为「🧮 计算器 · 打开系统计算器」，执行 `calc.exe`；原型「结果复制进粘贴板」的行为不采用 | 用户指定：直接打开系统计算器即可 |
| D39 | 浮窗呼出行为 | 呼出时 `panel_hide` + 清空输入；**不隐藏主窗口**（原型的 `closeAllWindows` 不采用） | 保留用户在主窗口继续操作的上下文 |
| D40 | 浮窗动作栏 | 保留现有的 Tab 动作栏（打开 / 管理员 / 打开所在文件夹）、`Alt+N` 直达与 `Ctrl+Enter` 管理员，**并入原型 `.launcher-footer` 右侧**；页脚左侧仍是原型的导航提示 | 能力超集，不新增视觉语言 |

## 3. 与原型的差异处理表

| # | 项 | 原型 | 现状 | 做法 |
|---|---|---|---|---|
| 1 | DOM | `#launcherWindow.glass` > `.launcher-input-row`(`.launcher-ico` 🚀 + `#launcherInput`) + `.launcher-list` + `.launcher-footer`；宽 `min(92vw,560px)` | 自建 `.launcher` 结构（`launcher.css`） | 就地重写模板；删 `src/styles/launcher.css`；`launcher.ts` 只引 `@/styles` |
| 2 | 条目 | `.launcher-item(.active)` > `.li-ico` + `.li-name`(+`<small>` 副文案) + `.li-cat` | 结构相近但带 `.launcher-hint`(Alt+N) 与动作栏 | 改原型结构；`Alt+N` 提示并入页脚（D40） |
| 3 | 结果源 | 计算器 → 应用 / 命令 → 笔记 → 待办 → 浏览器历史，各受范围开关控制；空查询只列应用 / 命令 | 仅后端 应用 / UWP / 文件 / 命令 | 共享件 `useLauncherResults()`（§4.1）：浮窗与启动台页共用；笔记 / 待办前端过滤，历史走后端查询 |
| 4 | 笔记检索 | 正文 + 标签 + 导图节点文本；跳转 `openNoteInPanel` / `openMindmapEditor` | 无 | 就地：`useNotes()` + `mindmapAllText`；导图 → `api.windows.mindmapOpen(id)`，文本 → `api.windows.panelOpenNote(id)` |
| 5 | 待办检索 | 内容匹配；跳 `openMainWindow('todos')` | 无 | 就地：`useTodos()`；跳 `api.windows.showMain('todos')` |
| 6 | 计算器 | `Function()` 求值 + 结果复制进粘贴板 | 无 | D38：`=` 开头或匹配「计算器」→ 条目「🧮 计算器」，执行 `cmd:calc`（后端启动 `calc.exe`）；不内置求值 |
| 7 | 浏览器历史 | 扩展回调模型（`recordBrowserVisit`） | 无 | D35：`services/browser_history.rs` 定时导入自有表；`browser_history_search` 查询；条目「🌐 标题 / URL」→ 默认浏览器打开 |
| 8 | 内置命令 | 6 条 Inkling 功能 | 3 条（设置 / 重建 / 退出） | D36：扩到 9 条（§4.5） |
| 9 | 页脚 | 有结果「↑↓ 导航 · ↵ 执行 · Esc 关闭 · 共 N 项」；无结果「↵ 回车搜索 · Esc 退出」 | 无页脚；动作栏常驻 | 原型两态文案；动作 / 快捷键提示放右侧（D40） |
| 10 | 无结果回车 | 用默认浏览器搜索该词 | 无 | 就地：`api.openUrl('https://www.google.com/search?q=' + encodeURIComponent(q))` |
| 11 | 入场 | y −18、opacity 0、scale .98 → 0.2s power2.out | 无 | `enter()` y −18 + 淡入 + scale .98、`--dur-base`（`SlideOptions.scale` 已于 `presets.ts` 支持，全参数对齐） |
| 12 | 呼出副作用 | `hidePanel(); closeAllWindows()` | 不隐藏任何窗口 | D39：`launcher_show` 内先 `panel_hide`，主窗口保留 |
| 13 | 窗口尺寸 | 宽 560、高随内容 | 640×464 固定 | D37：560×420（`windows.rs` 常量） |
| 14 | 失焦关闭 | 点击窗外 mousedown 关闭 | 窗口 `blur` 即隐藏 | 保留（能力等价） |
| 15 | 启动台页设置行 | 快捷键 / 检索范围 ×5 / 历史保留天数 / 已记录地址 / 浮窗启动台 | 快捷键 / 全盘索引 / 排除目录 / 索引与重建 / 扫描根目录 / 浮窗启动台 | 补 7 行：检索范围 ×5（应用与命令、笔记、待办、计算器、浏览器历史）、历史保留天数、已记录历史地址 N 条；项目扩展行（全盘索引等）排在原型行之后 |

## 4. 设计

### 4.1 共享结果源

**纯函数 `src/utils/launcherResults.ts`（可测）+ 组合式 `src/composables/useLauncherResults.ts`（取数据）。**

```ts
export interface LauncherResult {
  id: string                    // 稳定 key（kind:xxx）
  ico: string                   // emoji
  name: string
  sub?: string                  // <small> 副文案
  cat: string                   // .li-cat（应用 / 文件 / 文件夹 / 命令 / 笔记 / 待办 / 计算器 / 历史）
  /** 执行入口标识（由组合式解释）：后端命中带 path/kind，其余带语义 key。 */
  action: { type: 'launch'; path: string; kind: string } | { type: 'note'; id: string; mindmap: boolean } | { type: 'todos' } | { type: 'calc' } | { type: 'url'; url: string }
}

export interface MergeInputs {
  apps: LauncherHit[]          // 后端命中
  notes: Note[]                // 全量笔记（组合式已取）
  todos: Todo[]
  history: BrowserHistoryRow[] // 组合式按查询取
  query: string
  scopes: { apps: boolean; notes: boolean; todos: boolean; calc: boolean; history: boolean }
}
export function mergeLauncherResults(input: MergeInputs): LauncherResult[]
```

- 顺序：计算器 → 应用 / 命令 / 文件 → 笔记 → 待办 → 历史（原型拼接顺序）；空查询只保留第一段（后端常用项）。
- 笔记：`content / tags / mindmapAllText(mindmap_data)` 含查询词（不区分大小写）；同一笔记只入列一次，按命中原因分档（正文 → 标签 → 导图），**每档限流 8 条、整段合计 ≤24**（原型是全量拼接，限流只为避免正文命中一次刷出几十条把小分类挤出视野；标签 / 导图是更精确的命中，不占正文那 8 个名额）；`sub` 有标签显示 `[标签]`、否则 `formatStamp(updated_at)`（原型 `n.time` 口径）；`editor_mode === 'mindmap'` → 🧠 否则 📝；`cat: '笔记'`。
- 待办：`content` 含查询词，取前 8 条；`cat: '待办'`；`sub` 为 `dateKeyOf(due_at) · 优先级`。
- 计算器：查询以 `=` 开头，或 `/计算器|calc/i` 命中 → 首条「🧮 计算器」`sub: '打开系统计算器'`。
- 历史：`cat: '历史'`，`name` 为标题（空则 URL），`sub` 为 URL。
- 组合式：查询去抖 30ms → 并行取 `api.launcher.search(q)`、`api.browserHistory.search(q)`（历史开关开才取）；`runActive(i)` 按 `action.type` 分派（launch → `api.launcher.launch(path, kind, mode, query)`；note → `mindmapOpen` / `panelOpenNote`，先隐藏启动台；todos → `showMain('todos')`；calc → `launcher.launch('cmd:calc', 'command', 'open', q)`；url → `api.openUrl`）。

### 4.2 浮窗 `LauncherApp.vue` 重写

- 模板：

```vue
<div id="launcherWindow" ref="root" class="glass">
  <div class="launcher-input-row">
    <span class="launcher-ico">🚀</span>
    <input id="launcherInput" v-model="query" placeholder="搜索应用 / 命令 / 笔记 / 待办 / 历史，= 开头打开计算器" spellcheck="false" autocomplete="off" @keydown="onKeydown" />
  </div>
  <div class="launcher-list">
    <div v-for="(r, i) in results" :key="r.id" class="launcher-item" :class="{ active: i === active }" @click="runActive(i)" @mousemove="active = i">
      <span class="li-ico">{{ r.ico }}</span>
      <span class="li-name">{{ r.name }}<small v-if="r.sub">{{ r.sub }}</small></span>
      <span class="li-cat">{{ r.cat }}</span>
    </div>
    <div v-if="!results.length" class="launcher-empty">未找到匹配项，按 Enter 用默认浏览器搜索「{{ query.trim() }}」</div>
  </div>
  <div class="launcher-footer">
    <span>{{ footerHint }}</span>
    <span class="launcher-keys">{{ actionsHint }}</span>
  </div>
</div>
```

- `footerHint`：有结果 `↑↓ 导航 · ↵ 执行 · 点击直接执行 · 共 N 项`；无结果 `↵ 回车搜索 · Esc 退出`。
- `actionsHint`：`Tab 切换动作 · Alt+1..9 直达 · Ctrl+Enter 管理员`（当前项无多动作时省略 Tab 段）。
- 键盘：↑↓ 循环、Enter 执行（无结果 → 浏览器搜索）、Tab 循环动作、`Alt+数字` 直达、`Ctrl+Enter` 管理员、Esc 隐藏。
- 显示时：清空查询、`active = 0`、`nextTick` 聚焦、`enter(root, { axis: 'y', distance: -18 })`；失焦隐藏保留。
- 删除 `launcher.css`；`launcher.ts` 改为只引 `@/styles`。

### 4.3 后端设置与迁移（v8）

- `Settings` 新增：`launcher_scope_apps / launcher_scope_notes / launcher_scope_todos / launcher_scope_calc / launcher_scope_history: bool`（默认全 true）、`launcher_history_retention_days: i64`（默认 100，1–365）。`settings.rs` 读写；TS `Settings` 同步；`useData.ts` `DEFAULT_SETTINGS` 同步。
- v8 迁移：settings 为 KV 表只推版本号；**并建 `browser_history` 表**（本次是真实 DDL）：

```sql
CREATE TABLE IF NOT EXISTS browser_history(
  url        TEXT PRIMARY KEY,
  title      TEXT NOT NULL DEFAULT '',
  visited_at INTEGER NOT NULL          -- Unix 秒
);
CREATE INDEX IF NOT EXISTS idx_browser_history_visited ON browser_history(visited_at DESC);
```

- 单测：v8 版本推进与幂等；建表存在；`launcher_history_retention_days` 读写往返。

### 4.4 浏览器历史服务 `src-tauri/src/services/browser_history.rs`

- 线程：启动后延迟 30 秒首扫，之后每 10 分钟；`prune` 在每次导入末尾执行。
- 路径：`%LOCALAPPDATA%\Google\Chrome\User Data\{Default,Profile *}\History` 与 `%LOCALAPPDATA%\Microsoft\Edge\User Data\{Default,Profile *}\History`；文件不存在则跳过（未安装）。
- 读法：复制到 `std::env::temp_dir()/inkling-hist-<pid>-<n>.tmp`（浏览器可能独占原文件），`rusqlite` 只读打开副本，`SELECT url, title, last_visit_time FROM urls WHERE last_visit_time > ?`（只取保留窗口内的），**WebKit 时间戳 = 1601-01-01 起的微秒** → Unix 秒 `webkit / 1_000_000 - 11_644_473_600`；`INSERT OR REPLACE INTO browser_history`；删副本。
- 清理：`DELETE FROM browser_history WHERE visited_at < now - retention_days * 86400`。
- 命令：`browser_history_search(query: String, limit: usize) -> Vec<BrowserHistoryRow>`（`url LIKE %q%` 或 `title LIKE %q%`，按 `visited_at DESC`）；`browser_history_count() -> i64`。IPC 注册；`tauri.ts` `api.browserHistory.{search,count}`。行类型 `{ url, title, visited_at }`。
- 单测：WebKit 时间换算（已知值）；`search` / `prune` 走内存库（同 v6/v7 单测风格）。

### 4.5 内置命令与分发

`scan.rs::builtin_commands` 改为 9 条（名称自带 emoji，前端按 `kind === 'command'` 显示名称、`cat: '命令'`）：

| id | 名称 | 执行 |
|---|---|---|
| `cmd:notes` | 📚 历史归档 | `windows::show_main(app, "notes")` |
| `cmd:todos` | ✅ 待办清单 | `show_main("todos")` |
| `cmd:clips` | 📋 剪贴板历史 | `show_main("clips")` |
| `cmd:mindmap` | 🧠 新建思维导图 | `windows::mindmap_open(app, None)`（建窗，走 async 命令路径） |
| `cmd:stats` | 📊 统计报表 | `show_main("stats")` |
| `cmd:settings` | ⚙️ 偏好设置 | `show_main("settings")`（现有 id，名称改） |
| `cmd:calc` | 🧮 计算器 | 启动 `calc.exe` |
| `cmd:rebuild` | 🔄 重建启动器索引 | 现有 |
| `cmd:quit` | ⏻ 退出 Inkling | 现有 |

- `ipc::launcher_launch` 的 `kind == "command"` 分支按 id 分发；`cmd:mindmap` 建窗——`launcher_launch` 已是 `pub async fn`，直接调用。

### 4.6 窗口尺寸与呼出

- `windows.rs`：`LAUNCHER_WIDTH 640→560`、`LAUNCHER_HEIGHT 464→420`。
- `windows::launcher_show`：`show()` 之前加 `let _ = panel_hide(app);`（D39）并补日志。

### 4.7 启动台页（`LauncherPageView.vue`）

- 内嵌试用区改用 `useLauncherResults`（新数据源立即可试）。
- 设置区在「全局呼出快捷键」之后按原型补：检索范围：应用与命令 / 笔记 / 待办 → 计算器（= 开头，打开系统计算器）→ 检索范围：浏览器历史 → 历史保留天数（number 1–365）→ 已记录历史地址 `N 条`（`browser_history_count`，挂载与导入后刷新）→ 浮窗启动台；项目扩展行（全盘文件索引 / 额外排除目录 / 索引与重建 / 文件扫描目录）保持在最后。

## 5. 测试

- Rust：WebKit 时间换算；`browser_history` 插入 / 查询 / 清理（内存库）；v8 迁移（版本、建表、幂等）；`builtin_commands` 数量与 id（若已有测试则更新）。
- 前端：`mergeLauncherResults` 纯函数单测（范围开关、排序、空查询、`=` 计算器条目）。
- 全套：`pnpm typecheck`、`pnpm test`、`pnpm sync:styles --check`、`format:check`、`cargo fmt/check/test`、`vite build`。

## 6. 实机验收（打字机 + 深色）

1. 浮窗结构：`#launcherWindow.glass` 三段式、宽 560、入场位移淡入、页脚两态。
2. 检索：输入应用名（拼音 / 首字母）/ 笔记词 / 待办词 / 历史词 各命中对应分类；范围开关逐项关闭后对应分类消失。
3. `=` 与「计算器」→ 首条「🧮 计算器」，回车打开系统计算器。
4. 无结果回车 → 默认浏览器搜索该词。
5. 键盘：↑↓ / Enter / Tab 动作 / Alt+N / Ctrl+Enter / Esc；`Alt+3` 直达第 3 条。
6. 9 条内置命令逐条执行正确（导图、calc 会开窗；退出除外）。
7. 呼出时面板被收起、主窗口保留。
8. 浏览器历史：Chrome 与 Edge 各访问页面后进库；设置页「已记录历史地址 N 条」更新；保留天数改小后清理生效（可用 SQL 改 `visited_at` 验证）。
9. 启动台页内嵌试用区与浮窗结果一致。
10. 回归：阶段三 / 四A 的面板与浮窗交互不受影响。

## 7. 提交批次（中文）

1. `feat(settings): 启动台五个检索范围开关与历史保留天数（v8 迁移含 browser_history 建表）`
2. `feat(launcher): 浏览器历史采集服务（Chrome / Edge，复制后只读导入）与查询 / 计数命令`
3. `feat(launcher): 内置命令扩到九条并按 id 分发（含系统计算器）`
4. `feat(launcher): 共享结果源（笔记 / 待办 / 计算器 / 历史）+ 纯函数与单测`
5. `feat(launcher): 浮窗启动台对齐原型（DOM / 页脚 / 入场 / 560×420 / 呼出收起面板），删除 launcher.css`
6. `feat(launcher): 启动台页补检索范围与历史设置行`
7. `docs(design): 阶段四C 实机验收记录`

## 8. 风险

| 风险 | 应对 |
|---|---|
| Chrome / Edge 运行时独占 History 文件 | 复制副本再打开（D35）；复制失败只记日志跳过本轮 |
| WebKit 时间戳换算写错 | 单测用已知值 |
| 历史条目过多拖慢查询 | 只保留 `retention_days` 窗口内记录；`visited_at` 建索引 |
| `=` 前缀与普通查询冲突 | 只在 `=` 开头或匹配「计算器」时触发 |
| 浮窗 560 宽下长路径截断 | `.li-name` / `small` 已 ellipsis（生成层已有） |

## 9. 实机结论（实施后填写）

- 各数据源命中情况：待填。
- 历史采集（Chrome / Edge）与保留天数清理：待填。
- 补丁清单：待填。
