//! 导出服务：单条/批量导出为 Markdown / TXT / HTML / PDF / PNG。
//!
//! - md/txt/html：直接生成文本文件；
//! - pdf：genpdfi + 系统中文字体纯文本排版；
//! - png：ab_glyph 渲染简版文本位图。
//!
//! 全部导出到用户选择的目录（由前端 dialog 插件提供路径），默认回退桌面。

use std::fs;
use std::path::PathBuf;

use crate::domain::models::{ClipboardEntry, Note, Todo};

/// 导出条目（笔记 / 待办 / 剪贴板）。
pub enum ExportItem {
    Note(Note),
    Todo(Todo),
    Clip(ClipboardEntry),
}

impl ExportItem {
    fn markdown(&self) -> String {
        match self {
            ExportItem::Note(note) => {
                let mut md = String::new();
                md.push_str(&format!("<!-- Inkling 笔记 {} -->\n\n", note.updated_at()));
                md.push_str(note.content());
                if !note.tags().is_empty() {
                    md.push_str("\n\n---\n标签：");
                    md.push_str(
                        &note
                            .tags()
                            .iter()
                            .map(|t| format!("#{t}"))
                            .collect::<Vec<_>>()
                            .join(" "),
                    );
                }
                md
            }
            ExportItem::Todo(todo) => {
                let check = if todo.status == "done" { "x" } else { " " };
                format!(
                    "- [{check}] {}（完成时间 {}，优先级 {}）\n  标签：{}\n  备注：{}",
                    todo.content,
                    todo.due_at,
                    todo.priority,
                    todo.tags.join(" "),
                    todo.remark
                )
            }
            ExportItem::Clip(clip) => format!(
                "> {}（类型 {}，捕获于 {}）",
                clip.content(),
                clip.content_type(),
                clip.copied_at()
            ),
        }
    }

    fn plain(&self) -> String {
        match self {
            ExportItem::Note(note) => note.content().clone(),
            ExportItem::Todo(todo) => {
                format!(
                    "[{}] {}\n完成时间：{}\n备注：{}",
                    if todo.status == "done" {
                        "已完成"
                    } else {
                        "未完成"
                    },
                    todo.content,
                    todo.due_at,
                    todo.remark
                )
            }
            ExportItem::Clip(clip) => clip.content().clone(),
        }
    }
}

pub enum ExportFormat {
    Markdown,
    Txt,
    Html,
    Pdf,
    Png,
}

impl ExportFormat {
    pub fn from(value: &str) -> Option<Self> {
        match value {
            "md" => Some(Self::Markdown),
            "txt" => Some(Self::Txt),
            "html" => Some(Self::Html),
            "pdf" => Some(Self::Pdf),
            "png" => Some(Self::Png),
            _ => None,
        }
    }

    fn ext(&self) -> &'static str {
        match self {
            Self::Markdown => "md",
            Self::Txt => "txt",
            Self::Html => "html",
            Self::Pdf => "pdf",
            Self::Png => "png",
        }
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn default_export_dir() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .map(|home| home.join("Desktop"))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

/// 导出多条内容为单个文件；返回写入的绝对路径。
pub fn export_items(
    items: &[ExportItem],
    format: ExportFormat,
    output_dir: Option<PathBuf>,
    base_name: &str,
) -> Result<String, String> {
    let dir = output_dir.unwrap_or_else(default_export_dir);
    fs::create_dir_all(&dir).map_err(|e| format!("创建导出目录失败: {e}"))?;
    let filename = sanitize_filename(base_name);
    let path = dir.join(format!("{filename}.{}", format.ext()));
    let temp = path.with_extension(format!("{}.tmp", format.ext()));

    let bytes = render(items, &format, base_name)?;
    fs::write(&temp, &bytes).map_err(|e| format!("写入导出文件失败: {e}"))?;
    fs::rename(&temp, &path).map_err(|e| format!("提交导出文件失败: {e}"))?;
    Ok(path.to_string_lossy().to_string())
}

fn render(items: &[ExportItem], format: &ExportFormat, title: &str) -> Result<Vec<u8>, String> {
    match format {
        ExportFormat::Markdown => Ok(items
            .iter()
            .map(|item| item.markdown())
            .collect::<Vec<_>>()
            .join("\n\n---\n\n")
            .into_bytes()),
        ExportFormat::Txt => Ok(items
            .iter()
            .map(|item| item.plain())
            .collect::<Vec<_>>()
            .join("\n\n")
            .into_bytes()),
        ExportFormat::Html => Ok(render_html(items, title).into_bytes()),
        ExportFormat::Pdf => render_pdf(items, title),
        ExportFormat::Png => render_png(items, title),
    }
}

fn render_html(items: &[ExportItem], title: &str) -> String {
    use pulldown_cmark::{html, Options, Parser};
    let mut body = String::new();
    for item in items {
        let markdown = item.markdown();
        let parser = Parser::new_ext(&markdown, Options::all());
        html::push_html(&mut body, parser);
    }
    format!(
        "<!DOCTYPE html>\n<html lang=\"zh-CN\">\n<head>\n<meta charset=\"UTF-8\">\n<title>{title} · Inkling 导出</title>\n\
         <style>body{{font-family:'Segoe UI','Microsoft YaHei',sans-serif;max-width:760px;margin:40px auto;padding:0 24px;line-height:1.7;color:#222}}\
         code{{background:#f4f4f6;padding:2px 6px;border-radius:4px}}blockquote{{border-left:3px solid #bbb;margin:0;padding:2px 16px;color:#666}}</style>\n\
         </head>\n<body>\n<h1>{title}</h1>\n{body}\n<footer style=\"margin-top:48px;color:#999;font-size:12px\">由 Inkling（念头捕手）导出</footer>\n</body>\n</html>",
        title = title,
        body = body
    )
}

/// Windows 系统字体目录下常见的中文字体。
const CJK_FONT_CANDIDATES: [&str; 6] = [
    "C:/Windows/Fonts/msyh.ttc",
    "C:/Windows/Fonts/msyh.ttf",
    "C:/Windows/Fonts/simhei.ttf",
    "C:/Windows/Fonts/simsun.ttc",
    "/System/Library/Fonts/PingFang.ttc",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
];

fn find_cjk_font() -> Option<PathBuf> {
    CJK_FONT_CANDIDATES
        .iter()
        .map(PathBuf::from)
        .find(|p| p.exists())
}

fn render_pdf(items: &[ExportItem], title: &str) -> Result<Vec<u8>, String> {
    use genpdfi::Element;
    let font_path = find_cjk_font().ok_or("未找到可用的中文字体，无法导出 PDF")?;
    let font_family = genpdfi::fonts::from_files(
        font_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new(".")),
        font_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("msyh"),
        None,
    )
    .map_err(|e| format!("加载字体失败: {e}"))?;
    let mut doc = genpdfi::Document::new(font_family);
    doc.set_title(title);
    doc.set_font_size(11);
    doc.push(
        genpdfi::elements::Paragraph::new(title)
            .styled(genpdfi::style::Style::new().bold().with_font_size(16)),
    );
    doc.push(genpdfi::elements::Break::new(0.5));
    for item in items {
        let text = item.plain();
        for line in text.lines() {
            doc.push(genpdfi::elements::Paragraph::new(line));
        }
        doc.push(genpdfi::elements::Break::new(0.3));
    }
    let mut buffer = Vec::new();
    doc.render(&mut buffer)
        .map_err(|e| format!("生成 PDF 失败: {e}"))?;
    Ok(buffer)
}

fn render_png(items: &[ExportItem], title: &str) -> Result<Vec<u8>, String> {
    crate::services::export::png::render_text_png(items, title)
}

pub mod png {
    //! ab_glyph 文本渲染：简版白色画布 + 黑色文本位图导出。
    use super::find_cjk_font;
    use ab_glyph::{Font, FontArc, Glyph, PxScale, ScaleFont};

    const WIDTH: f32 = 900.0;
    const LINE_HEIGHT: f32 = 26.0;
    const PADDING: f32 = 24.0;
    const MAX_LINES: usize = 40;

    pub fn render_text_png(items: &[super::ExportItem], title: &str) -> Result<Vec<u8>, String> {
        let font_path = find_cjk_font().ok_or("未找到可用的中文字体，无法导出 PNG")?;
        let font = FontArc::try_from_vec(
            std::fs::read(&font_path).map_err(|e| format!("读取字体失败: {e}"))?,
        )
        .map_err(|e| format!("解析字体失败: {e}"))?;
        let scale = PxScale { x: 18.0, y: 18.0 };
        let scaled = font.as_scaled(scale);

        let mut lines: Vec<String> = vec![title.to_string(), String::new()];
        for item in items {
            let text = item.plain();
            for line in text.lines().take(MAX_LINES) {
                lines.push(wrap_line(line, &font, scale, WIDTH - PADDING * 2.0));
            }
            lines.push(String::new());
        }
        lines.truncate(MAX_LINES + 2);
        let height = (PADDING * 2.0 + LINE_HEIGHT * lines.len() as f32).ceil() as u32;

        let width = WIDTH as u32;
        let mut buffer = vec![255u8; (width * height * 3) as usize]; // 白底 RGB
        let mut baseline = PADDING + scaled.ascent();
        for line in &lines {
            let mut caret = PADDING;
            for ch in line.chars() {
                let glyph_id = font.glyph_id(ch);
                let glyph = Glyph {
                    id: glyph_id,
                    scale,
                    position: ab_glyph::Point {
                        x: caret,
                        y: baseline,
                    },
                };
                if let Some(outlined) = font.outline_glyph(glyph) {
                    let bounds = outlined.px_bounds();
                    outlined.draw(|x, y, coverage| {
                        let px = bounds.min.x as u32 + x;
                        let py = bounds.min.y as u32 + y;
                        if px >= width || py >= height {
                            return;
                        }
                        let alpha = 1.0 - coverage;
                        let idx = ((py * width + px) * 3) as usize;
                        for channel in buffer.iter_mut().skip(idx).take(3) {
                            *channel = (*channel as f32 * alpha).clamp(0.0, 255.0) as u8;
                        }
                    });
                }
                caret += scaled.h_advance(glyph_id);
                if caret > WIDTH - PADDING {
                    break;
                }
            }
            baseline += LINE_HEIGHT;
        }

        let mut out = Vec::new();
        let header =
            image::RgbaImage::from_raw(width, height, vec![255; (width * height * 4) as usize])
                .ok_or("画布分配失败")?;
        // RGB → PNG（走 image 库编码）。
        let rgb = image::RgbImage::from_raw(width, height, buffer).ok_or("画布分配失败")?;
        image::DynamicImage::ImageRgb8(rgb)
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .map_err(|e| format!("编码 PNG 失败: {e}"))?;
        let _ = header;
        Ok(out)
    }

    fn wrap_line(line: &str, font: &FontArc, scale: PxScale, max_width: f32) -> String {
        if line.is_empty() {
            return String::new();
        }
        let scaled = font.as_scaled(scale);
        let mut result = String::new();
        let mut width = 0.0;
        for ch in line.chars() {
            let advance = scaled.h_advance(font.glyph_id(ch));
            if width + advance > max_width {
                result.push('…');
                break;
            }
            result.push(ch);
            width += advance;
        }
        result
    }
}
