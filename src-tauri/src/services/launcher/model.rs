//! 启动器索引的数据模型（scan / search / snapshot 共用）。

use serde::{Deserialize, Serialize};

/// 候选类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// 传统程序（.exe / .lnk / .url）
    App,
    /// 商店应用，path 为 `shell:AppsFolder\<AppID>`
    Uwp,
    File,
    Folder,
    /// 内置命令，path 为命令 id（如 `cmd:settings`）
    Command,
}

/// 一条可搜索、可启动的候选。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    /// 索引内序号，同一代际内稳定；重扫后重新编号。
    pub id: u32,
    pub kind: Kind,
    /// 显示名（.lnk 去扩展名）。
    pub name: String,
    /// 启动 / 定位用的路径或标识。
    pub path: String,
    /// 预生成关键字（小写、去重）。
    pub keywords: Vec<String>,
    /// 稳定偏差（预留，默认 0）。
    #[serde(default)]
    pub bias: f64,
}

/// 搜索命中项（返给前端）。
#[derive(Debug, Clone, Serialize)]
pub struct Hit {
    pub id: u32,
    pub kind: Kind,
    pub name: String,
    pub path: String,
    pub score: f64,
    /// 命中的关键字（用于前端提示「为何匹配」）。
    pub matched_keyword: String,
}
