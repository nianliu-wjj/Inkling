//! 索引快照：把扫描结果写到 `launcher/index.json`，启动时先加载即可搜索，后台再重扫替换。
//!
//! 快照是可再生缓存，不进业务数据库；损坏时忽略并重扫。原子写（.tmp + rename）。

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::model::Candidate;

/// 快照文件格式版本；结构变化时递增，旧版本直接丢弃重扫。
const SNAPSHOT_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct Snapshot {
    version: u32,
    /// 生成时间（Unix 秒），用于状态展示。
    created_at: i64,
    candidates: Vec<Candidate>,
}

/// 加载快照；文件缺失、损坏或版本不符返回 None。
pub fn load(file: &Path) -> Option<(Vec<Candidate>, i64)> {
    let text = std::fs::read_to_string(file).ok()?;
    let snapshot: Snapshot = match serde_json::from_str(&text) {
        Ok(s) => s,
        Err(error) => {
            eprintln!("[launcher] 快照损坏，忽略并重扫: {error}");
            return None;
        }
    };
    if snapshot.version != SNAPSHOT_VERSION {
        eprintln!(
            "[launcher] 快照版本 {} 与当前 {} 不符，忽略",
            snapshot.version, SNAPSHOT_VERSION
        );
        return None;
    }
    Some((snapshot.candidates, snapshot.created_at))
}

/// 原子保存快照。
pub fn save(file: &Path, candidates: &[Candidate], created_at: i64) -> Result<(), String> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("创建快照目录失败: {e}"))?;
    }
    let snapshot = Snapshot {
        version: SNAPSHOT_VERSION,
        created_at,
        candidates: candidates.to_vec(),
    };
    let text = serde_json::to_string(&snapshot).map_err(|e| format!("序列化快照失败: {e}"))?;
    let tmp = file.with_extension("json.tmp");
    std::fs::write(&tmp, text).map_err(|e| format!("写快照失败: {e}"))?;
    std::fs::rename(&tmp, file).map_err(|e| format!("提交快照失败: {e}"))
}

#[cfg(test)]
mod tests {
    use super::super::model::Kind;
    use super::*;

    #[test]
    fn roundtrip_and_corruption_handling() {
        let dir = std::env::temp_dir().join(format!("inkling-snapshot-{}", std::process::id()));
        let file = dir.join("index.json");
        let candidates = vec![Candidate {
            id: 0,
            kind: Kind::App,
            name: "微信".into(),
            path: "C:/WeChat.exe".into(),
            keywords: vec!["weixin".into()],
            bias: 0.0,
        }];
        save(&file, &candidates, 42).unwrap();
        let (loaded, created) = load(&file).unwrap();
        assert_eq!(created, 42);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "微信");
        assert_eq!(loaded[0].kind, Kind::App);

        std::fs::write(&file, "{ not json").unwrap();
        assert!(load(&file).is_none());
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(load(&file).is_none());
    }
}
