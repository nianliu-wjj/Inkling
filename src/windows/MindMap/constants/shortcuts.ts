/**
 * 快捷键表（参考端 web/src/config/zh.js 的 shortcutKeyList）。
 * 只用于「快捷键」侧栏展示；实际绑定由库的 KeyboardNavigation 与 keyCommand 完成。
 */
export interface ShortcutItem {
  icon: string
  name: string
  value: string
}

export interface ShortcutGroup {
  type: string
  list: ShortcutItem[]
}

const isMac = navigator.platform.toUpperCase().includes('MAC')
const ctrl = isMac ? '⌘' : 'Ctrl'
const enter = isMac ? 'Return' : 'Enter'
const macFn = isMac ? 'fn + ' : ''

export const shortcutKeyList: readonly ShortcutGroup[] = [
  {
    type: '节点操作',
    list: [
      { icon: 'icontianjiazijiedian', name: '插入下级节点', value: 'Tab | Insert' },
      { icon: 'iconjiedian', name: '插入同级节点', value: enter },
      { icon: 'icondodeparent', name: '插入父节点', value: 'Shift + Tab' },
      { icon: 'iconshangyi', name: '上移节点', value: `${ctrl} + ↑` },
      { icon: 'iconxiayi', name: '下移节点', value: `${ctrl} + ↓` },
      { icon: 'icongaikuozonglan', name: '插入概要', value: `${ctrl} + G` },
      { icon: 'iconzhankai', name: '展开/收起节点', value: '/' },
      { icon: 'iconshanchu', name: '删除节点', value: 'Delete | Backspace' },
      { icon: 'iconshanchu', name: '仅删除当前节点', value: 'Shift + Backspace' },
      { icon: 'iconfuzhi', name: '复制节点', value: `${ctrl} + C` },
      { icon: 'iconjianqie', name: '剪切节点', value: `${ctrl} + X` },
      { icon: 'iconniantie', name: '粘贴节点', value: `${ctrl} + V` },
      { icon: 'iconbianji', name: '编辑节点', value: `${macFn}F2` },
      { icon: 'iconhuanhang', name: '文本换行', value: `Shift + ${enter}` },
      { icon: 'iconhoutui-shi', name: '回退', value: `${ctrl} + Z` },
      { icon: 'iconqianjin1', name: '前进', value: `${ctrl} + Y` },
      { icon: 'iconquanxuan', name: '全选', value: `${ctrl} + A` },
      { icon: 'iconquanxuan', name: '多选', value: `右键 / ${ctrl} + 左键` },
      { icon: 'iconzhengli', name: '一键整理布局', value: `${ctrl} + L` },
      { icon: 'iconsousuo', name: '搜索和替换', value: `${ctrl} + F` },
    ],
  },
  {
    type: '画布操作',
    list: [
      { icon: 'iconfangda', name: '放大', value: `${ctrl} + +` },
      { icon: 'iconsuoxiao', name: '缩小', value: `${ctrl} + -` },
      { icon: 'iconfangda', name: '放大/缩小', value: `${ctrl} + 鼠标滚动` },
      { icon: 'icondingwei', name: '回到根节点', value: `${ctrl} + ${enter}` },
      { icon: 'iconquanping1', name: '适应画布', value: `${ctrl} + i` },
    ],
  },
  {
    type: '大纲操作',
    list: [
      { icon: 'iconhuanhang', name: '文本换行', value: `Shift + ${enter}` },
      { icon: 'iconshanchu', name: '删除节点', value: 'Delete' },
      { icon: 'icontianjiazijiedian', name: '插入下级节点', value: 'Tab' },
      { icon: 'iconjiedian', name: '插入同级节点', value: enter },
      { icon: 'icondodeparent', name: '上移一个层级', value: 'Shift + Tab' },
    ],
  },
]
