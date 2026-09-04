/**
 * 全局样式入口。
 *
 * 加载顺序不可调换：
 *   tokens（默认 dark 令牌与基元）
 *   → base（交互修正、拖拽区、毛玻璃降级）
 *   → components（原型组件样式）
 *   → window-fit（原型浮层定位 → 真实窗口的适配，必须在 components 之后）
 *   → motion（动效层，需覆盖 components 里硬编码的 transition）
 *   → themes（29 套 [data-theme] 覆盖，必须压过默认值）
 *   → glass（3 档 [data-glass] 质感覆盖，必须在 themes 之后才能压过主题的阴影）
 *
 * 所有窗口入口（main / panel / pinned / reminder / hotzone）统一引入本文件。
 */
import './tokens.css'
import './base.css'
import './components.css'
import './window-fit.css'
import './motion.css'
import './themes.css'
import './glass.css'
