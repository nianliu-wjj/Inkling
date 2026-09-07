import catalogOrganization from '@/assets/mindmap/structures/catalogOrganization.jpg'
import fishbone from '@/assets/mindmap/structures/fishbone.jpg'
import fishbone2 from '@/assets/mindmap/structures/fishbone2.jpg'
import logicalStructure from '@/assets/mindmap/structures/logicalStructure.jpg'
import logicalStructureLeft from '@/assets/mindmap/structures/logicalStructureLeft.jpg'
import mindMap from '@/assets/mindmap/structures/mindMap.jpg'
import organizationStructure from '@/assets/mindmap/structures/organizationStructure.jpg'
import rightFishbone from '@/assets/mindmap/structures/rightFishbone.jpg'
import rightFishbone2 from '@/assets/mindmap/structures/rightFishbone2.jpg'
import timeline from '@/assets/mindmap/structures/timeline.jpg'
import timeline2 from '@/assets/mindmap/structures/timeline2.jpg'
import verticalTimeline from '@/assets/mindmap/structures/verticalTimeline.jpg'
import verticalTimeline2 from '@/assets/mindmap/structures/verticalTimeline2.jpg'
import verticalTimeline3 from '@/assets/mindmap/structures/verticalTimeline3.jpg'

/**
 * 结构（布局）常量：分组列表、缩略图与中文名。
 * 取值必须与库 `constants/constant.js` 的 layoutValueList 一致。
 */
export const layoutImgMap: Readonly<Record<string, string>> = {
  logicalStructure,
  logicalStructureLeft,
  mindMap,
  organizationStructure,
  catalogOrganization,
  timeline,
  timeline2,
  fishbone,
  fishbone2,
  rightFishbone,
  rightFishbone2,
  verticalTimeline,
  verticalTimeline2,
  verticalTimeline3,
}

/** 结构中文名（导航工具栏下拉与结构侧栏共用）。 */
export const layoutNameMap: Readonly<Record<string, string>> = {
  logicalStructure: '逻辑结构图',
  logicalStructureLeft: '向左逻辑结构图',
  mindMap: '思维导图',
  organizationStructure: '组织结构图',
  catalogOrganization: '目录组织图',
  timeline: '时间轴',
  timeline2: '时间轴2',
  fishbone: '鱼骨图',
  fishbone2: '鱼骨图2',
  rightFishbone: '向右鱼骨图',
  rightFishbone2: '向右鱼骨图2',
  verticalTimeline: '竖向时间轴',
  verticalTimeline2: '竖向时间轴2',
  verticalTimeline3: '竖向时间轴3',
}

/** 结构侧栏的分组（参考端 layoutGroupList，补齐库支持的全部取值）。 */
export const layoutGroupList: readonly { name: string; list: string[] }[] = [
  { name: '逻辑结构图', list: ['logicalStructure', 'logicalStructureLeft'] },
  { name: '思维导图', list: ['mindMap'] },
  { name: '组织结构图', list: ['organizationStructure'] },
  { name: '目录组织图', list: ['catalogOrganization'] },
  { name: '时间轴', list: ['timeline', 'timeline2', 'verticalTimeline', 'verticalTimeline2', 'verticalTimeline3'] },
  { name: '鱼骨图', list: ['fishbone', 'fishbone2', 'rightFishbone', 'rightFishbone2'] },
]

/** 支持某种连线类型的结构 */
export const supportLineStyleLayoutsMap: Readonly<Record<string, string[]>> = {
  curve: ['logicalStructure', 'logicalStructureLeft', 'mindMap', 'verticalTimeline', 'organizationStructure'],
  direct: ['logicalStructure', 'logicalStructureLeft', 'mindMap', 'organizationStructure', 'verticalTimeline'],
}

/** 直线模式支持设置圆角的结构 */
export const supportLineRadiusLayouts: readonly string[] = [
  'logicalStructure',
  'logicalStructureLeft',
  'mindMap',
  'verticalTimeline',
]

/** 支持只显示底边直线风格的结构 */
export const supportNodeUseLineStyleLayouts: readonly string[] = [
  'logicalStructure',
  'logicalStructureLeft',
  'mindMap',
  'catalogOrganization',
  'organizationStructure',
]

/** 支持曲线模式下根节点样式和其他节点样式保持一致的结构 */
export const supportRootLineKeepSameInCurveLayouts: readonly string[] = [
  'logicalStructure',
  'logicalStructureLeft',
  'mindMap',
  'organizationStructure',
]
