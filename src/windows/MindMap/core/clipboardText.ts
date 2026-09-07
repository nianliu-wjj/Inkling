import { imgToDataUrl } from 'simple-mind-map/src/utils/index.js'
import { logger } from '@/service/logger'

/**
 * 粘贴文本的自定义解析（参考端 web/src/utils/handleClipboardText.js 逐行移植）。
 *
 * 识别「知犀」思维导图复制到剪贴板的两种数据格式，转换成库能直接插入的节点列表；
 * 其余文本返回空串，交给库按普通文本处理。
 */
interface ZhixiNode {
  data: {
    text?: string
    hyperlink?: string
    hyperlinkTitle?: string
    note?: string
    image?: string
    imageSize?: unknown
    type?: string
  }
  children?: ZhixiNode[]
}

interface ConvertedNode {
  data: Record<string, unknown>
  children: ConvertedNode[]
}

async function handleZhixi(input: unknown): Promise<{ simpleMindMap: true; data: ConvertedNode[] } | ''> {
  try {
    let data: unknown = input
    try {
      if (!Array.isArray(data)) {
        data = JSON.parse(String(data).replace('￿﻿', ''))
      }
    } catch (error) {
      logger.debug('mindmap', '知犀数据解析失败，按空列表处理', error)
    }
    const list: ZhixiNode[] = Array.isArray(data) ? (data as ZhixiNode[]) : []
    const result: ConvertedNode[] = []
    const waitLoadImageList: Promise<void>[] = []

    const walk = (nodes: ZhixiNode[], target: ConvertedNode[]): void => {
      nodes.forEach((item) => {
        const newRoot: ConvertedNode = {
          data: {
            text: item.data.text,
            hyperlink: item.data.hyperlink,
            hyperlinkTitle: item.data.hyperlinkTitle,
            note: item.data.note,
          },
          children: [],
        }
        target.push(newRoot)
        // 图片：远程地址转 dataURL，失败则忽略图片
        if (item.data.image) {
          const image = item.data.image
          waitLoadImageList.push(
            imgToDataUrl(image)
              .then((url) => {
                newRoot.data.image = url
                newRoot.data.imageSize = item.data.imageSize
              })
              .catch(() => undefined),
          )
        }
        if (item.children && item.children.length > 0) {
          const children: ZhixiNode[] = []
          item.children.forEach((child) => {
            // 概要节点转为本节点的 generalization
            if (child.data.type === 'generalize') {
              newRoot.data.generalization = [{ text: child.data.text }]
            } else {
              children.push(child)
            }
          })
          walk(children, newRoot.children)
        }
      })
    }
    walk(list, result)
    await Promise.all(waitLoadImageList)
    return { simpleMindMap: true, data: result }
  } catch {
    return ''
  }
}

export async function handleClipboardText(text: string): Promise<unknown> {
  // 知犀数据格式 1：带 __c_zx_v 标记的 JSON
  try {
    const parsed = JSON.parse(text) as { __c_zx_v?: unknown; children?: unknown }
    if (parsed.__c_zx_v !== undefined) {
      return handleZhixi(parsed.children)
    }
  } catch {
    // 不是 JSON，继续判断格式 2
  }
  // 知犀数据格式 2：带特殊 BOM 字符
  if (text.includes('￿﻿')) {
    return handleZhixi(text)
  }
  return ''
}
