import { darkTheme, type GlobalThemeOverrides } from 'naive-ui'

/**
 * naive-ui 主题映射到 Inkling 令牌（spec D3）。
 * 只在导图窗口用；颜色取 CSS 变量的运行时值，主题切换时由 MindMapApp 重新计算。
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
      primaryColorSuppl: accent,
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
