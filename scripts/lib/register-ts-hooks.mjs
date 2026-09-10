/** `node --import` 入口：注册 TS 解析钩子（见 ts-resolve-hooks.mjs）。 */
import { register } from 'node:module'

register('./ts-resolve-hooks.mjs', import.meta.url)
