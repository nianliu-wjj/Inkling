export interface ThemeOption {
  key: string
  label: string
}

export const themes: ThemeOption[] = [
  ['dark', '深色'],
  ['light', '浅色'],
  ['cupcake', '纸杯蛋糕'],
  ['bumblebee', '大黄蜂'],
  ['emerald', '翡翠绿'],
  ['business', '商务蓝'],
  ['neon', '霓虹未来'],
  ['retro', '复古'],
  ['romance', '浪漫'],
  ['halloween', '万圣节'],
  ['fantasy', '奇幻'],
  ['oled', '极黑'],
  ['luxury', '奢华'],
  ['dracula', '德古拉'],
  ['print', '印刷色'],
  ['autumn', '秋日'],
  ['businessgray', '商务灰'],
  ['psychedelic', '迷幻'],
  ['lemon', '柠檬'],
  ['night', '夜色'],
  ['coffee', '咖啡'],
  ['winter', '冬日'],
  ['abyss', '深渊'],
  ['aqua', '水色'],
  ['latte', '焦糖拿铁'],
  ['dim', '暗色'],
  ['aurora', '北极光'],
  ['pastel', '粉彩'],
  ['sunset', '日落'],
  ['wireframe', '线框'],
].map(([key, label]) => ({ key, label }))
