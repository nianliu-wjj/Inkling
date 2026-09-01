/**
 * 全局样式入口。
 *
 * 加载顺序不可调换：
 *   tokens（默认 dark 令牌与基元）
 *   → base（交互修正、拖拽区、毛玻璃降级）
 *   → components（原型组件样式）
 *   → themes（29 套 [data-theme] 覆盖，必须最后加载才能压过默认值）
 *
 * 所有窗口入口（main / panel / pinned / reminder / hotzone）统一引入本文件。
 */
import './tokens.css'
import './base.css'
import './components.css'
import './themes.css'
