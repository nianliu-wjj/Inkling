/**
 * 玻璃质感档位。
 *
 * 与配色主题（`constants/themes.ts`）**正交**：同一套配色可以是轻薄玻璃也可以是厚玻璃。
 * 30 套配色 × 3 档质感 = 90 种组合，不必为「深色 + 极简玻璃」再复制一套 CSS 主题。
 *
 * 档位定义在 `styles/glass.css` 的 `:root[data-glass="..."]`；
 * `standard` 是 tokens.css 中 :root 的默认值，不设该属性——与主题的 `dark` 同一约定。
 */

/** 单个档位的展示信息。 */
export interface GlassOption {
  /** 档位标识，对应 CSS 的 data-glass 值；standard 表示不设该属性。 */
  key: string
  label: string
  /** 设置页下拉里的一句话说明。 */
  hint: string
}

export const glassLevels: readonly GlassOption[] = [
  { key: 'minimal', label: '极简', hint: '弱模糊、单层浅阴影，界面最轻' },
  { key: 'standard', label: '标准', hint: '默认质感' },
  { key: 'frosted', label: '厚玻璃', hint: '强模糊高饱和，质感最重' },
] as const

/** 默认档位标识。 */
export const DEFAULT_GLASS_LEVEL = 'standard'
