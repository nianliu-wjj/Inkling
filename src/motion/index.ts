/**
 * 动效模块统一出口。调用方只用这里导出的预设与指令，不直接依赖 animejs。
 */
export { readMotionTokens, type MotionTokens } from './tokens'
export { enter, exit, staggerIn, pop, popIn, crossfade, type Axis, type SlideOptions } from './presets'
export { vStaggerList } from './directive'
