# 动画美化 · 面板插件化 · 极简毛玻璃主题 设计

**日期**：2026-09-03
**状态**：待评审
**对应需求**：需求规格说明书 P3 阶段「动画组件渲染美化 + 多套极简毛玻璃主题 + 插件化能力」

## 关于本文档的决策方式

用户指示「直接采用提问的推荐模式，但需要将提问的信息记录在文档中」。
因此下文每个决策点都以「问题 / 备选 / 采用 / 理由」四段记录，未逐个打断用户确认。
用户可据此逐条复核；若某条不认可，只需改该条，不影响其余设计。

另：用户提到可用 `frontend-design`、`ui-ux-pro-max` skill，但当前环境两个 skill 名均无法解析
（`Unknown skill`），故设计基于项目既有的 CSS 令牌体系推导，不引入外部设计系统。

## 现状盘点

| 方向 | 现状 | 差距 |
|---|---|---|
| 动画 | GSAP 仅用于面板滑入滑出；全站 5 个 `@keyframes`、38 处 `transition`，时长与曲线散落各处硬编码 | 无统一节奏，列表/卡片/状态变化几乎无动效 |
| 主题 | 30 套配色（`constants/themes.ts` + `styles/themes.css`），每套均覆盖 `--glass-bg / --glass-border / --glass-shadow` | 配色够用，但**玻璃质感固定**：`.glass` 里 `blur(24px) saturate(160%)` 写死，与主题无关 |
| 面板 | `PanelApp.vue` 硬编码 `MODES` 三元组 + 三个 `v-show` 组件（NotePage / ClipPage / TodoPage） | 新增一种捕获能力必须改 PanelApp 本体 |

## 范围拆分

三个方向量级差异很大，**拆成三个独立子项目**，各自 spec → plan → 实现：

| 子项目 | 量级 | 建议顺序 | 理由 |
|---|---|---|---|
| B. 极简毛玻璃主题 | 小 | 1 | 只加令牌维度，不动结构；为 A 提供令牌治理范式 |
| A. 动画渲染美化 | 中 | 2 | 沿用 B 建立的令牌约定，避免又一批硬编码 |
| C. 面板插件化 | 大（架构级） | 3 | 改 PanelApp 结构 + 契约设计，改动面最广 |

---

# 子项目 B：极简毛玻璃主题

## 决策 B1：新增主题，还是新增「玻璃质感」维度？

**备选**
1. 再写若干套「极简毛玻璃」配色主题，加进 30 套里
2. 把玻璃质感抽成与配色**正交**的独立维度，用令牌 + 档位控制

**采用**：方案 2。

**理由**：现有 30 套已覆盖配色需求，用户要的「极简毛玻璃」是**质感**而非配色 —— 同一套配色可以是厚重玻璃也可以是轻薄玻璃。正交后 30 套配色 × 3 档质感 = 90 种组合，不必新增任何一套 CSS 主题；若走方案 1，想要「深色 + 极简玻璃」还得再复制一套深色。

## 决策 B2：质感有哪些档位？

**采用**：三档。

| 档位 | 语义 | blur | saturate | 阴影 | 边框 |
|---|---|---|---|---|---|
| `minimal` | 极简 | 12px | 120% | 单层、浅 | 0.5px、更淡 |
| `standard` | 标准（默认，等于现状） | 24px | 160% | 双层含内高光 | 1px |
| `frosted` | 厚玻璃 | 40px | 180% | 三层、更深 | 1px、更亮 |

**理由**：三档覆盖「几乎无质感 → 现状 → 强质感」的完整区间，再多档用户分辨不出差异。默认档与现状逐字一致，升级后观感不变，不打扰既有用户。

## 决策 B3：「极简」具体减什么？

**采用**：减模糊、减饱和、减阴影层次、收窄边框，**不减圆角、不减配色对比**。

**理由**：圆角是窗口形态的一部分（DWM 窗口圆角已对齐 8px），改它会与系统窗口不一致；配色对比关系到可读性，属于无障碍底线，不参与「极简」的取舍。

## 决策 B4：质感设置存哪？

**采用**：`Settings` 新增 `glass_level: String`（`minimal` / `standard` / `frosted`），走既有 settings 键值表，与 `theme` 并列。

**理由**：与主题选择同源同生命周期，复用现成的「settings_save → 广播 settings-changed → 各窗口同步」链路，不引入新机制。前端同样在 localStorage 存镜像，避免启动瞬间闪变（与 `applyCachedTheme` 一致）。

## 架构

**令牌层**（`styles/tokens.css`）：把 `.glass` 里写死的值抽成令牌

```css
:root {
  --glass-blur: 24px;
  --glass-saturate: 160%;
}
.glass {
  backdrop-filter: blur(var(--glass-blur)) saturate(var(--glass-saturate));
  -webkit-backdrop-filter: blur(var(--glass-blur)) saturate(var(--glass-saturate));
}
```

**档位层**（新建 `styles/glass.css`）：`:root[data-glass='minimal']` / `[data-glass='frosted']` 覆盖
上述令牌与 `--glass-shadow` / `--glass-border`。`standard` 不设属性，等于 `:root` 默认值 ——
与主题的 `dark` 采用同一约定。

**注意**：`--glass-bg / --glass-border / --glass-shadow` 目前由每套主题各自定义（87 处）。
档位层必须**只覆盖不冲突的部分**：`blur` / `saturate` 由档位独占；
`shadow` / `border` 用档位层的 `--glass-shadow-scale`（阴影强度系数）参与运算，
而不是直接覆盖主题给的值 —— 否则 30 套主题精心调过的阴影配色会被三档统一冲掉。

**应用层**（`composables/useGlass.ts`）：与 `useTheme` 同构 —— `applyCachedGlass()` +
`useGlass().applyGlass(level)`，写 `data-glass` 属性与 localStorage 镜像。

**界面层**：设置页在「主题」下拉旁新增「玻璃质感」下拉（三项）。

## 未决

无。

---

# 子项目 A：动画渲染美化

## 决策 A1：动画参数令牌化，还是逐处调整？

**备选**
1. 逐个组件调 `transition` 的时长与曲线
2. 先建一套动效令牌（时长 / 曲线 / 位移距离），所有动画引用令牌

**采用**：方案 2。

**理由**：需求规格说明书对配色已有明确约束 ——「新增界面元素必须使用既有令牌，禁止硬编码颜色」。动效同理：38 处散落的 `transition` 正是无令牌治理的结果，逐处调只会再产生一批硬编码。令牌化后「全站动效快一点」是改一个值的事。

## 决策 A2：GSAP 还是纯 CSS？

**采用**：**CSS 为主，GSAP 仅保留窗口级的物理弹性过渡**（面板滑入滑出）。

**理由**：四个窗口各自是独立 Vue app，GSAP 全量引入会同时拖慢四个窗口的启动，而需求有「1 秒原则」。CSS 动画由合成器执行，不占主线程，更适合列表项、hover、状态变化这类高频小动效。GSAP 的价值在于 `back.out` 这类弹性曲线，只在面板入场那一处真正需要。

## 决策 A3：动画覆盖哪些位置？

**采用**：六处，按「用户能感知到状态变化」筛选，不做纯装饰动画。

| 位置 | 动效 | 目的 |
|---|---|---|
| 列表项进出 | 淡入 + 轻微上移，同批次 stagger 30ms | 让「新增了一条」可感知 |
| 卡片 hover | 抬升 2px + 阴影加深 | 指示可交互 |
| 三态切换 | 淡入淡出交叉 120ms | 消除切换时的硬跳 |
| 徽章状态变化 | 数值/颜色过渡 180ms | 优先级、提醒改动有反馈 |
| Toast 进出 | 下滑淡入 / 上滑淡出 | 现状是硬出现 |
| 提醒卡片入场 | 右侧滑入 + 轻微弹性 | 提醒是打扰性事件，需要视觉引导 |

## 决策 A4：`prefers-reduced-motion` 怎么处理？

**采用**：保留并扩展现有规则（`base.css` 已有），新增动画一律走令牌，
在 reduced-motion 下把时长令牌整体归零即可，无需逐个覆盖。

**理由**：这正是令牌化的直接收益 —— 一处生效，不会漏。

## 架构

**令牌**（`styles/tokens.css` 新增）

```css
:root {
  --dur-instant: 90ms;
  --dur-fast: 150ms;
  --dur-base: 220ms;
  --dur-slow: 320ms;
  --ease-out: cubic-bezier(0.22, 1, 0.36, 1);
  --ease-in-out: cubic-bezier(0.65, 0, 0.35, 1);
  --lift: 2px;
  --stagger: 30ms;
}
```

reduced-motion 下把四个 `--dur-*` 全部设为 `0.01ms`。

**列表 stagger**：用 Vue 的 `<TransitionGroup>` + CSS `transition-delay: calc(var(--stagger) * var(--i))`，
`--i` 由模板按索引写入行内样式。不用 JS 逐个 setTimeout。

**注意**：`TransitionGroup` 的 FLIP 动画依赖 `transform`，而面板里的 `.glass` 元素有
`backdrop-filter` —— 二者叠加在 WebView2 下会导致模糊层错位（本项目已两次踩到
backdrop-filter 的绘制不跟随变换的坑）。因此列表项动画只作用在**卡片内层**元素上，
不给带 `backdrop-filter` 的容器加 transform。

## 未决

无。

---

# 子项目 C：面板插件化

## 决策 C1：运行时动态加载外部插件，还是编译期注册表？

**备选**
1. 运行时从磁盘加载第三方 JS 插件（真正的插件系统）
2. 编译期注册表：插件是本仓库内的模块，通过注册表声明，PanelApp 不感知具体实现

**采用**：方案 2。

**理由**：`tauri.conf.json` 的 CSP 是 `script-src 'self'`，运行时加载外部 JS 必须放宽到
`unsafe-eval` 或允许任意源 —— 那等于把一个能读写本地 SQLite 与文件系统的应用的脚本沙箱打开，
代价远超收益。而用户表述的目标是「能够通过插件的方式引入更多的组件功能」，
即**新增能力不必改 PanelApp 本体** —— 注册表完全满足这一点。

若将来确实需要第三方插件，正确的做法是把插件跑在独立 WebView 里、通过 IPC 通信，
那是另一个量级的项目，不在本次范围。

## 决策 C2：插件契约长什么样？

**采用**

```typescript
export interface PanelPlugin {
  /** 唯一标识，同时作为 Settings 里启用列表的键。 */
  id: string
  /** 圆点导航的展示名与色点。 */
  label: string
  dot: string
  /** 快捷键序号（⌃1..⌃9），由注册顺序决定，不由插件指定。 */
  component: Component
  /** 面板高度上报是否需要该插件参与（含内部滚动区的插件返回 false）。 */
  reportsHeight?: boolean
}
```

**理由**：字段只保留 PanelApp 真正需要的信息。快捷键序号刻意**不让插件自选** ——
否则两个插件都想要 ⌃1 时无解；由注册顺序决定则天然唯一。

## 决策 C3：启用与排序放哪？

**采用**：`Settings` 新增 `panel_plugins: String`（逗号分隔的插件 id 有序列表）。
为空时回落到内置三件套的默认顺序。

**理由**：顺序与启用状态用一个字段表达（在列表里 = 启用，顺序 = 列表次序），
比「启用集合 + 顺序数组」两个字段更难产生不一致状态。

## 决策 C4：样式隔离？

**采用**：**不做** shadow DOM 或 CSS Modules 隔离，改为约定 —— 插件只能使用既有令牌，
类名以插件 id 为前缀。

**理由**：shadow DOM 会切断 CSS 自定义属性之外的继承，而本项目的玻璃效果依赖
祖先的 `backdrop-filter` 与 `:root` 上的主题/质感属性；隔离后插件要么失去玻璃质感，
要么得把整套令牌重新注入。约定 + code review 的成本远低于此。

## 架构

```
src/panel-plugins/
  index.ts          # 注册表：导出有序的 PanelPlugin[]，PanelApp 只依赖它
  note.ts           # 包装既有 NotePage
  clipboard.ts      # 包装既有 ClipPage
  todo.ts           # 包装既有 TodoPage
```

`PanelApp.vue` 的改动：
- `MODES` 常量删除，改为从注册表按 `panel_plugins` 设置过滤 + 排序得出
- 三个 `v-show` 组件改为 `<component :is>` 循环
- `⌃1/2/3` 改为按当前启用列表的索引动态绑定（最多 ⌃9）

**注意**：面板高度上报（`reportHeight`）目前直接量 `#panel`。插件化后仍然量 `#panel`，
但 `MutationObserver` 的观察范围要覆盖插件挂载点 —— 插件内容异步加载时高度才跟得上
（这个坑本项目已经踩过一次：数据异步到达时 ResizeObserver 未必回调）。

## 未决

无。

---

# 实施建议

三个子项目各自独立可交付，建议按 B → A → C 顺序，每个子项目单独写实现计划。
其中 C 涉及 `PanelApp.vue` 结构性重写，建议在 B、A 落地后再动，避免三方改动交织在同一文件上。

**遗留**：待办提醒改造的 Task 10（真实 SMTP 发信验证）仍未做，需要用户提供凭据。
