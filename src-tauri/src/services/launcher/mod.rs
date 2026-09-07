//! 启动器搜索（spec: docs/superpowers/specs/2026-09-08-launcher-search-design.md）。
//!
//! 模块划分：
//! - `keyword`：关键字生成管道（纯函数）；
//! - `score`：标准检索引擎（纯函数）；
//! - `history`：启动历史与查询亲和度加权（纯函数 + JSON 落盘）；
//! - `snapshot`：索引快照读写；
//! - `scan`：数据源扫描（程序 / UWP / 文件根目录 / 内置命令）；
//! - `search`：并行打分 + 加权 + Top-K + L1 缓存；
//! - `launch`：打开 / 管理员 / 打开所在文件夹 / 复制路径。
//!
//! dead_code 放行是暂时的：B3 接线搜索服务后移除。

#[allow(dead_code)]
pub mod keyword;
#[allow(dead_code)]
pub mod score;
