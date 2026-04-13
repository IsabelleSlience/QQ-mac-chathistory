use anyhow::{Context, Result};
use chrono::{Local, TimeZone};
use ntdb_unwrap::db;
use ntdb_unwrap::db::model::Message;
use ntdb_unwrap::ntqq::DBDecryptInfo;
use rusqlite::{Connection, OpenFlags};
use serde_json::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_QQ_CONTAINER_ROOT: &str =
    "Library/Containers/com.tencent.qq/Data/Library/Application Support/QQ";

pub fn resolve_nt_db_root(user_provided: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = user_provided {
        return Ok(path.to_path_buf());
    }

    let home = env::var("HOME").context("HOME is not set")?;
    let base = Path::new(&home).join(DEFAULT_QQ_CONTAINER_ROOT);
    let entries = fs::read_dir(&base)
        .with_context(|| format!("read QQ container root {}", base.display()))?;

    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in entries {
        let entry = entry?;
        let nt_db = entry.path().join("nt_db");
        let nt_msg = nt_db.join("nt_msg.db");
        if !nt_msg.exists() {
            continue;
        }
        let modified = nt_msg
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        match &best {
            Some((current, _)) if modified <= *current => {}
            _ => best = Some((modified, nt_db)),
        }
    }

    best.map(|(_, path)| path)
        .context("could not auto-detect nt_db root; pass --db-root explicitly")
}

pub fn open_encrypted_db(path: &Path, key: &str) -> Result<Connection> {
    db::register_offset_vfs()
        .map_err(|code| anyhow::anyhow!("register offset vfs failed: {code}"))?;
    let conn = Connection::open_with_flags_and_vfs(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY,
        db::OFFSET_VFS_NAME,
    )
    .with_context(|| format!("open encrypted db {}", path.display()))?;
    db::try_decrypt_db(
        &conn,
        DBDecryptInfo {
            key: key.to_string(),
            ..Default::default()
        },
    )
    .with_context(|| format!("decrypt db {}", path.display()))?;
    Ok(conn)
}

pub fn summarize_message_from_row(row: &rusqlite::Row, column: &str, msg_type: i64, sub_msg_type: i64) -> rusqlite::Result<String> {
    let message_blob: Option<Vec<u8>> = row.get(column)?;
    let parsed_message: Option<Message> = row.get(column)?;
    let message_json = parsed_message
        .as_ref()
        .and_then(|m| serde_json::to_value(m).ok());
    let mut summary = message_json
        .as_ref()
        .map(summarize_message_json)
        .filter(|s| !s.is_empty())
        .unwrap_or_default();
    if summary.is_empty() {
        summary = fallback_summary(msg_type, sub_msg_type, message_blob.as_deref());
    }
    Ok(summary)
}

pub fn trim_opt(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

pub fn sanitize_filename(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_ascii_control() {
            continue;
        }
        match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => out.push('_'),
            _ => out.push(ch),
        }
    }
    out.trim().trim_matches('.').chars().take(120).collect()
}

pub fn ts_to_local_string(ts: i64) -> String {
    Local
        .timestamp_opt(ts, 0)
        .single()
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
        .unwrap_or_default()
}

fn fallback_summary(msg_type: i64, sub_msg_type: i64, blob: Option<&[u8]>) -> String {
    if blob.is_some() {
        format!("[message type {msg_type}/{sub_msg_type}]")
    } else {
        format!("[empty type {msg_type}/{sub_msg_type}]")
    }
}

fn summarize_message_json(v: &Value) -> String {
    let mut parts = Vec::new();
    collect_message_text(v, &mut parts);
    parts.join(" ").trim().to_string()
}

fn collect_message_text(v: &Value, parts: &mut Vec<String>) {
    match v {
        Value::Object(map) => {
            if let Some(text) = map.get("messageText").and_then(|x| x.as_str()) {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.to_string());
                }
            }
            if let Some(text) = map.get("imageText").and_then(|x| x.as_str()) {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    parts.push(trimmed.to_string());
                }
            }
            if let Some(text) = map.get("fileName").and_then(|x| x.as_str()) {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    parts.push(format!("[file:{trimmed}]"));
                }
            }
            for value in map.values() {
                collect_message_text(value, parts);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_message_text(item, parts);
            }
        }
        _ => {}
    }
}
