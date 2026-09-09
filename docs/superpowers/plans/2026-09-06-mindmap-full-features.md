# 思维导图全功能实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 Inkling 的思维导图窗口里实现参考端 `D:\参考项目\mind-map\web` 的全部编辑功能（AI / 协同 / 本地文件目录树除外，见 spec「不做」表）。

**Architecture:** 复用同一个 `simple-mind-map` 0.14.0-fix.3 与其 20 个插件（`core/plugins.ts` 等价参考端 `full.js`）；参考端 Vue 2 + element-ui + vuex + `$bus` + i18n 的 48 个组件逐个移植为 Vue 3 `<script setup>` + naive-ui + `core/store.ts` + `core/bus.ts` + 中文字面量。文档数据升级为 `getData(true)` 全量并兼容旧格式；导出经 `plugin-dialog` 保存框 + Rust `write_file_base64` 落盘。

**Tech Stack:** Vue 3.5 + TypeScript、naive-ui 2.45、simple-mind-map 0.14.0-fix.3、simple-mind-map-plugin-themes、Tauri 2（Rust）、@tauri-apps/plugin-dialog

**Spec:** `docs/superpowers/specs/2026-09-06-mindmap-full-features-design.md`（12 条决策 D1–D12 均已落定）

## Global Constraints

- 回答与提交信息用中文；提交格式 `<type>(<scope>): <中文描述>`，scope 统一用 `mindmap`（Rust 侧用 `ipc`）。每完成一个 Task 立即提交，不攒。
- 前端提交前必须通过 `npx vue-tsc --noEmit` 与 `npx prettier --check "src/**/*.{vue,ts,css}"`；Rust 提交前 `cargo fmt` + `cargo test` 零告警。
- 全项目禁止裸 `console.*`，前端一律 `logger.info/error('mindmap', ...)`；Rust 用 `eprintln!("[scope] ...")`。关键节点必须有日志：实例创建、插件动态���卸、保存（手动/自动）、导入导出开始与结束、主题/结构切换。
- 每个 `.vue` / `.ts` 文件头部有说明职责的注释块（沿用项目既有密度）。
- naive-ui 只在导图窗口引入，**不得**被 `panel.ts` / `main.ts` / `hotzone.ts` 间接 import。
- `naive-ui` 用 `darkTheme` + `themeOverrides` 映射 Inkling 令牌（见 Task 7），不引入参考端白色风格与 `isDark` 开关。
- 参考端组件里 `this.$t('x.y')` 一律替换为 `web/src/lang/zh_cn.js` 中对应的中文字面量；`this.$bus.$emit/$on` → `bus.emit/on`；`mapState(localConfig.*)` → `useMindMap().localConfig`；`setActiveSidebar(x)` → `ui.activeSidebar = x`；`this.$message.success/error` → `toast()`；`this.$confirm` → naive-ui `useDialog().warning({...})` 返回 Promise；`el-*` → 对应 `n-*`（映射表见 Task 9）。
- 移植时**只改胶水，不改算法**：参考端里对 `mindMap.*` 的调用序列、参数、事件名原样保留。
- 数据兼容：`mindmap_data` 里若无 `root` 字段视为旧格式 root，读得进、存出去时升级为全量格式（Task 4）。
- `getData(true)` 只含 `root / layout / theme / view`；「设置」侧栏的库实例配置存 `localStorage['inkling.mindmap.config']`，编辑器偏好存 `localStorage['inkling.mindmap.localConfig']`（spec D5）。

---

## File Structure

新增 / 修改文件与职责（spec D11 的落地版）：

| 路径 | 职责 |
|---|---|
| `src/windows/MindMap/MindMapApp.vue` | 窗口壳：Inkling 操作条、`n-config-provider`、画布宿主、挂载全部 chrome/sidebars/popups/dialogs；持久化与自动保存 |
| `src/windows/MindMap/core/bus.ts` | 极简事件总线 |
| `src/windows/MindMap/core/store.ts` | `reactive` UI 共享状态 |
| `src/windows/MindMap/core/localConfig.ts` | 编辑器偏好 + 库实例配置的 localStorage 读写 |
| `src/windows/MindMap/core/persistence.ts` | `mindmap_data` 字符串 ⇄ 全量数据，旧格式兼容 |
| `src/windows/MindMap/core/plugins.ts` | 注册 18 个常驻插件 + 主题包；导出 `RichText` / `Scrollbar` 供动态挂卸 |
| `src/windows/MindMap/core/createMindMap.ts` | 构造实例（全部 options）、事件转发到 bus、动态插件挂卸 |
| `src/windows/MindMap/core/useMindMap.ts` | provide / inject 上下文 |
| `src/windows/MindMap/core/naiveTheme.ts` | naive-ui 主题覆盖（映射 Inkling 令牌） |
| `src/windows/MindMap/core/exportFile.ts` | dataURL → 保存对话框 → Rust 落盘 |
| `src/windows/MindMap/core/clipboardText.ts` | 参考端 `utils/handleClipboardText.js` 移植 |
| `src/windows/MindMap/constants/{lists,shortcuts,formulas,rainbow,layouts,stickers}.ts` | 纯数据 |
| `src/windows/MindMap/widgets/{FieldRow,ColorField,SelectField,NumberField,SwitchField}.vue` | 侧栏通用表单件 |
| `src/windows/MindMap/chrome/*.vue` | 常驻 UI（见各 Task） |
| `src/windows/MindMap/sidebars/*.vue` | 侧栏 |
| `src/windows/MindMap/popups/*.vue` | 浮层 |
| `src/windows/MindMap/dialogs/*.vue` | 对话框 |
| `src/assets/mindmap/icon-font/iconfont.{css,woff2}`、`structures/*.jpg`、`image-broken.svg` | 静态资源 |
| `src/styles/mindmap.css` | 导图窗口专用样式（只由 `mindmap.ts` 引入） |
| `src/typings/simple-mind-map.d.ts` | 扩展类型声明 |
| `src/service/tauri.ts` | 新增 `files.writeBase64` |
| `src-tauri/src/ipc.rs`、`src-tauri/src/main.rs` | 新增 `write_file_base64` 命令 |
| `src/editor/MindMapEditor.vue` | 读数据改走 `persistence.parse`（只取 root） |

---

# P0 · 基础设施

### Task 1: 依赖与静态资源

**Files:**
- Modify: `package.json`（新增依赖）
- Create: `src/assets/mindmap/icon-font/iconfont.css`、`src/assets/mindmap/icon-font/iconfont.woff2`
- Create: `src/assets/mindmap/structures/*.jpg`（14 张）
- Create: `src/assets/mindmap/image-broken.svg`

- [ ] **Step 1: 安装主题包**

```bash
cd D:/Inkling && pnpm add simple-mind-map-plugin-themes
```
预期：`package.json` dependencies 出现 `"simple-mind-map-plugin-themes"`；`node_modules/simple-mind-map-plugin-themes/themeList.js` 存在。若无网络，从 `D:/参考项目/mind-map/web/node_modules/simple-mind-map-plugin-themes` 复制整个目录到 `node_modules/` 并手写依赖行（版本以该目录 `package.json` 为准），并在提交信息里注明。

- [ ] **Step 2: 复制静态资源**

```bash
cd D:/Inkling
mkdir -p src/assets/mindmap/icon-font src/assets/mindmap/structures
cp "/d/参考项目/mind-map/web/src/assets/icon-font/iconfont.woff2" src/assets/mindmap/icon-font/
cp "/d/参考项目/mind-map/web/src/assets/icon-font/iconfont.css" src/assets/mindmap/icon-font/
cp "/d/参考项目/mind-map/web/src/assets/img/structures/"*.jpg src/assets/mindmap/structures/
cp "/d/参考项目/mind-map/web/src/assets/img/图片加载失败.svg" src/assets/mindmap/image-broken.svg
```

- [ ] **Step 3: 精简 iconfont.css 的 @font-face**

把 `src/assets/mindmap/icon-font/iconfont.css` 开头的 `@font-face` 改为只引用 woff2：

```css
@font-face {
  font-family: 'iconfont';
  src: url('./iconfont.woff2') format('woff2');
}
```
其余 `.iconfont` 与 `.icon*:before` 规则原样保留。

- [ ] **Step 4: 验证与提交**

```bash
npx prettier --write src/assets/mindmap/icon-font/iconfont.css
git add package.json pnpm-lock.yaml src/assets/mindmap
git commit -m "chore(mindmap): 引入主题包与参考端图标字体、结构缩略图等静态资源"
```

---

### Task 2: 类型声明扩展

**Files:**
- Modify: `src/typings/simple-mind-map.d.ts`（整文件重写）

**Interfaces:**
- Produces: `MindMap` 类（主类常用方法 + 插件实例属性）、`MindMapNode`、`MindMapOptions`、`simple-mind-map/src/*` 与 `simple-mind-map-plugin-themes*` 通配声明

- [ ] **Step 1: 重写声明文件**

```ts
/**
 * simple-mind-map 0.14.0-fix.3 的最小类型声明。
 *
 * 库本身是 JS，此处只声明本项目用到的 API；插件实例属性（renderer / view / keyCommand /
 * associativeLine / painter / watermark / miniMap / search / formula / demonstrate / scrollbar）
 * 按 any 放行，调用点以参考端 web 代码为准。
 */
declare module 'simple-mind-map' {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  type Any = any

  export interface MindMapNode {
    uid: string
    isRoot: boolean
    isGeneralization: boolean
    layerIndex: number
    nodeData: { data: Record<string, Any>; children: Any[] }
    parent: MindMapNode | null
    children: MindMapNode[]
    width: number
    height: number
    left: number
    top: number
    getData(key?: string): Any
    getStyle(prop: string, root?: boolean): Any
    getSelfStyle(prop: string): Any
    setStyle(prop: string, value: Any): void
    setStyles(styles: Record<string, Any>): void
    setText(text: string, richText?: boolean, resetRichText?: boolean): void
    setImage(img: { url: string; title: string; width: number; height: number }): void
    setIcon(icons: string[]): void
    setHyperlink(link: string, title: string): void
    setNote(note: string): void
    setTag(tags: Array<string | { text: string; style?: Record<string, Any> }>): void
    setAttachment(url: string, name: string): void
    active(): void
    deactivate(): void
    removeExpandBtn(): void
    getRect(): { left: number; top: number; width: number; height: number }
    getRectInSvg(): { left: number; top: number; width: number; height: number }
    hasCustomStyle(): boolean
    [key: string]: Any
  }

  export interface MindMapOptions {
    el: HTMLElement
    data?: Any
    layout?: string
    theme?: string
    themeConfig?: Record<string, Any>
    viewData?: Any
    fit?: boolean
    readonly?: boolean
    [key: string]: Any
  }

  export default class MindMap {
    constructor(options: MindMapOptions)
    static usePlugin(plugin: Any, options?: Any): typeof MindMap
    static hasPlugin(plugin: Any): number
    static pluginList: Any[]
    static iconList: Any[]

    opt: Record<string, Any>
    el: HTMLElement
    width: number
    height: number
    elRect: DOMRect
    renderer: Any
    view: Any
    keyCommand: Any
    command: Any
    event: Any
    svg: Any
    draw: Any
    themeConfig: Record<string, Any>
    // 插件实例（注册后存在）
    doExport?: Any
    miniMap?: Any
    watermark?: Any
    search?: Any
    painter?: Any
    formula?: Any
    associativeLine?: Any
    demonstrate?: Any
    scrollbar?: Any
    select?: Any
    richText?: Any
    outerFrame?: Any
    rainbowLines?: Any

    on(event: string, listener: (...args: Any[]) => void): void
    off(event: string, listener: (...args: Any[]) => void): void
    emit(event: string, ...args: Any[]): void
    getData(withConfig?: boolean): Any
    setData(data: Any): void
    setFullData(data: Any): void
    updateData(data: Any): void
    render(callback?: () => void, source?: string): void
    reRender(callback?: () => void, source?: string): void
    resize(): void
    destroy(): void
    setTheme(theme: string, notRender?: boolean): void
    getTheme(): string
    setThemeConfig(config: Record<string, Any>, notRender?: boolean): void
    getThemeConfig(prop?: string): Any
    getCustomThemeConfig(): Record<string, Any>
    getLayout(): string
    setLayout(layout: string, notRender?: boolean): void
    getConfig(prop?: string): Any
    updateConfig(config: Record<string, Any>): void
    execCommand(name: string, ...args: Any[]): void
    setMode(mode: 'readonly' | 'edit'): void
    export(type: string, isDownload?: boolean, name?: string, ...args: Any[]): Promise<Any>
    addPlugin(plugin: Any, options?: Any): void
    removePlugin(plugin: Any): void
    toPos(x: number, y: number): { x: number; y: number }
    getElRectInfo(): void
    addCss(key: string, css: string): void
    removeCss(key: string): void
  }
}

declare module 'simple-mind-map/src/plugins/*' {
  const plugin: unknown
  export default plugin
}

declare module 'simple-mind-map/src/parse/xmind.js' {
  const xmind: {
    parseXmindFile(file: Blob, handleMultiCanvas?: (content: unknown[]) => Promise<unknown>): Promise<unknown>
    transformXmind(content: string, files: unknown[]): Promise<unknown>
  }
  export default xmind
}

declare module 'simple-mind-map/src/parse/markdown.js' {
  const markdown: { transformMarkdownTo(md: string): unknown }
  export default markdown
}

declare module 'simple-mind-map/src/parse/toMarkdown.js' {
  export function transformToMarkdown(root: unknown): string
}

declare module 'simple-mind-map/src/parse/toTxt.js' {
  export function transformToTxt(root: unknown): string
}

declare module 'simple-mind-map/src/svg/icons.js' {
  export const nodeIconList: Array<{ name: string; type: string; list: Array<{ name: string; icon: string }> }>
}

declare module 'simple-mind-map/src/utils/index.js' {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  type Any = any
  export function mergerIconList(list: Any[]): Any[]
  export function imgToDataUrl(src: string): Promise<string>
  export function isMobile(): boolean
  export function readBlob(blob: Blob): Promise<string>
  export function getTextFromHtml(html: string): string
  export function nodeRichTextToTextWithWrap(html: string): string
  export function textToNodeRichTextWithWrap(text: string): string
  export function simpleDeepClone<T>(v: T): T
  export function getVisibleColorFromTheme(config: Any): string
  export function createUid(): string
  export function throttle<T extends (...args: Any[]) => void>(fn: T, time?: number, ctx?: Any): T
}

declare module 'simple-mind-map/src/constants/constant.js' {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  export const CONSTANTS: any
  export const layoutList: Array<{ name: string; value: string }>
  export const layoutValueList: string[]
}

declare module 'simple-mind-map/src/utils/Lru.js' {
  export default class Lru<T> {
    constructor(max: number)
    get(key: string): T | undefined
    add(key: string, value: T): void
  }
}

declare module 'simple-mind-map-plugin-themes' {
  const Themes: { init(mindMap: unknown): void }
  export default Themes
}

declare module 'simple-mind-map-plugin-themes/themeList' {
  const themeList: Array<{ name: string; value: string; dark: boolean }>
  export default themeList
}

declare module 'simple-mind-map-plugin-themes/themeImgMap' {
  const themeImgMap: Record<string, string>
  export default themeImgMap
}

declare module 'simple-mind-map/dist/simpleMindMap.esm.css'
```

- [ ] **Step 2: 类型检查与提交**

```bash
npx vue-tsc --noEmit
git add src/typings/simple-mind-map.d.ts
git commit -m "chore(mindmap): 扩展 simple-mind-map 与主题包的类型声明"
```
预期：无报错（既有 `MindMapEditor.vue` 用到的 `on/getData/setData/resize/destroy` 仍被覆盖）。

---

### Task 3: Rust 落盘命令 `write_file_base64`

**Files:**
- Modify: `src-tauri/src/ipc.rs`（在 `data_dir` 命令之后追加）
- Modify: `src-tauri/src/main.rs`（`generate_handler!` 追加 `ipc::write_file_base64`）
- Modify: `src-tauri/Cargo.toml`（确认 `base64` 依赖；若无则 `cargo add base64@0.22`）
- Modify: `src/service/tauri.ts`（新增 `files.writeBase64`）

**Interfaces:**
- Produces: Rust `write_file_base64(path: String, base64: String) -> Result<String, String>`（返回写入的绝对路径）；前端 `api.files.writeBase64(path, base64)`

- [ ] **Step 1: 写失败测试**

在 `src-tauri/src/ipc.rs` 末尾追加：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_base64_decodes_and_writes() {
        let dir = std::env::temp_dir().join(format!("inkling-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("hello.txt");
        // "hello" 的 base64
        let written = write_base64_to_path(path.to_str().unwrap(), "aGVsbG8=").unwrap();
        assert_eq!(std::fs::read_to_string(&written).unwrap(), "hello");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn write_base64_rejects_bad_input() {
        let dir = std::env::temp_dir();
        let path = dir.join("inkling-bad.bin");
        assert!(write_base64_to_path(path.to_str().unwrap(), "***not base64***").is_err());
    }
}
```

- [ ] **Step 2: 运行确认失败**

```bash
cd D:/Inkling/src-tauri && cargo test write_base64 2>&1 | tail -5
```
预期：编译错误 `cannot find function write_base64_to_path`。

- [ ] **Step 3: 实现**

`Cargo.toml` 若无 `base64`：`cargo add base64@0.22`。在 `ipc.rs` 的 `data_dir` 命令之后追加：

```rust
/// 把 base64 内容解码后写到指定绝对路径（思维导图导出使用）。
///
/// 前端拿到库导出的 dataURL 后剥掉 `data:*;base64,` 前缀再传入；路径来自系统保存对话框，
/// 不做目录创建——对话框选出的目录必然存在。先写临时文件再重命名，避免半截文件。
fn write_base64_to_path(path: &str, base64_content: &str) -> Result<String, String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(base64_content.trim())
        .map_err(|e| format!("base64 解码失败: {e}"))?;
    let target = std::path::PathBuf::from(path);
    let temp = target.with_extension(format!(
        "{}.tmp",
        target.extension().and_then(|e| e.to_str()).unwrap_or("bin")
    ));
    std::fs::write(&temp, &bytes).map_err(|e| format!("写入文件失败: {e}"))?;
    std::fs::rename(&temp, &target).map_err(|e| format!("提交文件失败: {e}"))?;
    eprintln!("[files] 已写入 {} ({} 字节)", target.display(), bytes.len());
    Ok(target.to_string_lossy().to_string())
}

#[tauri::command]
pub fn write_file_base64(path: String, base64: String) -> Result<String, String> {
    write_base64_to_path(&path, &base64)
}
```

`main.rs` 的 `generate_handler!` 末尾追加 `ipc::write_file_base64`。

`src/service/tauri.ts` 的 `api` 里追加：

```ts
  /** 文件落盘（思维导图导出）。 */
  files: {
    /** 把 base64 内容写到绝对路径，返回写入的路径。 */
    writeBase64: (path: string, base64: string) => invoke<string>('write_file_base64', { path, base64 }),
  },
```

- [ ] **Step 4: 测试通过与提交**

```bash
cd D:/Inkling/src-tauri && cargo fmt && cargo test 2>&1 | grep -E "test result|warning|error"
cd D:/Inkling && npx vue-tsc --noEmit && npx prettier --check src/service/tauri.ts
git add src-tauri src/service/tauri.ts
git commit -m "feat(ipc): 新增 write_file_base64 命令供思维导图导出落盘"
```
预期：`test result: ok. 30 passed`（原 28 + 2），无 warning。

---

### Task 4: 持久化纯函数 `core/persistence.ts`

**Files:**
- Create: `src/windows/MindMap/core/persistence.ts`
- Modify: `src/editor/MindMapEditor.vue:31-41`（`parseData` 改用 `parseMindMapData(...).root`）

**Interfaces:**
- Produces:
  - `interface MindMapFullData { root: MindMapRoot; layout?: string; theme?: { template?: string; config?: Record<string, unknown> }; view?: unknown }`
  - `interface MindMapRoot { data: Record<string, unknown>; children: MindMapRoot[] }`
  - `parseMindMapData(source: string | null | undefined, rootText?: string): MindMapFullData`
  - `serializeMindMapData(data: MindMapFullData): string`
  - `createEmptyRoot(text: string): MindMapRoot`

- [ ] **Step 1: 实现**

```ts
/**
 * 思维导图文档数据的持久化格式。
 *
 * `note.mindmap_data` 存 `mindMap.getData(true)` 的 JSON：`{ root, layout, theme, view }`。
 * 历史数据（09-06 之前）只存了 root（`{ data, children }`），读取时按 root 兼容；
 * 一旦保存就升级为全量格式。纯函数，不依赖 Vue 与库。
 */
export interface MindMapRoot {
  data: Record<string, unknown>
  children: MindMapRoot[]
}

export interface MindMapFullData {
  root: MindMapRoot
  layout?: string
  theme?: { template?: string; config?: Record<string, unknown> }
  view?: unknown
}

export function createEmptyRoot(text: string): MindMapRoot {
  return { data: { text }, children: [] }
}

function isRoot(value: unknown): value is MindMapRoot {
  return (
    typeof value === 'object' &&
    value !== null &&
    typeof (value as MindMapRoot).data === 'object' &&
    (value as MindMapRoot).data !== null
  )
}

/** 解析持久化字符串；空串 / 非法 JSON / 结构不对时回退为一个可编辑的空根节点。 */
export function parseMindMapData(source: string | null | undefined, rootText = '中心主题'): MindMapFullData {
  if (!source?.trim()) return { root: createEmptyRoot(rootText) }
  let parsed: unknown
  try {
    parsed = JSON.parse(source)
  } catch {
    return { root: createEmptyRoot(rootText) }
  }
  if (typeof parsed !== 'object' || parsed === null) return { root: createEmptyRoot(rootText) }
  const candidate = parsed as Partial<MindMapFullData>
  // 全量格式：含合法 root。
  if (isRoot(candidate.root)) {
    return {
      root: candidate.root,
      layout: typeof candidate.layout === 'string' ? candidate.layout : undefined,
      theme: typeof candidate.theme === 'object' && candidate.theme !== null ? candidate.theme : undefined,
      view: candidate.view,
    }
  }
  // 旧格式：整个对象就是 root。
  if (isRoot(parsed)) return { root: parsed }
  return { root: createEmptyRoot(rootText) }
}

export function serializeMindMapData(data: MindMapFullData): string {
  return JSON.stringify(data)
}
```

- [ ] **Step 2: `MindMapEditor.vue` 改用它**

把 `MindMapEditor.vue` 中 `createDefaultData` / `parseData` 两个函数删掉，改为：

```ts
import { parseMindMapData } from '@/windows/MindMap/core/persistence'
// ...
function parseData(source: string | null): unknown {
  // 面板内的简易画布只关心节点树；全量格式里的主题/结构由独立导图窗口负责。
  return parseMindMapData(source, props.placeholder).root
}
```
`publishData` 仍发 `getData(false)`（该组件只在 NoteEditor 的导图页签里用，保持只写 root 的行为不变——独立窗口保存时会升级为全量格式）。

- [ ] **Step 3: 验证与提交**

```bash
npx vue-tsc --noEmit && npx prettier --write src/windows/MindMap/core/persistence.ts src/editor/MindMapEditor.vue
git add src/windows/MindMap/core/persistence.ts src/editor/MindMapEditor.vue
git commit -m "feat(mindmap): 文档数据升级为全量格式并兼容旧的仅 root 数据"
```

---

### Task 5: 事件总线、共享状态、本地配置、上下文注入

**Files:**
- Create: `src/windows/MindMap/core/bus.ts`
- Create: `src/windows/MindMap/core/store.ts`
- Create: `src/windows/MindMap/core/localConfig.ts`
- Create: `src/windows/MindMap/core/useMindMap.ts`

**Interfaces:**
- Produces:
  - `createBus(): Bus`，`Bus = { on(event, handler): () => void; off(event, handler): void; once(event, handler): void; emit(event, ...args): void }`
  - `createUiState(): MindMapUiState`（reactive）
  - `LocalConfig`、`MapConfig`、`loadLocalConfig / saveLocalConfig / loadMapConfig / saveMapConfig`、`DEFAULT_LOCAL_CONFIG`、`DEFAULT_MAP_CONFIG`
  - `provideMindMapContext(ctx)`、`useMindMap(): MindMapContext`

- [ ] **Step 1: bus.ts**

```ts
/**
 * 导图窗口的事件总线。
 *
 * 参考端用 Vue 2 的 `$bus`（全局 EventEmitter）把库事件转发给几十个组件，
 * 这里用 40 行自实现替代，不引入 mitt。事件名不做枚举约束：
 * 一半是库原生事件名（node_active / data_change …），另一半是窗口内自定义事件（showExport …）。
 */
export type BusHandler = (...args: unknown[]) => void

export interface Bus {
  on(event: string, handler: BusHandler): () => void
  off(event: string, handler: BusHandler): void
  once(event: string, handler: BusHandler): void
  emit(event: string, ...args: unknown[]): void
}

export function createBus(): Bus {
  const handlers = new Map<string, Set<BusHandler>>()
  const on = (event: string, handler: BusHandler): (() => void) => {
    if (!handlers.has(event)) handlers.set(event, new Set())
    handlers.get(event)!.add(handler)
    return () => off(event, handler)
  }
  const off = (event: string, handler: BusHandler): void => {
    handlers.get(event)?.delete(handler)
  }
  const once = (event: string, handler: BusHandler): void => {
    const wrapped: BusHandler = (...args) => {
      off(event, wrapped)
      handler(...args)
    }
    on(event, wrapped)
  }
  const emit = (event: string, ...args: unknown[]): void => {
    // 拷贝一份再遍历：handler 内部可能 off 自己。
    Array.from(handlers.get(event) ?? []).forEach((handler) => handler(...args))
  }
  return { on, off, once, emit }
}
```

- [ ] **Step 2: store.ts**

```ts
import { reactive } from 'vue'
import type { MindMapNode } from 'simple-mind-map'

/** 侧栏标识，与参考端 `activeSidebar` 取值一致。 */
export type SidebarName =
  | 'nodeStyle'
  | 'baseStyle'
  | 'theme'
  | 'structure'
  | 'outline'
  | 'setting'
  | 'shortcutKey'
  | 'nodeIconSidebar'
  | 'formulaSidebar'
  | 'nodeNoteSidebar'
  | ''

/**
 * 导图窗口内跨组件共享的 UI 状态（替代参考端 vuex 的非持久化部分）。
 * 持久化偏好见 localConfig.ts。
 */
export interface MindMapUiState {
  activeSidebar: SidebarName
  isReadonly: boolean
  isZenMode: boolean
  isOutlineEdit: boolean
  isSourceCodeEdit: boolean
  isDragOutlineTreeNode: boolean
  /** 导出时底部附加文字（导出对话框写，createMindMap 的 addContentToFooter 读）。 */
  extraTextOnExport: string
  /** 最近一次 node_active 的激活节点列表。 */
  activeNodes: MindMapNode[]
}

export function createUiState(): MindMapUiState {
  return reactive<MindMapUiState>({
    activeSidebar: '',
    isReadonly: false,
    isZenMode: false,
    isOutlineEdit: false,
    isSourceCodeEdit: false,
    isDragOutlineTreeNode: false,
    extraTextOnExport: '',
    activeNodes: [],
  })
}
```

- [ ] **Step 3: localConfig.ts**

```ts
import { logger } from '@/service/logger'

/**
 * 导图窗口的本机持久化配置（spec D5），两个 localStorage 键：
 * - localConfig：编辑器行为偏好（参考端 vuex.localConfig）；
 * - mapConfig：「设置」侧栏里的库实例配置（参考端 storeConfig），建实例时展开进 options。
 * 读写都包 try/catch：隐私模式或存储被清空时按默认值工作。
 */
const LOCAL_KEY = 'inkling.mindmap.localConfig'
const MAP_KEY = 'inkling.mindmap.config'

export interface LocalConfig {
  isZenMode: boolean
  openNodeRichText: boolean
  useLeftKeySelectionRightKeyDrag: boolean
  isShowScrollbar: boolean
  enableDragImport: boolean
}

export const DEFAULT_LOCAL_CONFIG: LocalConfig = {
  isZenMode: false,
  openNodeRichText: true,
  useLeftKeySelectionRightKeyDrag: false,
  isShowScrollbar: false,
  enableDragImport: true,
}

/** 库实例配置：键名与 simple-mind-map options 一致，值类型放宽为 unknown 由库校验。 */
export type MapConfig = Record<string, unknown>

export const DEFAULT_MAP_CONFIG: MapConfig = {
  enableFreeDrag: false,
  mousewheelAction: 'zoom',
  mousewheelZoomActionReverse: true,
  createNewNodeBehavior: 'default',
  openRealtimeRenderOnNodeTextEdit: true,
  enableAutoEnterTextEditWhenKeydown: true,
  isUseHandDrawnLikeStyle: false,
  isUseMomentum: true,
  alwaysShowExpandBtn: false,
  enableInheritAncestorLineStyle: false,
  imgTextMargin: 5,
  textContentMargin: 2,
  openPerformance: false,
  demonstrateConfig: { openBlankMode: false },
}

function read<T extends object>(key: string, fallback: T): T {
  try {
    const raw = localStorage.getItem(key)
    if (!raw) return { ...fallback }
    const parsed = JSON.parse(raw) as Partial<T>
    return { ...fallback, ...parsed }
  } catch (error) {
    logger.warn('mindmap', `读取 ${key} 失败，使用默认配置`, error)
    return { ...fallback }
  }
}

function write(key: string, value: object): void {
  try {
    localStorage.setItem(key, JSON.stringify(value))
  } catch (error) {
    logger.warn('mindmap', `写入 ${key} 失败`, error)
  }
}

export const loadLocalConfig = (): LocalConfig => read(LOCAL_KEY, DEFAULT_LOCAL_CONFIG)
export const saveLocalConfig = (config: LocalConfig): void => write(LOCAL_KEY, config)
export const loadMapConfig = (): MapConfig => read(MAP_KEY, DEFAULT_MAP_CONFIG)
export const saveMapConfig = (config: MapConfig): void => write(MAP_KEY, config)
```

- [ ] **Step 4: useMindMap.ts**

```ts
import { inject, provide, type ShallowRef } from 'vue'
import type MindMap from 'simple-mind-map'
import type { Bus } from './bus'
import type { LocalConfig, MapConfig } from './localConfig'
import type { MindMapUiState } from './store'

/**
 * 导图窗口上下文：MindMapApp 在根部 provide，其余组件用 useMindMap() 取。
 * `mindMap` 是 shallowRef——库实例内部结构巨大且自管理，不能被 Vue 深度代理。
 */
export interface MindMapContext {
  mindMap: ShallowRef<MindMap | null>
  bus: Bus
  ui: MindMapUiState
  /** reactive；改动后由 MindMapApp 统一落 localStorage 并同步到库。 */
  localConfig: LocalConfig
  mapConfig: MapConfig
}

const KEY = Symbol('inkling-mindmap-context')

export function provideMindMapContext(ctx: MindMapContext): void {
  provide(KEY, ctx)
}

export function useMindMap(): MindMapContext {
  const ctx = inject<MindMapContext | null>(KEY, null)
  if (!ctx) throw new Error('useMindMap 必须在 MindMapApp 子树内调用')
  return ctx
}
```

- [ ] **Step 5: 验证与提交**

```bash
npx vue-tsc --noEmit && npx prettier --write src/windows/MindMap/core
git add src/windows/MindMap/core
git commit -m "feat(mindmap): 事件总线、UI 共享状态、本机配置与上下文注入"
```

---

### Task 6: 插件注册、常量表、实例工厂

**Files:**
- Create: `src/windows/MindMap/core/plugins.ts`
- Create: `src/windows/MindMap/core/createMindMap.ts`
- Create: `src/windows/MindMap/core/clipboardText.ts`（参考端 `web/src/utils/handleClipboardText.js` 逐行移植为 TS，函数名与逻辑不变；`console.log` 改 `logger.debug`）
- Create: `src/windows/MindMap/constants/lists.ts`（参考端 `web/src/config/zh.js` 中除 `shortcutKeyList / layoutGroupList / store / langList` 外的全部常量：`fontFamilyList / fontSizeList / colorList / borderWidthList / borderDasharrayList / borderRadiusList / lineWidthList / lineHeightList / lineStyleMap / lineStyleList / rootLineKeepSameInCurveList / backgroundRepeatList / backgroundPositionList / backgroundSizeList / shapeListMap / shapeList / sidebarTriggerList / downTypeList / numberTypeList / numberLevelList / linearGradientDirList / alignList`，加 `as const` 与类型）
- Create: `src/windows/MindMap/constants/shortcuts.ts`（`shortcutKeyList`，`isMac` 用 `navigator.platform`）
- Create: `src/windows/MindMap/constants/layouts.ts`（`layoutGroupList` + `layoutImgMap`：用 `new URL('@/assets/mindmap/structures/x.jpg', import.meta.url).href`，或 `import x from '.../x.jpg'` 静态导入 14 张）
- Create: `src/windows/MindMap/constants/formulas.ts`（`formulaList`）
- Create: `src/windows/MindMap/constants/rainbow.ts`（`rainbowLinesOptions`，来自参考端 `config/constant.js`；另含 `supportLineStyleLayoutsMap / supportLineRadiusLayouts / supportNodeUseLineStyleLayouts / supportRootLineKeepSameInCurveLayouts`）
- Create: `src/windows/MindMap/constants/stickers.ts`（参考端 `config/icon.js` 原样拷贝为 `export const stickerIconGroups = [...]`，**只由 `import()` 异步引用**）

**Interfaces:**
- Produces:
  - `registerPlugins(): void`（幂等，注册 18 个常驻插件 + `Themes.init(MindMap)`）；`RichTextPlugin`、`ScrollbarPlugin` 导出
  - `createMindMap(params: CreateMindMapParams): MindMap`
  - `FORWARDED_EVENTS: readonly string[]`
  - `loadStickers(): Promise<unknown[]>`

- [ ] **Step 1: plugins.ts**

```ts
/**
 * 插件注册（等价参考端 simple-mind-map/full.js 与 Edit.vue 的 usePlugin 链）。
 *
 * RichText 与 Scrollbar 不在此常驻注册：它们随本机偏好动态 addPlugin / removePlugin。
 * `registerPlugins` 幂等——窗口热重载时模块会重新执行，重复 usePlugin 会被库忽略但主题包会重复 init。
 */
import MindMap from 'simple-mind-map'
import MiniMap from 'simple-mind-map/src/plugins/MiniMap.js'
import Watermark from 'simple-mind-map/src/plugins/Watermark.js'
import KeyboardNavigation from 'simple-mind-map/src/plugins/KeyboardNavigation.js'
import ExportPDF from 'simple-mind-map/src/plugins/ExportPDF.js'
import ExportXMind from 'simple-mind-map/src/plugins/ExportXMind.js'
import Export from 'simple-mind-map/src/plugins/Export.js'
import Drag from 'simple-mind-map/src/plugins/Drag.js'
import Select from 'simple-mind-map/src/plugins/Select.js'
import AssociativeLine from 'simple-mind-map/src/plugins/AssociativeLine.js'
import NodeImgAdjust from 'simple-mind-map/src/plugins/NodeImgAdjust.js'
import TouchEvent from 'simple-mind-map/src/plugins/TouchEvent.js'
import SearchPlugin from 'simple-mind-map/src/plugins/Search.js'
import Painter from 'simple-mind-map/src/plugins/Painter.js'
import Formula from 'simple-mind-map/src/plugins/Formula.js'
import RainbowLines from 'simple-mind-map/src/plugins/RainbowLines.js'
import Demonstrate from 'simple-mind-map/src/plugins/Demonstrate.js'
import OuterFrame from 'simple-mind-map/src/plugins/OuterFrame.js'
import MindMapLayoutPro from 'simple-mind-map/src/plugins/MindMapLayoutPro.js'
import NodeBase64ImageStorage from 'simple-mind-map/src/plugins/NodeBase64ImageStorage.js'
import RichText from 'simple-mind-map/src/plugins/RichText.js'
import Scrollbar from 'simple-mind-map/src/plugins/Scrollbar.js'
import Themes from 'simple-mind-map-plugin-themes'
import { logger } from '@/service/logger'

export const RichTextPlugin = RichText
export const ScrollbarPlugin = Scrollbar

let registered = false

export function registerPlugins(): void {
  if (registered) return
  registered = true
  MindMap.usePlugin(MiniMap)
    .usePlugin(Watermark)
    .usePlugin(Drag)
    .usePlugin(KeyboardNavigation)
    .usePlugin(ExportPDF)
    .usePlugin(ExportXMind)
    .usePlugin(Export)
    .usePlugin(Select)
    .usePlugin(AssociativeLine)
    .usePlugin(NodeImgAdjust)
    .usePlugin(TouchEvent)
    .usePlugin(SearchPlugin)
    .usePlugin(Painter)
    .usePlugin(Formula)
    .usePlugin(RainbowLines)
    .usePlugin(Demonstrate)
    .usePlugin(OuterFrame)
    .usePlugin(MindMapLayoutPro)
    .usePlugin(NodeBase64ImageStorage)
  Themes.init(MindMap)
  logger.info('mindmap', `已注册 ${MindMap.pluginList.length} 个插件与主题包`)
}
```

- [ ] **Step 2: createMindMap.ts**

```ts
import MindMap, { type MindMapOptions } from 'simple-mind-map'
import { logger } from '@/service/logger'
import brokenImage from '@/assets/mindmap/image-broken.svg'
import type { Bus } from './bus'
import { handleClipboardText } from './clipboardText'
import type { LocalConfig, MapConfig } from './localConfig'
import type { MindMapFullData } from './persistence'
import { registerPlugins, RichTextPlugin, ScrollbarPlugin } from './plugins'

/** 从库转发到 bus 的事件（参考端 Edit.vue 列表 + 本项目补充的 node_dblclick）。 */
export const FORWARDED_EVENTS = [
  'node_active',
  'data_change',
  'view_data_change',
  'back_forward',
  'node_contextmenu',
  'node_click',
  'node_dblclick',
  'draw_click',
  'expand_btn_click',
  'svg_mousedown',
  'mouseup',
  'mode_change',
  'node_tree_render_end',
  'rich_text_selection_change',
  'transforming-dom-to-images',
  'generalization_node_contextmenu',
  'painter_start',
  'painter_end',
  'scrollbar_change',
  'scale',
  'translate',
  'node_attachmentClick',
  'node_attachmentContextmenu',
  'demonstrate_jump',
  'exit_demonstrate',
  'node_note_dblclick',
  'node_mousedown',
  'node_img_dblclick',
  'search_info_change',
  'search_match_node_list_change',
  'set_data',
  'layout_change',
  'theme_change',
  'outer_frame_active',
  'outer_frame_delete',
  'associative_line_click',
  'node_tag_click',
] as const

export interface CreateMindMapParams {
  el: HTMLElement
  data: MindMapFullData
  localConfig: LocalConfig
  mapConfig: MapConfig
  bus: Bus
  /** 二次确认（naive-ui dialog 封装），resolve true 表示用户确认。 */
  confirm: (content: string) => Promise<boolean>
  /** 导出对话框设置的底部文字。 */
  extraTextOnExport: () => string
  onExportError: () => void
}

/**
 * 构造库实例（等价参考端 Edit.vue 的 init()，去掉多语言 / AI / 本地文件相关项）。
 * 全部库事件转发到 bus；RichText / Scrollbar 按本机偏好动态挂载。
 */
export function createMindMap(params: CreateMindMapParams): MindMap {
  registerPlugins()
  const { el, data, localConfig, mapConfig, bus } = params
  const options: MindMapOptions = {
    el,
    data: data.root,
    fit: false,
    layout: data.layout,
    theme: data.theme?.template,
    themeConfig: data.theme?.config,
    viewData: data.view,
    nodeTextEditZIndex: 1000,
    nodeNoteTooltipZIndex: 1000,
    customNoteContentShow: {
      show: (content: string, left: number, top: number, node: unknown) => bus.emit('showNoteContent', content, left, top, node),
      hide: () => bus.emit('hideNoteContent'),
    },
    ...mapConfig,
    useLeftKeySelectionRightKeyDrag: localConfig.useLeftKeySelectionRightKeyDrag,
    customInnerElsAppendTo: null,
    customHandleClipboardText: handleClipboardText,
    defaultNodeImage: brokenImage,
    initRootNodePosition: ['center', 'center'],
    handleIsSplitByWrapOnPasteCreateNewNode: () => params.confirm('是否按换行自动分割节点？'),
    errorHandler: (code: string, error: unknown) => {
      logger.error('mindmap', `库错误 ${code}`, error)
      if (code === 'export_error') params.onExportError()
    },
    addContentToFooter: () => {
      const text = params.extraTextOnExport().trim()
      if (!text) return null
      const footer = document.createElement('div')
      footer.className = 'footer'
      footer.innerHTML = text
      return {
        el: footer,
        cssText: `.footer{width:100%;height:30px;display:flex;justify-content:center;align-items:center;font-size:12px;color:#979797;}`,
        height: 30,
      }
    },
    expandBtnNumHandler: (num: number) => (num >= 100 ? '…' : num),
    beforeDeleteNodeImg: () => params.confirm('是否确认删除该节点图片？').then((ok) => !ok),
  }
  logger.info('mindmap', `创建实例 layout=${data.layout ?? '(默认)'} theme=${data.theme?.template ?? '(默认)'}`)
  const mindMap = new MindMap(options)
  FORWARDED_EVENTS.forEach((event) => mindMap.on(event, (...args: unknown[]) => bus.emit(event, ...args)))
  if (localConfig.openNodeRichText) mindMap.addPlugin(RichTextPlugin)
  if (localConfig.isShowScrollbar) mindMap.addPlugin(ScrollbarPlugin)
  return mindMap
}

/** 富文本插件动态挂卸（设置侧栏切换）。 */
export function toggleRichText(mindMap: MindMap, enabled: boolean): void {
  logger.info('mindmap', `富文本插件 ${enabled ? '挂载' : '卸载'}`)
  if (enabled) mindMap.addPlugin(RichTextPlugin)
  else mindMap.removePlugin(RichTextPlugin)
}

export function toggleScrollbar(mindMap: MindMap, enabled: boolean): void {
  logger.info('mindmap', `滚动条插件 ${enabled ? '挂载' : '卸载'}`)
  if (enabled) mindMap.addPlugin(ScrollbarPlugin)
  else mindMap.removePlugin(ScrollbarPlugin)
}
```
`beforeDeleteNodeImg` 语义：参考端 `resolve(false)` 表示**允许删除**（确认后），`resolve(true)` 表示阻止；上面的 `!ok` 正是这个映射。

`import brokenImage from '@/assets/mindmap/image-broken.svg'` 需要 `src/vite-env.d.ts` 里有 `/// <reference types="vite/client" />`（检查是否已存在，无则新建该文件）。

- [ ] **Step 3: constants/*.ts 与 stickers 异步加载**

`constants/stickers.ts` 末尾不导出加载函数；在 `constants/index.ts`：

```ts
/** 贴纸组 560KB base64，只在打开图标侧栏时才加载。 */
export async function loadStickers(): Promise<unknown[]> {
  const module = await import('./stickers')
  return module.stickerIconGroups
}
```

- [ ] **Step 4: 验证与提交**

```bash
npx vue-tsc --noEmit && npx prettier --write src/windows/MindMap
git add src/windows/MindMap src/vite-env.d.ts
git commit -m "feat(mindmap): 插件注册、实例工厂与参考端常量表移植"
```

---

### Task 7: 窗口壳重构（naive-ui 主题、画布宿主、持久化与自动保存）

**Files:**
- Create: `src/windows/MindMap/core/naiveTheme.ts`
- Create: `src/styles/mindmap.css`
- Modify: `src/windows/mindmap.ts`（引入 `mindmap.css` 与 iconfont.css）
- Modify: `src/windows/MindMap/MindMapApp.vue`（整文件重写）
- Modify: `src/styles/window-fit.css:83-135`（删除 `.mindmap-*` 旧规则，迁到 `mindmap.css`）

**Interfaces:**
- Consumes: Task 4–6 全部
- Produces: 组件树骨架——所有后续 Task 的组件都挂在 `MindMapApp.vue` 的 `<template v-if="mindMap">` 区域；`bus` 事件 `saved`、`setData`（导入时全量替换）、`execCommand`、`export`、`startPainter`、`createAssociativeLine`、`startTextEdit`、`endTextEdit`、`showLoading` / `hideLoading`

- [ ] **Step 1: naiveTheme.ts**

```ts
import { darkTheme, type GlobalThemeOverrides } from 'naive-ui'

/**
 * naive-ui 主题映射到 Inkling 令牌。
 * 只在导图窗口用；颜色取 CSS 变量的运行时值（主题切换时 MindMapApp 重新计算）。
 */
export const naiveDark = darkTheme

function cssVar(name: string, fallback: string): string {
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim()
  return value || fallback
}

export function buildThemeOverrides(): GlobalThemeOverrides {
  const accent = cssVar('--accent', '#6c8cff')
  const text = cssVar('--text', '#e8e8f0')
  const textDim = cssVar('--text-dim', '#9a9ab0')
  const menuBg = cssVar('--menu-bg', 'rgba(28,30,44,0.96)')
  return {
    common: {
      primaryColor: accent,
      primaryColorHover: accent,
      primaryColorPressed: accent,
      textColorBase: text,
      textColor1: text,
      textColor2: text,
      textColor3: textDim,
      popoverColor: menuBg,
      modalColor: menuBg,
      cardColor: menuBg,
      inputColor: 'rgba(255,255,255,0.06)',
      borderColor: 'rgba(255,255,255,0.14)',
      borderRadius: '8px',
      fontSize: '13px',
    },
  }
}
```

- [ ] **Step 2: MindMapApp.vue 结构**

保留现有 header（标题 / 未保存标记 / 标签 / 关闭 / 保存）与 `save()` / `close()` / `saveTags()` / 参数拉取逻辑，改动点：

```vue
<script setup lang="ts">
// …既有 import 保留，去掉 MindMapEditor，新增：
import { NConfigProvider, NDialogProvider, zhCN, dateZhCN } from 'naive-ui'
import { onBeforeUnmount, shallowRef } from 'vue'
import { createBus } from './core/bus'
import { createMindMap, toggleRichText, toggleScrollbar } from './core/createMindMap'
import { loadLocalConfig, loadMapConfig, saveLocalConfig, saveMapConfig } from './core/localConfig'
import { buildThemeOverrides, naiveDark } from './core/naiveTheme'
import { parseMindMapData, serializeMindMapData, type MindMapFullData } from './core/persistence'
import { createUiState } from './core/store'
import { provideMindMapContext } from './core/useMindMap'
import MindMapStage from './MindMapStage.vue'

const bus = createBus()
const ui = createUiState()
const localConfig = reactive(loadLocalConfig())
const mapConfig = reactive(loadMapConfig())
const mindMap = shallowRef<MindMap | null>(null)
provideMindMapContext({ mindMap, bus, ui, localConfig, mapConfig })

const themeOverrides = ref(buildThemeOverrides())
watch(() => settings.value.theme, () => nextTick(() => (themeOverrides.value = buildThemeOverrides())))

// 本机配置改动：落盘 + 同步到库
watch(localConfig, (next) => saveLocalConfig({ ...next }), { deep: true })
watch(() => localConfig.openNodeRichText, (v) => mindMap.value && toggleRichText(mindMap.value, v))
watch(() => localConfig.isShowScrollbar, (v) => mindMap.value && toggleScrollbar(mindMap.value, v))
watch(() => localConfig.useLeftKeySelectionRightKeyDrag, (v) => mindMap.value?.updateConfig({ useLeftKeySelectionRightKeyDrag: v }))
watch(mapConfig, (next) => { saveMapConfig({ ...next }); mindMap.value?.updateConfig({ ...next }) }, { deep: true })
</script>
```

画布宿主拆到 `MindMapStage.vue`（同目录）：负责 `ref host`、等尺寸就绪后 `createMindMap`、`ResizeObserver → mindMap.resize()`、销毁；通过 `emit('created', mindMap)` 回传。`MindMapApp` 收到后 `mindMap.value = instance`，并绑定：

```ts
function bindSaveEvents(instance: MindMap): void {
  bus.on('data_change', () => markDirtyAndAutosave())
  bus.on('view_data_change', () => markDirtyAndAutosave(true))
  bus.on('setData', (data) => {
    // 导入：含 root 用全量，否则纯节点
    const full = data as Partial<MindMapFullData>
    if (full.root) instance.setFullData(full)
    else instance.setData(data)
    instance.view.reset()
    markDirtyAndAutosave()
  })
  bus.on('execCommand', (...args) => instance.execCommand(...(args as [string, ...unknown[]])))
  bus.on('export', async (...args) => {
    try { await instance.export(...(args as [string, boolean, string])) } catch (error) { logger.error('mindmap', '导出失败', error) }
  })
  bus.on('startPainter', () => instance.painter?.startPainter())
  bus.on('createAssociativeLine', () => instance.associativeLine?.createLineFromActiveNode())
  bus.on('startTextEdit', () => instance.renderer.startTextEdit())
  bus.on('endTextEdit', () => instance.renderer.endTextEdit())
  bus.on('node_active', (_node, list) => { ui.activeNodes = [...(list as MindMapNode[])] })
  bus.on('mode_change', (mode) => { ui.isReadonly = mode === 'readonly' })
  instance.keyCommand.addShortcut('Control+s', () => void save())
}
```

自动保存（spec D6）：

```ts
let autosaveTimer: ReturnType<typeof setTimeout> | null = null
/** view_data_change 只改视图位置：仍要保存（视图是全量数据一部分），但不标记为「未保存」打扰用户。 */
function markDirtyAndAutosave(viewOnly = false): void {
  const instance = mindMap.value
  if (!instance) return
  mindmapData.value = serializeMindMapData(instance.getData(true) as MindMapFullData)
  if (!viewOnly) dirty.value = true
  if (!noteId.value) return // 新建导图首次必须手动保存
  if (autosaveTimer) clearTimeout(autosaveTimer)
  autosaveTimer = setTimeout(() => { autosaveTimer = null; void save(true) }, 1500)
}
```
`save(silent = false)`：silent 时不弹 Toast，只写日志 `logger.info('mindmap', '自动保存 id=…')`。

模板：

```vue
<template>
  <NConfigProvider :theme="naiveDark" :theme-overrides="themeOverrides" :locale="zhCN" :date-locale="dateZhCN">
    <NDialogProvider>
      <div class="mindmap-window" :class="{ zen: ui.isZenMode }">
        <header class="mindmap-bar">…既有…</header>
        <div class="mm-stage">
          <MindMapStage v-if="ready" :data="initialData" @created="onCreated" />
          <!-- 后续 Task 在此追加：Toolbar / NavigatorToolbar / SidebarTrigger / 各侧栏 / 各浮层 / 各对话框 -->
        </div>
        <TagManagerModal … />
        <ToastHost />
      </div>
    </NDialogProvider>
  </NConfigProvider>
</template>
```
`initialData = parseMindMapData(note.mindmap_data)` 在 `note` 就绪时计算一次（保持现有「参数未就绪不渲染」逻辑）。`.mm-stage { position: relative; flex: 1; min-height: 0; }`，`.mm-canvas { position:absolute; inset:0; }`。

- [ ] **Step 3: mindmap.css 与入口**

`src/styles/mindmap.css`：迁入原 `window-fit.css` 的 `.mindmap-*` 规则，新增 `.mm-stage / .mm-canvas`，以及 `.mm-*` 命名前缀供后续组件使用（浮层统一 `background: var(--menu-bg); border:1px solid rgba(var(--wsa),0.14); border-radius:10px; box-shadow:0 10px 30px rgba(0,0,0,0.5); backdrop-filter: blur(20px)`）。`src/windows/mindmap.ts` 新增 `import '@/assets/mindmap/icon-font/iconfont.css'` 与 `import '@/styles/mindmap.css'`。

- [ ] **Step 4: 实机验证**

`pnpm tauri:dev` → 主窗口笔记页打开一张已有导图：节点树正常渲染；切主题（后续 Task 才有 UI，此步用控制台不可行，改为：在 `MindMapStage` 创建后临时 `instance.setTheme('dark')` 验证 `getData(true)` 落库再重开仍是 dark，验证后移除临时代码）。旧格式数据（只有 root 的笔记）打开不报错。

- [ ] **Step 5: 提交**

```bash
npx vue-tsc --noEmit && npx prettier --check "src/**/*.{vue,ts,css}"
git add src/windows/MindMap src/windows/mindmap.ts src/styles/mindmap.css src/styles/window-fit.css
git commit -m "feat(mindmap): 窗口壳重构——naive-ui 主题映射、画布宿主、全量持久化与自动保存"
```

---

# P1 · 编辑主干

**通用移植映射表（本阶段起所有 UI Task 适用）**

| 参考端 | 本项目 |
|---|---|
| `el-dialog` | `n-modal` `preset="card"` |
| `el-select / el-option` | `n-select :options` |
| `el-input / el-input-number` | `n-input / n-input-number` |
| `el-switch` | `n-switch` |
| `el-slider` | `n-slider` |
| `el-color-picker` | `widgets/ColorField.vue`（色板 `colorList` + `n-color-picker` 「更多颜色」） |
| `el-tabs` | `n-tabs` |
| `el-tooltip` | `n-tooltip` |
| `el-popover` | `n-popover` |
| `el-dropdown` | `n-dropdown` |
| `el-radio-group` | `n-radio-group` |
| `el-upload` | 原生 `<input type="file">` |
| `el-tree` | 自实现（大纲用 `sidebars/OutlineTree.vue`，见 Task 21） |
| `this.$message` | `useToast().toast` |
| `this.$confirm` | `useDialog().warning({ title:'提示', content, positiveText:'是', negativeText:'否' })` 包成 Promise<boolean> —— 封装在 `core/confirm.ts` 的 `useConfirm()` |
| `this.$notify` | `toast(text, 4000)` |
| `mapState(isDark)` | 删除（沿用 Inkling 令牌） |
| `mapState(localConfig.x)` | `useMindMap().localConfig.x` |
| `setActiveSidebar(x)` | `ui.activeSidebar = x` |
| `this.mindMap` | `useMindMap().mindMap.value!`（组件仅在 `v-if="mindMap"` 内渲染） |
| `created()` 里 `$bus.$on` | `onMounted` 里 `bus.on`，返回的 off 函数收集到数组，`onBeforeUnmount` 统一调用 |
| 图标 `class="iconfont iconxxx"` | 原样保留（iconfont.css 已引入） |

每个 UI Task 的固定收尾步骤（下文不再重复写）：
1. `npx vue-tsc --noEmit && npx prettier --write <files>`
2. `pnpm tauri:dev` 实机按该 Task 的「验收」列表逐项操作
3. `git add <files> && git commit -m "feat(mindmap): <该 Task 标题>"`

### Task 8: 侧栏壳、触发条与通用表单件

**Files:**
- Create: `src/windows/MindMap/sidebars/SidebarShell.vue`（移植 `Sidebar.vue`：`props: title, name: SidebarName`；`show = computed(() => ui.activeSidebar === props.name)`；关闭按钮把 `ui.activeSidebar = ''`；zIndex 递增用模块级计数器；宽 300px，右侧抽屉，`.mm-sidebar` 样式）
- Create: `src/windows/MindMap/chrome/SidebarTrigger.vue`（移植 `SidebarTrigger.vue`：右侧竖排按钮列 `sidebarTriggerList`，点击切换 `ui.activeSidebar`；禅模式隐藏）
- Create: `src/windows/MindMap/core/confirm.ts`（`useConfirm(): (content: string, title?: string) => Promise<boolean>`，基于 `useDialog`）
- Create: `src/windows/MindMap/widgets/FieldRow.vue`（label + 控件横排）、`ColorField.vue`（36 色色板 + `n-color-picker`，`v-model:value`）、`SelectField.vue`、`NumberField.vue`、`SwitchField.vue`

**验收**：点击触发条各项，对应空侧栏滑出/关闭；同一时刻只开一个。

### Task 9: 工具栏与节点按钮

**Files:**
- Create: `src/windows/MindMap/chrome/Toolbar.vue`（移植 `Toolbar.vue` 去掉目录/新建/打开/另存为/AI；保留：`NodeButtons` 列表 + 导入 / 导出按钮；窗口窄于 1000px 时溢出项进「更多」`n-popover`——参考端 `computeToolbarShow` 逻辑原样移植）
- Create: `src/windows/MindMap/chrome/NodeButtons.vue`（移植 `ToolbarNodeBtnList.vue`：按钮列表与禁用条件 `readonly / hasRoot / hasGeneralization / backEnd / forwardEnd / isInPainter` 原样；事件：`bus.emit('execCommand','BACK'|'FORWARD'|'INSERT_NODE'|'INSERT_CHILD_NODE'|'REMOVE_NODE'|'ADD_GENERALIZATION'|'ADD_OUTER_FRAME')`、`bus.emit('startPainter')`、`bus.emit('showNodeImage'|'showNodeLink'|'showNodeNote'|'showNodeTag')`、`ui.activeSidebar = 'nodeIconSidebar' | 'formulaSidebar'`、`bus.emit('selectAttachment', ui.activeNodes)`；订阅 `mode_change / node_active / back_forward / painter_start / painter_end`）
- Modify: `MindMapApp.vue` 挂载 `<Toolbar v-if="!ui.isZenMode" />`

**验收**：选中节点后 同级/子节点/删除 生效；根节点时同级禁用；回退/前进随历史变灰；格式刷进入态高亮。

### Task 10: 右键菜单

**Files:**
- Create: `src/windows/MindMap/popups/ContextMenu.vue`（移植 `Contextmenu.vue` 全部：`type: 'node' | 'svg'`；订阅 `node_contextmenu`（记录 node、定位 `left/top`，右/下越界翻转）、`node_click`、`draw_click`、`expand_btn_click`、`svg_mousedown`（右键时 `type='svg'`）、`mouseup`；`exec(key, disabled, ...args)` 的 switch 分支原样：`COPY_NODE→renderer.copy()`、`CUT_NODE→renderer.cut()`、`PASTE_NODE→renderer.paste()`、`RETURN_CENTER→renderer.setRootNodeCenter()`、`TOGGLE_ZEN_MODE→localConfig.isZenMode=!…`、`FIT_CANVAS→view.fit()`、`REMOVE_HYPERLINK→node.setHyperlink('','')`、`REMOVE_NOTE→node.setNote('')`、`EXPORT_CUR_NODE_TO_PNG→bus.emit('export','png',true,node.getData('text'),false,node)`、`UNEXPAND_ALL/EXPAND_ALL/UNEXPAND_TO_LEVEL/其余→bus.emit('execCommand',key,...)`；「复制到剪贴板」子菜单：smm/json→`JSON.stringify(getData(true))`、md→`transformToMarkdown(getData())`、txt→`transformToTxt(getData())`、png→`export('png',false)` 转 Blob 写 `navigator.clipboard.write`；「编号其子节点」→ `numberTypeList/numberLevelList` 二级菜单 → `node.setStyle` 与参考端 `setNumber` 逻辑一致（`mindMap.execCommand('SET_NODE_DATA', node, { number: {...} })`）；「添加/删除待办」→ `SET_NODE_DATA` 写 `checkbox`；「链接到指定节点」→ `bus.emit('showNodeLink', node)`（Task 34 实现对话框）
- Modify: `MindMapApp.vue` 挂载

**验收**：节点右键 / 画布右键两套菜单每项可点；「展开到第 N 级」六项生效；复制到剪贴板五种格式粘贴到记事本可见。

### Task 11: 快捷键侧栏与统计

**Files:**
- Create: `src/windows/MindMap/sidebars/ShortcutSidebar.vue`（`SidebarShell name="shortcutKey"`，渲染 `constants/shortcuts.ts`）
- Create: `src/windows/MindMap/chrome/Count.vue`（移植 `Count.vue`：订阅 `data_change`/`node_tree_render_end` 后遍历 `mindMap.renderer.renderTree` 统计字数与节点数；左下角）

**验收**：编辑节点后字数/节点数实时变化。

### Task 12: 导入

**Files:**
- Create: `src/windows/MindMap/dialogs/ImportDialog.vue`（移植 `Import.vue`：`n-modal`；`<input type="file" accept=".smm,.json,.xmind,.md">`；`handleSmm / handleXmind / handleMd` 原样，成功 `bus.emit('setData', data)` + `toast('导入成功')`；xmind 多画布 `n-modal` 选择；Markdown 粘贴文本导入页签（参考端 `mdImportDialogTitle`）；订阅 `showImport`、`importFile`）
- Modify: `Toolbar.vue` 的导入按钮 `bus.emit('showImport')`

**验收**：分别导入一个 .smm、.json、.xmind（含多画布）、.md；失败文件提示「文件解析失败」。

### Task 13: 导出

**Files:**
- Create: `src/windows/MindMap/core/exportFile.ts`：

```ts
import { save } from '@tauri-apps/plugin-dialog'
import { api } from '@/service/tauri'
import { logger } from '@/service/logger'

const EXT_NAME: Record<string, string> = { smm: 'SMM 文件', json: 'JSON', png: 'PNG 图片', svg: 'SVG', pdf: 'PDF', md: 'Markdown', xmind: 'XMind', txt: '文本' }

/** dataURL → 系统保存框 → Rust 落盘；用户取消返回 null。 */
export async function saveDataUrl(dataUrl: string, fileName: string, ext: string): Promise<string | null> {
  const base64 = dataUrl.replace(/^data:[^,]*,/, '')
  const path = await save({ defaultPath: `${fileName}.${ext}`, filters: [{ name: EXT_NAME[ext] ?? ext, extensions: [ext] }] })
  if (!path) { logger.info('mindmap', '用户取消导出'); return null }
  return api.files.writeBase64(path, base64)
}
```
- Create: `src/windows/MindMap/dialogs/ExportDialog.vue`（移植 `Export.vue`：格式列表 `downTypeList` 过滤掉 `mm/xlsx`；选项：文件名、含配置（smm/json）、透明背景（png/pdf）、完整背景图（png/pdf 且非透明）、内边距（→ `mindMap.updateConfig({exportPaddingX, exportPaddingY})`）、底部文字（→ `ui.extraTextOnExport`）；`confirm()` 按参考端各分支组装参数，但 `isDownload=false` 拿 dataURL：`const url = await mindMap.export(type, false, name, ...rest)`，再 `saveDataUrl(url, name, ext)`（png 的 `imageFormat` 分支 ext 用 `png`；svg 传入参考端那段重置 CSS 字串）；Toast「已导出到 <path>」）
- Modify: `Toolbar.vue` 的导出按钮 `bus.emit('showExport')`

**验收**：八种格式各导出一次到桌面并能用对应软件打开；透明背景 PNG 生效；取消保存框不报错。

### Task 14: 节点图片 / 超链接 / 备注 / 标签对话框

**Files:**
- Create: `dialogs/NodeImageDialog.vue`（移植 `NodeImage.vue`：`<input type=file accept=image/*>` → FileReader dataURL + `new Image()` 取宽高；图片标题；确认 `node.setImage({url,title,width,height})`；订阅 `showNodeImage`；回显当前节点 `node.getData('image')`）
- Create: `dialogs/NodeHyperlinkDialog.vue`（`node.setHyperlink(link, title)`；订阅 `showNodeLink`）
- Create: `dialogs/NodeNoteDialog.vue`（移植 `NodeNote.vue`：多行 `n-input`；`node.setNote(note)`；订阅 `showNodeNote`）
- Create: `dialogs/NodeTagDialog.vue`（移植 `NodeTag.vue`：回车添加、最多 `mindMap.opt.maxTag` 个、点 ✕ 删除；`node.setTag(list)`；订阅 `showNodeTag`）

**验收**：四种内容加到节点上、再打开对话框能回显并修改；右键「移除超链接 / 备注」生效。

---

# P2 · 侧栏

### Task 15: 结构侧栏
- Create: `sidebars/StructureSidebar.vue`（移植 `Structure.vue`：`layoutGroupList` 分组 + 缩略图；`mindMap.setLayout(v)`；订阅 `layout_change` 回显；日志「结构切换为 …」）

### Task 16: 主题侧栏
- Create: `sidebars/ThemeSidebar.vue`（移植 `Theme.vue`：分组 经典 / 深色 / 朴素 用 `themeList.filter(dark)` 等逻辑原样；预览图 `themeImgMap`；`useTheme(theme)`：若 `Object.keys(mindMap.getCustomThemeConfig()).length > 0` 弹确认「你当前自定义过基础样式，是否覆盖？」→ 覆盖则 `setThemeConfig({}, true)`；`mindMap.setTheme(value)`；订阅 `theme_change` 回显）

### Task 17: 基础样式侧栏（最大）
- Create: `sidebars/BaseStyleSidebar.vue`（移植 `BaseStyle.vue` 1332 行：分区 背景（颜色 / 图片上传 / 内置背景图 **不做**（参考端 bgList 来自其服务端）/ 重复 / 位置 / 大小）、连线（颜色 / 粗细 / 风格 `lineStyleList` 受 `supportLineStyleLayoutsMap` 限制 / 圆角 `supportLineRadiusLayouts` / 根节点连线起始位置 / 括号形态 `rootLineKeepSameInCurveList` 受 `supportRootLineKeepSameInCurveLayouts` 限制 / 概要连线 / 箭头 / 流动效果与周期）、彩虹线条（`rainbowLinesOptions` → `mindMap.rainbowLines.updateRainLinesConfig({open, colorsList})`）、节点内外边距（`nodePadding / nodeMargin` 按层级 root / second / node）、图标大小、字体字号（root/second/node 三级）、节点边框风格 `nodeUseLineStyle` 受 `supportNodeUseLineStyleLayouts` 限制、关联线（粗细 / 颜色 / 激活粗细 / 激活颜色 / 文字字体字号颜色）、外框内边距（`mindMap.updateConfig({outerFramePaddingX/Y})`）。所有写入走 `mindMap.setThemeConfig({ [prop]: value })`；读取走 `mindMap.getThemeConfig(prop)`；订阅 `theme_change` 与 `layout_change` 刷新可用项）

### Task 18: 节点样式侧栏
- Create: `sidebars/NodeStyleSidebar.vue`（移植 `Style.vue` 922 行：常态 / 选中态两个页签（选中态属性名加 `active` 前缀由 `getStyleProp`）；文字（字体 / 字号 / 行高 / 颜色 / 加粗 / 斜体 / 划线）、边框（颜色 / 样式 `borderDasharrayList` / 宽度 / 圆角）、背景（颜色 / 渐变开关 + 起止色 + 方向 `linearGradientDirList`）、形状 `shapeList`（`node.setShape` → `mindMap.execCommand('SET_NODE_SHAPE', node, shape)`）、线条（颜色 / 粗细 / 虚线 / 箭头位置）、内边距、图片布局 `imgPlacement`、标签（`tagPlacement`）；多选时对 `ui.activeNodes` 全部 `setStyle`；订阅 `node_active` 回显首个节点 `getStyle(prop, isRoot)`；无选中节点显示「请选择一个节点」）

### Task 19: 设置侧栏（含水印）
- Create: `sidebars/SettingSidebar.vue`（移植 `Setting.vue`：每项对应 `mapConfig.x`（Task 5 的 reactive，改动自动 `updateConfig` + 落盘）；`openNodeRichText` 切换：先 `mindMap.renderer.textEdit.hideEditTextBox()`，弹确认「该操作会清空所有历史修改记录…是否继续？」，确认后 `localConfig.openNodeRichText = v`（MindMapApp 的 watch 完成挂卸）；`isShowScrollbar / useLeftKeySelectionRightKeyDrag / enableDragImport` 写 `localConfig`；水印：`show / onlyExport / text / lineSpacing / textSpacing / angle / textStyle{color,opacity,fontSize} / belowNode` → `mindMap.watermark.updateWatermark({...})` 再 `mapConfig.watermarkConfig = mindMap.getConfig('watermarkConfig')`；`show=false` 时 text 置空）

### Task 20: 大纲侧栏
- Create: `sidebars/OutlineTree.vue`（自实现树：递归组件，节点行 = 展开箭头 + `contenteditable` 文本；数据来自 `mindMap.renderer.renderTree` 转换（参考端 `refresh()` 的 `walk`：`{ label: textToHtml, uid, children, data }`）；编辑失焦 → `node.setText`（富文本时 `textToNodeRichTextWithWrap`）；Tab 插入子级 `INSERT_CHILD_NODE`、Enter 同级 `INSERT_NODE`、Shift+Tab `MOVE_UP_ONE_LEVEL`、Delete `REMOVE_NODE`（参考端 `outline` 快捷键表）；拖拽排序 → `MOVE_NODE_TO` / `INSERT_AFTER`（HTML5 dnd，拖拽中 `ui.isDragOutlineTreeNode = true`）；点击节点 → `mindMap.execCommand('GO_TARGET_NODE', uid)`）
- Create: `sidebars/OutlineSidebar.vue`（`SidebarShell name="outline"` + 顶部「全屏编辑」按钮 → `ui.isOutlineEdit = true`（Task 33）+ `<OutlineTree>`；订阅 `data_change` 刷新）
- Create: `src/styles/mindmap-outline.css`（参考端 `style/outlineTree.less` 移植为 CSS）

### Task 21: 图标与贴纸侧栏
- Create: `sidebars/IconSidebar.vue`（移植 `NodeIconSidebar.vue`：两页签 图标 / 贴纸；图标组 = `mergerIconList([...nodeIconList, ...await loadStickers()])`（参考端就是把 icon.js 合进图标组；贴纸页签 `image.js` **不做**，页签隐藏并在文件头注释注明 D9）；点击图标：`iconList` 里同 type 替换、同 key 取消，`node.setIcon(list)`；订阅 `node_active` 回显）

### Task 22: 公式侧栏
- Create: `sidebars/FormulaSidebar.vue`（移植 `FormulaSidebar.vue`：LaTeX 输入 + `formulaList` 常用公式点选；完成 → `mindMap.execCommand('INSERT_FORMULA', latex)`；非富文本模式显示提示「非富文本模式下不支持插入公式」并禁用；打开时若有激活节点回显其公式）

### Task 23: 备注侧栏
- Create: `sidebars/NoteSidebar.vue`（移植 `NodeNoteSidebar.vue`：与 Task 14 的备注对话框共享编辑逻辑，此处为常驻侧栏形态，`node_active` 时回显、失焦保存 `node.setNote`）

---

# P3 · 画布层

### Task 24: 导航工具栏、缩放、鼠标行为、全屏、演示
- Create: `chrome/NavigatorToolbar.vue`（移植 `NavigatorToolbar.vue` 去掉 语言 / 暗色 / 源码入口注释 / 下载客户端 / 官网 / 版本：结构 `n-select`（`layoutList` 中文名映射自 `constants/layouts.ts`）→ `setLayout`；回到根节点 `renderer.setRootNodeCenter()`；搜索 `bus.emit('showSearch')`；`<Scale>`；小地图开关 `bus.emit('toggleMiniMap')`；只读切换 `mindMap.setMode(readonly ? 'edit' : 'readonly')`（订阅 `mode_change`）；`<MouseAction>`；`<Fullscreen>`；`<Demonstrate>`；更多 `n-dropdown`：快捷键 → `ui.activeSidebar='shortcutKey'`、源码编辑 → `ui.isSourceCodeEdit=true`）
- Create: `chrome/Scale.vue`（移植：`view.narrow()` / `view.enlarge()` / 输入百分比 `view.setScale(ratio, cx, cy)`；订阅 `scale`）
- Create: `chrome/MouseAction.vue`（切 `localConfig.useLeftKeySelectionRightKeyDrag`，tooltip 文案 tip1/tip2）
- Create: `chrome/Fullscreen.vue`（`document.documentElement.requestFullscreen()`；「全屏查看」= 全屏 + `setMode('readonly')`，「全屏编辑」= 仅全屏；监听 `fullscreenchange` 退出时恢复模式）
- Create: `chrome/Demonstrate.vue`（`mindMap.demonstrate.enter()`；演示中底部控制条：上一步 / 下一步 / 退出，订阅 `demonstrate_jump` / `exit_demonstrate`）

### Task 25: 小地图
- Create: `chrome/Navigator.vue`（移植 `Navigator.vue`：订阅 `toggleMiniMap`；`mindMap.miniMap.calculationMiniMap(width, height)` 取 `{ getImgUrl, viewBoxStyle, miniMapBoxScale, miniMapBoxLeft, miniMapBoxTop }`；视口框拖拽 `onMovingViewBox`；点击定位 `onMiniMapClick`；订阅 `data_change / view_data_change / node_tree_render_end` 节流 500ms 重绘）

### Task 26: 滚动条
- Create: `chrome/ScrollbarBars.vue`（移植 `Scrollbar.vue`：`v-if="localConfig.isShowScrollbar"`；订阅 `scrollbar_change` 更新两条滚动条尺寸位置；拖拽 → `mindMap.scrollbar.onMousedown/onMousemove/onMouseup`；点击轨道 → `onClick`）

### Task 27: 搜索与替换
- Create: `popups/SearchBox.vue`（移植 `Search.vue`：订阅 `showSearch`；`mindMap.search.search(text, () => …)` 回车逐个定位；替换 `replace(text)` / 全部 `replaceAll(text)`；订阅 `search_info_change` 显示 `currentIndex / total`；Esc 或关闭 `mindMap.search.endSearch()`）

### Task 28: 富文本浮动工具栏
- Create: `popups/RichTextToolbar.vue`（移植 `RichTextToolbar.vue`：订阅 `rich_text_selection_change (hasRange, rect, formatInfo)` 定位与回显；按钮：加粗 / 斜体 / 下划线 / 删除线 / 字体 `n-select` / 字号 / 字体颜色 / 背景颜色 / 对齐 / 清除样式 → `mindMap.richText.formatText({...})` 与 `removeFormat()`）

### Task 29: 节点图标浮动栏与图片位置浮动栏
- Create: `popups/NodeIconToolbar.vue`（移植：`node_active` 单选且有图标时显示于节点上方；点击图标 → 打开 `nodeIconSidebar`；✕ → `node.setIcon([])`；订阅 `close_node_icon_toolbar`）
- Create: `popups/NodeImgPlacementToolbar.vue`（移植：节点有图片时显示上下左右四向按钮 → `node.setStyle('imgPlacement', dir)`；`mindMap.view` 变化时重新定位）

### Task 30: 备注悬浮与图片预览
- Create: `popups/NoteContentShow.vue`（订阅 `showNoteContent (content,left,top,node)` / `hideNoteContent`；`markdown-it` 渲染 `content`（项目已有 `renderMarkdown`）；`node_note_dblclick` → 打开备注对话框）
- Create: `popups/NodeImgPreview.vue`（订阅 `node_img_dblclick (node, e)`；`n-image` 预览 `node.getData('image')`）

### Task 31: 外框 / 标签 / 关联线样式面板
- Create: `popups/OuterFramePanel.vue`（移植 `NodeOuterFrame.vue`：订阅 `outer_frame_active (el, parentNode, range)`；边框样式 / 颜色 / 填充 / 圆角 / 文字与文字样式 → `mindMap.outerFrame.updateActiveOuterFrame({...})`；删除 → `removeActiveOuterFrame()`）
- Create: `popups/TagStylePanel.vue`（移植 `NodeTagStyle.vue`：订阅 `node_tag_click (node, index)`；文字 / 颜色 / 背景 / 圆角 / 字号 → `node.setTag(newList)`；删除此标签）
- Create: `popups/AssociativeLineStylePanel.vue`（移植 `AssociativeLineStyle.vue`：订阅 `associative_line_click (line, node, toNode)`；颜色 / 粗细 / 虚线 / 箭头方向 / 文字字体字号颜色 → `mindMap.associativeLine.setActiveLineStyle({...})`；删除 → `removeLine`）

### Task 32: 大纲全屏编辑与源码编辑
- Create: `dialogs/OutlineEditDialog.vue`（`v-if="ui.isOutlineEdit"` 铺满窗口；复用 `OutlineTree`；顶部：返回 / 打印（参考端 `printOutline` 用 iframe，移植到 `core/print.ts`））
- Create: `dialogs/SourceCodeDialog.vue`（`n-modal` 内 `codemirror`（项目已有 `@codemirror/*`，JSON 用 `@codemirror/lang-json`——若未装则 `pnpm add @codemirror/lang-json`）展示 `JSON.stringify(getData(true), null, 2)`；格式化 / 复制 / 完成（`JSON.parse` 后 `bus.emit('setData', data)`，失败 Toast「JSON格式有误」））

### Task 33: 节点链接与附件
- Create: `dialogs/NodeLinkDialog.vue`（移植参考端 `nodeLink` 逻辑（在 `Contextmenu.vue` 与 `Edit.vue` 里）：订阅 `showNodeLink (node)`；进入「选择目标节点」模式：监听一次 `node_click`，不能链接自己（tip2），确认后 `node.setHyperlink('#' + target.uid ... )`——参考端用 `mindMap.opt.customHyperlinkJump`；本项目在 `createMindMap` 增加 `customHyperlinkJump: (link, node) => link.startsWith('#') ? mindMap.execCommand('GO_TARGET_NODE', link.slice(1)) : api.system.openUrl(link)`；「是否添加反向链接」勾选则目标节点也 setHyperlink；删除节点链接 → `setHyperlink('','')`）
- Create: `dialogs/AttachmentPicker.ts`（订阅 `selectAttachment (nodes)`：`open({ multiple:false })` 选文件 → 对每个节点 `mindMap.execCommand('SET_NODE_ATTACHMENT', node, path, fileName)`；订阅 `node_attachmentClick (node)` → `api.system.openPath(node.getData('attachmentUrl'))`；`node_attachmentContextmenu` → 小菜单「删除附件」→ `SET_NODE_ATTACHMENT(node, '', '')`）

---

# P4 · 收尾

### Task 34: 拖拽导入、禅模式、清理
- Modify: `MindMapApp.vue`：`dragenter/dragleave/drop` 遮罩（`localConfig.enableDragImport && !ui.isDragOutlineTreeNode`），drop → `bus.emit('importFile', file)`；禅模式 `.zen` 下隐藏 Toolbar / SidebarTrigger / Count / NavigatorToolbar（右键菜单可退出）；`ui.isZenMode` 与 `localConfig.isZenMode` 同步
- 删除 `src/styles/window-fit.css` 中残留的 `.mindmap-*` 注释；`README.md` 思维导图章节更新功能清单

### Task 35: 全量验收
- 按 spec「范围 · 做」表逐项在实机操作，结果写入本文件末尾「验收记录」表（项目 / 结果 / 备注）
- `npx vue-tsc --noEmit`、`npx prettier --check`、`cargo test`、`vite build` 全绿
- `git commit -m "docs(mindmap): 思维导图全功能实机验收记录"`

## 验收记录

**验收日期**：2026-09-09　**分支**：feature/tauri-vue

### 自动化校验（全绿）

| 项 | 结果 |
|---|---|
| `npx vue-tsc --noEmit` | ✅ 0 错误 |
| `npx prettier --check "src/**/*.{vue,ts,css}"` | ✅ All matched files use Prettier code style |
| `npx vite build` | ✅ built（mindmap chunk 正常打包；仅有 chunk >500KB 的信息级提示，非错误） |
| `cargo test` | ✅ 63 passed / 0 failed |

### 功能实现对照（对齐 spec「范围 · 做」；实机 GUI 交互项待用户在 `tauri dev` 中验收）

| 分区 | 实现 | 备注 |
|---|---|---|
| 窗口壳 / 持久化 / 自动保存 | ✅ Task 1-7 | 全量数据格式 + 旧格式兼容 |
| 侧栏壳 / 触发条 / 表单件 | ✅ Task 8 | |
| 工具栏 / 节点按钮 | ✅ Task 9（+链接节点/附件入口 Task 33） | |
| 右键菜单 | ✅ Task 10 | |
| 快捷键侧栏 / 字数节点统计 | ✅ Task 11 | |
| 导入 / 导出 / 图片·超链接·备注·标签对话框 | ✅ Task 12-14 | |
| 结构 / 主题侧栏 | ✅ Task 15-16 | 含 31 套主题（新增棕褐） |
| 基础样式侧栏 | ✅ Task 17 | 内置背景图列表按 spec 不做 |
| 节点样式侧栏 | ✅ Task 18 | 选中态死页签按裁决砍除 |
| 设置（含水印）/ 图标 / 公式 / 备注侧栏 | ✅ Task 19/21-23 | |
| 大纲侧栏（自实现递归树）| ✅ Task 20 | |
| 导航栏 / 缩放 / 鼠标 / 全屏 / 演示 | ✅ Task 24 | 结构下拉按裁决不加（在结构侧栏） |
| 小地图 | ✅ Task 25 | |
| 滚动条 | ✅ Task 26 | |
| 搜索与替换 | ✅ Task 27 | |
| 富文本浮动工具栏 | ✅ Task 28 | |
| 图标浮动栏 / 图片位置浮动栏 | ✅ Task 29 | |
| 备注悬浮 / 图片预览 | ✅ Task 30 | |
| 外框 / 标签 / 关联线样式面板 | ✅ Task 31 | 关联线按参考端实际 API（无箭头方向项） |
| 大纲全屏编辑 / 源码编辑 | ✅ Task 32 | 源码编辑用 textarea，未引入需联网拉取的 @codemirror/lang-json |
| 节点链接 / 附件 | ✅ Task 33 | |
| 拖拽导入 / 禅模式 / 清理 | ✅ Task 34 | |

### 说明

- 本环境无显示器，实机 GUI 交互（节点拖拽、演示投屏、全屏等）留待用户在 `pnpm tauri:dev` 中逐项确认；代码路径均已实现并通过静态校验与构建。
- 未做项均为 spec 明确排除（AI / 协同 / 浏览器本地文件目录树 / 内置背景图服务端列表 / 贴纸 image.js）。
