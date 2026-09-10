/**
 * Node `node --test` 直接运行 src 下 TypeScript 单测的解析钩子：
 *   1. `@/xxx` 别名 → `src/xxx`（与 tsconfig paths 一致）；
 *   2. 无扩展名的相对导入按 .ts / .mts / .js / .mjs 依次探测（项目源码统一省略扩展名）。
 * 类型剥离由 Node 24 内建完成，无需编译步骤。只用于纯函数模块的测试，不加载 Vue 组件。
 */
import { existsSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

const SRC = pathToFileURL(path.resolve('src') + '/').href
const EXTENSIONS = ['.ts', '.mts', '.js', '.mjs']

export async function resolve(specifier, context, nextResolve) {
  let spec = specifier
  if (spec.startsWith('@/')) spec = new URL(spec.slice(2), SRC).href
  const relative = spec.startsWith('./') || spec.startsWith('../') || spec.startsWith('file:')
  if (relative && !path.extname(spec)) {
    const base = spec.startsWith('file:') ? spec : new URL(spec, context.parentURL).href
    for (const ext of EXTENSIONS) {
      if (existsSync(fileURLToPath(base + ext))) return nextResolve(base + ext, context)
    }
  }
  return nextResolve(spec, context)
}
