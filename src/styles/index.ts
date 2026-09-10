/**
 * 全局样式入口。
 *
 * 加载顺序不可调换：
 *   tokens（生成：尺度令牌 + 深色基础主题令牌 + 基元）
 *   → base（生成：全局质量层）
 *   → components（生成：原型组件样式）
 *   → themes（生成：30 套 [data-theme] 覆盖，必须压过默认值）
 *   → extensions（手写：项目扩展令牌 / 交互修正 / 自建组件 / 过渡桥接 / 棕褐主题，必须在 themes 之后）
 *   → window-fit（手写：原型浮层定位 → 真实窗口的适配）
 *   → motion（手写：动效层，覆盖组件里硬编码的 transition）
 *   → glass（手写：3 档 [data-glass] 质感覆盖，必须在 themes 之后才能压过主题的阴影）
 *
 * 生成层由 scripts/sync-prototype-styles.mjs 从 docs/styles.css 生成，勿手改。
 * 所有窗口入口统一引入本文件。
 */
import './tokens.css'
import './base.css'
import './components.css'
import './themes.css'
import './extensions.css'
import './window-fit.css'
import './motion.css'
import './glass.css'
