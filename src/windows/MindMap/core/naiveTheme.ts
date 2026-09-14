import { darkTheme, type GlobalTheme, type GlobalThemeOverrides } from 'naive-ui'

/**
 * naive-ui 主题映射到 Inkling 令牌（spec D3）。
 * 只在导图窗口用；颜色取 CSS 变量的运行时值，主题切换时由 MindMapApp 重新计算。
 */

/** 深色基底。浅色主题用 naive 自带的浅色基底，即 `theme` 传 `null`。 */
export const naiveDark = darkTheme

/**
 * `buildThemeOverrides()` 的结果。
 *
 * 基底与覆盖项一起返回，是因为两者都由「当前主题的明暗」决定：调用方（MindMapApp）若自己去读
 * `--scheme`，就会出现两处判断、两份可能不同步的结论。宁可让本模块判一次。
 */
export interface MindMapNaiveTheme {
  /** naive 基底：深色主题给 `naiveDark`，浅色主题给 `null`（naive 的默认浅色基底）。 */
  theme: GlobalTheme | null
  /** 令牌映射出的覆盖项。 */
  overrides: GlobalThemeOverrides
}

function cssVar(name: string, fallback: string): string {
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim()
  return value || fallback
}

/**
 * 当前主题的明暗。
 *
 * 依据是主题块里的 `--scheme`（tokens.css 的基础 `:root`、themes.css 的 30 个块、extensions.css 的
 * sepia 都定义了它，取值与 `color-scheme` 同源）。取不到时按深色处理：基础 `:root` 本就是深色，
 * 且原实现恒用深色基底，回退到深色不会比它更差。
 */
function currentScheme(): 'light' | 'dark' {
  return cssVar('--scheme', 'dark').trim() === 'light' ? 'light' : 'dark'
}

export function buildThemeOverrides(): MindMapNaiveTheme {
  const accent = cssVar('--accent', '#6c8cff')
  const text = cssVar('--text', '#e8e8f0')
  const textDim = cssVar('--text-dim', '#9a9ab0')
  const menuBg = cssVar('--menu-bg', 'rgba(28,30,44,0.96)')
  const overrides: GlobalThemeOverrides = {
    common: {
      primaryColor: accent,
      primaryColorHover: accent,
      primaryColorPressed: accent,
      primaryColorSuppl: accent,
      textColorBase: text,
      textColor1: text,
      textColor2: text,
      textColor3: textDim,
      popoverColor: menuBg,
      modalColor: menuBg,
      cardColor: menuBg,
      // 输入框底与描边原先是写死的白色半透明——那等于把「深色基底」硬编码进来，浅色主题下
      // 输入框会是浅灰底上的白蒙层。改吃主题的墨色通道 --wsa：浅色主题的 --wsa 是深墨
      // （浅灰底 + 浅灰边），深色主题是浅墨（与原来的白蒙层同效），透明度和原来一致。
      inputColor: 'rgba(var(--wsa), 0.06)',
      borderColor: 'rgba(var(--wsa), 0.14)',
      borderRadius: '8px',
      fontSize: '13px',
    },
  }
  // 浅色主题给 null：naive 的默认基底就是浅色。恒用 darkTheme 时，凡是没有被上面这些令牌
  // 覆盖到的角落（表格斑马纹、空态插图、滚动条……）在浅色主题下依然是暗的。
  return { theme: currentScheme() === 'light' ? null : naiveDark, overrides }
}
