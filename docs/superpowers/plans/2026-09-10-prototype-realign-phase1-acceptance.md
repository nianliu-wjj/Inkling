# 原型对齐 · 阶段一「基础层」实机验收记录

**日期**：2026-09-10
**对应计划**：`2026-09-10-prototype-realign-phase1-foundation.md`（Task 10）
**对应设计**：`../specs/2026-09-10-prototype-realign-phase1-foundation-design.md`（§9 验证清单、§12 实机结论）
**提交范围**：`ac81857`（设计）… 本记录所在提交，共 17 个中文批次提交
**环境**：Windows 11 · 2560×1600 @150% · WebView2 · `pnpm tauri:dev` 四轮启动
**方法**：PowerShell + Win32（`.tmp/win.ps1` 枚举/显示/截图/按键/点击，`.tmp/frames.ps1` 28ms 间隔逐帧连拍，`.tmp/devtools.ps1` 读 WebView DevTools 控制台，`.tmp/anim.ps1` 切系统动效开关，`node:sqlite` 只读查库）。截图存于 `.tmp/run/`（已 gitignore，不入库）。

---

## 1. 自动化校验（全绿）

| 项 | 结果 |
|---|---|
| `pnpm sync:styles --check` | 通过（原型 SHA-256 `9b7fb60924cb…`），再生成一次 `git status` 无变化 |
| 选择器覆盖核对（`.tmp/coverage.mjs`） | 新原型选择器在生成层缺失 **0**；扩展层与原型同名仅 8 条（`:root .glass .window-titlebar .te-field input/select .prio-opt .card-confirm-text #todoEditorOverlay`，均为有意的补充声明） |
| `pnpm typecheck` | 0 错误 |
| `pnpm test:scripts` | 6 / 6 |
| `cargo test` | 74 / 74（含 v5 迁移 4 个新用例） |
| `cargo fmt --check` / `pnpm format:check` | 干净 |
| `pnpm exec vite build` | 成功 |
| gsap 残留 | `src/**/*.{ts,vue}`、`package.json` 均无；仅生成层 `components.css` 内原型自带的一句注释 |

## 2. 数据库迁移（spec §9-10）

- 启动前（node:sqlite 只读）：`user_version=4`、`theme=dark`；首轮启动日志 `[data] v5 迁移：主题由 dark 归到新默认 typewriter`；启动后 `user_version=5`、`theme=typewriter`。
- 设置页下拉切到深色再切回：数据库 `theme` 同步 `dark → typewriter`，四轮启动结束均为 `typewriter`。
- 本机无 `sqlite3` CLI，改用 `node:sqlite`（Node 24 内建）只读查询，结论等价。

## 3. 逐窗口比对（打字机 / 深色）

| # | 窗口 | 打字机 | 深色 | 备注 |
|---|---|---|---|---|
| 1 | 主窗口（笔记 / 粘贴板 / 待办 / 设置 / 统计 / 日期详情） | ✅ | ✅ | 米色纸面、白卡黑描边、等宽字体、锐利直角；深色毛玻璃与迁移前一致。实机修复两处：主题下拉标记顺序、粘贴板裸 `<ul>` 项目符号（见 §7） |
| 2 | 呼出面板 | ✅ | ✅ | 逐帧：入场约 +80ms 呈半透明缩放态、+190ms 到位；收起约 +100ms 起淡出、+240ms 隐藏；隐藏再显示重播（面板 DevTools 日志 `enter` / `exit` 交替 ≥ 6 次）；无残影、无停在透明态。实机修复：`.nav-dot::before` 桥接（见 §7） |
| 3 | 置顶浮窗 | ❌ 未能验证 | ❌ 未能验证 | **既有缺陷，与阶段一无关**：`ipc::pin_create` 是同步命令建窗，`Inkling Pin` 窗口句柄出现但 `pinned.html` 从未被 vite 请求（WebView 未初始化），且此后主窗口全部 invoke 挂起（新建待办点保存无响应、统计页热力图空白）；两轮独立启动均复现，重启后消失。`pin_create` 自 `d7bac0b` 起未改动，`inkling-tauri-window-pitfalls` 记忆早已预测。建议修法：改为 `pub async fn`（与 `editor_open` / `mindmap_open` 同款），待用户拍板 |
| 4 | 提醒卡片 | ✅ | ✅ | 新建「15 分钟后到期」待办 → `offset` 时隙在下一个 30s tick 弹出（日志 `[reminder] 弹窗提醒 … slot=offset`）；卡片随主题渲染 |
| 5 | 感应区 | ✅（补丁后） | ✅ | 补丁前打字机下「正在感应」为深字压深底不可见 → `extensions.css` §8 钉回浅色令牌；进度条与红点正常，悬停 100ms 后面板呼出 |
| 6 | 灵动岛 | ✅（补丁后） | ✅ | 同上；补丁后「今天没有待办 · 点此新建」可读 |
| 7 | 启动台 | ✅ | ✅ | Alt+Space 呼出，输入框可键入（IME 候选窗弹出证明输入到达），结果列表渲染 |
| 8 | 独立编辑窗口 | ✅ | ✅ | 面板待办页 📅 → 全屏遮罩 + 居中弹窗（`extensions.css` §5 桥接生效），字段可编辑 |
| 9 | 思维导图窗口 | ✅ | ✅ | 工具栏 / 画布 / 右侧栏无错位（`mindmap.css` 未受影响；接入主题属阶段五） |

额外巡检：Tab 焦点环两主题均可见且无双环（深色：粘贴按钮；打字机：确认条「取消」）；删除二次确认浮层（旧原型 `.card-confirm` 桥接）两主题正常；主窗口待办编辑弹窗（ModalShell）两主题居中正常。

## 4. 列表错落与悬浮复位（spec §9-8）

- 主窗口粘贴板 89 条：点页签后 +140ms 中间帧第 5 张卡片半透明上浮，其余已到位；DevTools 日志「列表新增 89 项，错落入场前 12 项」。
- 待办新增 1 条后：「列表新增 1 项，错落入场前 1 项」；搜索词变化时旧项不重播（keyed 复用）。
- 动画结束后悬浮：卡片操作按钮正常显现，打字机主题 hover 位移 / 投影正常，说明内联样式已清。

## 5. 减少动态效果（spec §9-9）

- 用 `SPI_SETCLIENTAREAANIMATION` 关闭系统「动画效果」并重启应用：面板呼出即完整可见（无淡入），面板 DevTools 日志「系统要求减少动态效果，JS 动效整体关闭」；粘贴板列表完整可见无淡入。
- 测试后已恢复（`animations_enabled=True` 复核）。

## 6. 主题切换（spec §9-10）

- 下拉顺序：打字机首位，其后深色、浅色、纸杯蛋糕、大黄蜂、翡翠…，与原型 `THEMES` 一致；棕褐末位由 `constants/themes.ts` 保证。
- 实机切换打字机 ↔ 深色各 3 次，主窗口 DevTools 无错误；其余 29 套未逐一切换（纯 `data-theme` 属性切换 + 一次 `settings_save`，无主题相关 JS 分支）。
- **发现原型自身问题**：`.settings-body .setting-row span:first-child { width: 130px }` 会命中主题下拉里的 `.theme-dots`（它是触发器 / 选项的首个子 span），把名称推到中间；用无头 Edge 渲染 `docs/styles.css` + 原型下拉片段确认原型同样如此，故**未在应用侧改动**；150% 缩放下「纸杯蛋糕」会折行。建议在原型里把该规则改为 `.setting-row > span:first-child`，改后 `pnpm sync:styles` 即可。

## 7. 实机补丁清单（回填 spec §12）

| 位置 | 内容 | 删除时机 |
|---|---|---|
| `extensions.css` §5 | `.nav-dot::before { content: none }`：圆点导航仍是 emoji，原型的 `::before` 矢量圆点在 22px grid 里占一行把 emoji 挤到下方 | 阶段三改矢量圆点后 |
| `extensions.css` §5 | `.archive-page > ul { list-style: none; margin: 0; padding: 0 }`：主窗口粘贴板仍用裸 `<ul>`，新原型 `.clip-item` 不再是 flex 盒子，露出「•」 | 阶段二主窗口对齐后 |
| `extensions.css` §8 | `.island, .hotzone-indicator { --text / --text-dim / --wsa 钉回浅色 }`：固定深色底 + 主题文字令牌，在浅色默认主题下不可见 | 阶段四灵动岛 / 感应区对齐后 |
| `SettingsView.vue` | 主题下拉标记顺序对齐原型 `renderThemeDD`：色点 → `.dd-name` → `.dd-chevron` / `.dd-check`，并给 `.theme-dd` 加 `open` 类 | 阶段二设置页对齐时复核 |

## 8. 遗留与后续

1. **置顶浮窗缺陷**（§3-3）：用户拍板后已单独提交修复（`ipc::pin_create` 改为 `pub async fn`），实机复验留待阶段四小窗口对齐时一并做。
2. 验收在用户数据库里创建了 2 条测试待办：「phase remindertest」（19:17 到期）、「darkremindertest」（19:46 到期），均为 `open` 状态，**未代删**，请在待办页手动删除。
3. 面板圆点导航仍为 emoji（阶段三）；主窗口侧栏仍为三页签、无启动台 / 灵动岛页（阶段二）；导图窗口未接入主题（阶段五）。
4. 验收辅助脚本（`.tmp/win.ps1` `frames.ps1` `devtools.ps1` `anim.ps1`）未入库；后续阶段若复用，可用 `/run-skill-generator` 固化为项目技能。
