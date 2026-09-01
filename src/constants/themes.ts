/**
 * 主题清单。
 *
 * 30 套主题：默认 `dark` 定义在 styles/tokens.css 的 :root，
 * 其余 29 套定义在 styles/themes.css 的 :root[data-theme="..."]。
 * 色点用于偏好设置页的主题下拉预览（样式 .theme-dots）。
 *
 * 数据来源：doc/app.js:1276-1306 的 THEMES 常量，保持逐字一致。
 */

/** 单套主题的展示信息。 */
export interface ThemeOption {
  /** 主题标识，对应 CSS 的 data-theme 值；dark 表示不设该属性。 */
  key: string
  /** 中文展示名。 */
  label: string
  /** 四枚预览色点：背景 / 主强调 / 次强调 / 第三强调。 */
  dots: readonly [string, string, string, string]
}

export const themes: readonly ThemeOption[] = [
  { key: 'dark', label: '深色', dots: ['#1e2232', '#6c8cff', '#ffd76e', '#7ee0a8'] },
  { key: 'light', label: '浅色', dots: ['#f2f5fc', '#4c68e0', '#b8860b', '#12805c'] },
  { key: 'cupcake', label: '纸杯蛋糕', dots: ['#fdf0f4', '#e56ba5', '#8fd3c7', '#f5c26b'] },
  { key: 'bumblebee', label: '大黄蜂', dots: ['#f6f3ea', '#a3860b', '#2b2b2b', '#8a8a8a'] },
  { key: 'emerald', label: '翡翠绿', dots: ['#e9f5ee', '#0f8f5f', '#2f7d6d', '#c2654a'] },
  { key: 'business', label: '商务蓝', dots: ['#e8edf6', '#2752c4', '#5b7bd5', '#94a3c4'] },
  { key: 'neon', label: '霓虹未来', dots: ['#160b2e', '#22d3ee', '#e879f9', '#a3e635'] },
  { key: 'retro', label: '复古', dots: ['#f0e4cc', '#b4713a', '#7a5c2e', '#c9a86a'] },
  { key: 'romance', label: '浪漫', dots: ['#fbeaf1', '#d2568f', '#9f7aea', '#f4a7c3'] },
  { key: 'halloween', label: '万圣节', dots: ['#1a1220', '#ff7a1a', '#8a2be2', '#5c4033'] },
  { key: 'fantasy', label: '奇幻', dots: ['#1c1030', '#c084fc', '#fcd34d', '#7dd3fc'] },
  { key: 'oled', label: '极黑', dots: ['#050505', '#4d8dff', '#f5c518', '#66bb6a'] },
  { key: 'luxury', label: '奢华', dots: ['#14100a', '#d4af37', '#e8c96a', '#a8c686'] },
  { key: 'dracula', label: '德古拉', dots: ['#282a36', '#bd93f9', '#50fa7b', '#ff79c6'] },
  { key: 'print', label: '印刷色', dots: ['#f5f5f1', '#1f1f24', '#0e7490', '#c0392b'] },
  { key: 'autumn', label: '秋日', dots: ['#f7ecd9', '#c6612c', '#7d8a2c', '#b0527e'] },
  { key: 'businessgray', label: '商务灰', dots: ['#eef0f2', '#495057', '#4a7c59', '#a35376'] },
  { key: 'psychedelic', label: '迷幻', dots: ['#12002e', '#ff3ec8', '#3ee8ff', '#ffe14d'] },
  { key: 'lemon', label: '柠檬', dots: ['#fbf8d8', '#9b7900', '#5c8a2c', '#b3400c'] },
  { key: 'night', label: '夜色', dots: ['#0b1026', '#5c7cfa', '#fbbf24', '#7dd3fc'] },
  { key: 'coffee', label: '咖啡', dots: ['#1b1210', '#c08552', '#ddb271', '#9caf88'] },
  { key: 'winter', label: '冬日', dots: ['#eef4fa', '#4a7fb5', '#4d8a6a', '#c05b6a'] },
  { key: 'abyss', label: '深渊', dots: ['#020c14', '#0e9db8', '#34c98e', '#e0b84d'] },
  { key: 'aqua', label: '水色', dots: ['#e4f6f8', '#0891b2', '#2c8a6b', '#3a6ea5'] },
  { key: 'latte', label: '焦糖拿铁', dots: ['#f2e7d8', '#a0673c', '#6b8a4a', '#b06a8a'] },
  { key: 'dim', label: '暗色', dots: ['#17181c', '#7c8cf8', '#d9b44a', '#6fbf8f'] },
  { key: 'aurora', label: '北极光', dots: ['#06131a', '#34d399', '#67e8f9', '#fbbf24'] },
  { key: 'pastel', label: '粉彩', dots: ['#fdf0f7', '#9d7bd8', '#6bbf95', '#d67ba0'] },
  { key: 'sunset', label: '日落', dots: ['#1f1030', '#fb923c', '#fde047', '#f472b6'] },
  { key: 'wireframe', label: '线框', dots: ['#f8f8f6', '#52525b', '#4a7c59', '#c04440'] },
] as const

/** 默认主题标识。 */
export const DEFAULT_THEME = 'dark'
