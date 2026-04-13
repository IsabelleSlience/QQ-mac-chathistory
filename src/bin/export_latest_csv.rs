use anyhow::{Context, Result};
use clap::Parser;
use csv::Writer;
use qq_mac_export_tools::{
    open_encrypted_db, resolve_nt_db_root, sanitize_filename, summarize_message_from_row,
    trim_opt, ts_to_local_string,
};
use rusqlite::{Connection, Row};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "export-latest-csv")]
#[command(about = "Export NTQQ Mac chat history into one CSV per conversation")]
struct Args {
    #[arg(long)]
    key: String,

    #[arg(long)]
    db_root: Option<PathBuf>,

    #[arg(long, default_value = "./exports")]
    output: PathBuf,

    #[arg(long)]
    self_uid: Option<String>,
}

#[derive(Debug, Clone)]
struct DirectLabel {
    remark: Option<String>,
    nickname: Option<String>,
    qq_number: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
struct IndexRecord {
    kind: String,
    conversation_id: String,
    conversation_number: Option<i64>,
    label: String,
    file_name: String,
    message_count: u64,
}

#[derive(Debug)]
struct ExportRow {
    conversation_id: String,
    conversation_number: Option<i64>,
    sender_uid: String,
    sender_uin: Option<i64>,
    send_time: i64,
    msg_type: i64,
    sub_msg_type: i64,
    send_status: i64,
    message_summary: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let db_root = resolve_nt_db_root(args.db_root.as_deref())?;
    let nt_msg = open_encrypted_db(&db_root.join("nt_msg.db"), &args.key)?;
    let profile_info = open_encrypted_db(&db_root.join("profile_info.db"), &args.key)?;

    let output_root = args.output;
    let direct_dir = output_root.join("direct");
    let group_dir = output_root.join("group");
    fs::create_dir_all(&direct_dir)?;
    fs::create_dir_all(&group_dir)?;

    let self_uid = match args.self_uid {
        Some(uid) => uid,
        None => detect_self_uid(&nt_msg)?,
    };

    let direct_labels = load_direct_labels(&profile_info)?;
    let group_labels = load_group_labels(&nt_msg)?;

    let direct_index = export_split_csv(
        &nt_msg,
        "c2c_msg_table",
        "direct",
        &direct_dir,
        &self_uid,
        &direct_labels,
        &group_labels,
    )?;
    let group_index = export_split_csv(
        &nt_msg,
        "group_msg_table",
        "group",
        &group_dir,
        &self_uid,
        &direct_labels,
        &group_labels,
    )?;

    write_index_csv(&output_root.join("direct_index.csv"), &direct_index)?;
    write_index_csv(&output_root.join("group_index.csv"), &group_index)?;
    write_readme(&output_root, &db_root, &self_uid, direct_index.len(), group_index.len())?;

    println!("export complete: {}", output_root.display());
    Ok(())
}

fn detect_self_uid(conn: &Connection) -> Result<String> {
    let mut stmt = conn.prepare(
        r#"
        SELECT "40020", COUNT(*) AS cnt
        FROM c2c_msg_table
        WHERE "40033" IS NOT NULL AND "40033" > 0
        GROUP BY "40020"
        ORDER BY cnt DESC
        LIMIT 1
        "#,
    )?;
    let uid: String = stmt.query_row([], |row| row.get(0)).context("detect self uid")?;
    Ok(uid)
}

fn load_direct_labels(conn: &Connection) -> Result<HashMap<String, DirectLabel>> {
    let mut labels = HashMap::new();
    let mut stmt = conn.prepare(
        r#"
        SELECT "1000" AS uid, "20009" AS remark, "20002" AS nickname, "20003" AS qq_number
        FROM profile_info_v6
        "#,
    )?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let uid: String = row.get("uid")?;
        labels.insert(
            uid,
            DirectLabel {
                remark: trim_opt(row.get("remark")?),
                nickname: trim_opt(row.get("nickname")?),
                qq_number: row.get("qq_number")?,
            },
        );
    }
    Ok(labels)
}

fn load_group_labels(conn: &Connection) -> Result<HashMap<String, String>> {
    let mut labels = HashMap::new();
    let mut stmt = conn.prepare(
        r#"
        SELECT
            "40021" AS conversation_id,
            "40090" AS c1,
            "40093" AS c2,
            "40094" AS c3,
            "40095" AS c4,
            "40096" AS c5,
            "41110" AS c6,
            "40022" AS c7,
            "40092" AS c8,
            "40091" AS c9
        FROM recent_contact_v3_table
        WHERE "40021" NOT LIKE 'u_%'
        "#,
    )?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let cid: String = row.get("conversation_id")?;
        let mut best = None;
        for col in ["c3", "c1", "c2", "c4", "c5", "c6", "c7", "c8", "c9"] {
            let value: Option<String> = row.get(col)?;
            if let Some(v) = trim_opt(value) {
                if !v.starts_with("u_") {
                    best = Some(v);
                    break;
                }
            }
        }
        if let Some(label) = best {
            labels.insert(cid, label);
        }
    }
    Ok(labels)
}

fn export_split_csv(
    conn: &Connection,
    table: &str,
    kind: &str,
    output_dir: &Path,
    self_uid: &str,
    direct_labels: &HashMap<String, DirectLabel>,
    group_labels: &HashMap<String, String>,
) -> Result<Vec<IndexRecord>> {
    let mut index = Vec::new();
    let mut used_names = HashSet::new();
    let mut errors = Vec::new();
    let conversation_ids = match load_conversation_ids(conn, table) {
        Ok(ids) => ids,
        Err(err) if kind == "group" => {
            eprintln!("fallback group conversation id loading: {err}");
            load_group_conversation_ids_from_recent_contact(conn)?
        }
        Err(err) => return Err(err),
    };

    for cid in conversation_ids {
        match export_one_conversation(
            conn,
            table,
            kind,
            output_dir,
            self_uid,
            direct_labels,
            group_labels,
            &cid,
            &mut used_names,
        ) {
            Ok(Some(row)) => index.push(row),
            Ok(None) => {}
            Err(err) => errors.push(format!("{kind},{cid},{err}")),
        }
    }

    if !errors.is_empty() {
        let path = output_dir
            .parent()
            .unwrap_or(output_dir)
            .join(format!("{kind}_errors.csv"));
        let mut writer = BufWriter::new(File::create(&path)?);
        writeln!(writer, "kind,conversation_id,error")?;
        for line in errors {
            writeln!(writer, "{line}")?;
        }
        writer.flush()?;
    }

    Ok(index)
}

fn load_conversation_ids(conn: &Connection, table: &str) -> Result<Vec<String>> {
    let sql = format!(r#"SELECT DISTINCT "40021" FROM {table} ORDER BY "40021" ASC"#);
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut ids = Vec::new();
    for row in rows {
        ids.push(row?);
    }
    Ok(ids)
}

fn load_group_conversation_ids_from_recent_contact(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT DISTINCT "40021"
        FROM recent_contact_v3_table
        WHERE "40021" NOT LIKE 'u_%'
        ORDER BY "40021" ASC
        "#,
    )?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut ids = Vec::new();
    for row in rows {
        ids.push(row?);
    }
    Ok(ids)
}

fn export_one_conversation(
    conn: &Connection,
    table: &str,
    kind: &str,
    output_dir: &Path,
    self_uid: &str,
    direct_labels: &HashMap<String, DirectLabel>,
    group_labels: &HashMap<String, String>,
    conversation_id: &str,
    used_names: &mut HashSet<String>,
) -> Result<Option<IndexRecord>> {
    let sql = format!(
        r#"
        SELECT
            "40021" AS conversation_id,
            "40027" AS conversation_number,
            "40020" AS sender_uid,
            "40033" AS sender_uin,
            "40050" AS send_time,
            "40011" AS msg_type,
            "40012" AS sub_msg_type,
            "40041" AS send_status,
            "40800" AS message_blob
        FROM {table}
        WHERE "40021" = ?
        ORDER BY "40050" ASC, "40001" ASC
        "#,
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query([conversation_id])?;
    let mut writer: Option<Writer<BufWriter<File>>> = None;
    let mut count = 0_u64;
    let mut conversation_number: Option<i64> = None;
    let mut current_label = String::new();
    let mut current_file = String::new();

    while let Some(row) = rows.next()? {
        let rec = map_row(row)?;
        if writer.is_none() {
            conversation_number = rec.conversation_number;
            let label = if kind == "direct" {
                direct_display_name(&rec.conversation_id, direct_labels)
            } else {
                group_display_name(&rec.conversation_id, rec.conversation_number, group_labels)
            };
            current_label = label.clone();
            current_file = unique_file_name(kind, &label, used_names);
            let path = output_dir.join(&current_file);
            let file = File::create(&path).with_context(|| format!("create {}", path.display()))?;
            let mut csv = csv::Writer::from_writer(BufWriter::new(file));
            csv.write_record([
                "conversation_id",
                "conversation_number",
                "conversation_label",
                "sender_uid",
                "sender_uin",
                "is_self",
                "send_time",
                "send_time_iso",
                "msg_type",
                "sub_msg_type",
                "send_status",
                "message_summary",
            ])?;
            writer = Some(csv);
        }

        let row_out = [
            rec.conversation_id.clone(),
            rec.conversation_number.map(|v| v.to_string()).unwrap_or_default(),
            current_label.clone(),
            rec.sender_uid.clone(),
            rec.sender_uin.map(|v| v.to_string()).unwrap_or_default(),
            (rec.sender_uid == self_uid).to_string(),
            rec.send_time.to_string(),
            ts_to_local_string(rec.send_time),
            rec.msg_type.to_string(),
            rec.sub_msg_type.to_string(),
            rec.send_status.to_string(),
            rec.message_summary.clone(),
        ];
        if let Some(csv) = writer.as_mut() {
            csv.write_record(row_out)?;
        }
        count += 1;
    }

    if let Some(csv) = writer.as_mut() {
        csv.flush()?;
        return Ok(Some(IndexRecord {
            kind: kind.to_string(),
            conversation_id: conversation_id.to_string(),
            conversation_number,
            label: current_label,
            file_name: current_file,
            message_count: count,
        }));
    }

    Ok(None)
}

fn map_row(row: &Row) -> rusqlite::Result<ExportRow> {
    let msg_type: i64 = row.get("msg_type")?;
    let sub_msg_type: i64 = row.get("sub_msg_type")?;
    Ok(ExportRow {
        conversation_id: row.get("conversation_id")?,
        conversation_number: row.get("conversation_number")?,
        sender_uid: row.get("sender_uid")?,
        sender_uin: row.get("sender_uin")?,
        send_time: row.get("send_time")?,
        msg_type,
        sub_msg_type,
        send_status: row.get("send_status")?,
        message_summary: summarize_message_from_row(row, "message_blob", msg_type, sub_msg_type)?,
    })
}

fn direct_display_name(conversation_id: &str, labels: &HashMap<String, DirectLabel>) -> String {
    if let Some(info) = labels.get(conversation_id) {
        if let Some(remark) = &info.remark {
            return remark.clone();
        }
        if let Some(nickname) = &info.nickname {
            if let Some(qq) = info.qq_number {
                return format!("{nickname}__{qq}");
            }
            return nickname.clone();
        }
        if let Some(qq) = info.qq_number {
            return qq.to_string();
        }
    }
    conversation_id.to_string()
}

fn group_display_name(
    conversation_id: &str,
    conversation_number: Option<i64>,
    labels: &HashMap<String, String>,
) -> String {
    if let Some(label) = labels.get(conversation_id) {
        if let Some(num) = conversation_number {
            return format!("{label}__{num}");
        }
        return label.clone();
    }
    conversation_number
        .map(|v| v.to_string())
        .unwrap_or_else(|| conversation_id.to_string())
}

fn unique_file_name(kind: &str, label: &str, used: &mut HashSet<String>) -> String {
    let base = sanitize_filename(label);
    let fallback = if base.is_empty() {
        kind.to_string()
    } else {
        base
    };
    let mut candidate = format!("{fallback}.csv");
    if !used.contains(&candidate) {
        used.insert(candidate.clone());
        return candidate;
    }
    let mut i = 2_u32;
    loop {
        candidate = format!("{fallback}__{i}.csv");
        if !used.contains(&candidate) {
            used.insert(candidate.clone());
            return candidate;
        }
        i += 1;
    }
}

fn write_index_csv(path: &Path, rows: &[IndexRecord]) -> Result<()> {
    let file = File::create(path).with_context(|| format!("create {}", path.display()))?;
    let mut writer = csv::Writer::from_writer(BufWriter::new(file));
    writer.write_record([
        "kind",
        "conversation_id",
        "conversation_number",
        "label",
        "file_name",
        "message_count",
    ])?;
    for row in rows {
        writer.serialize(row)?;
    }
    writer.flush()?;
    Ok(())
}

fn write_readme(
    output_root: &Path,
    db_root: &Path,
    self_uid: &str,
    direct_count: usize,
    group_count: usize,
) -> Result<()> {
    let path = output_root.join("README.txt");
    let mut writer = BufWriter::new(File::create(&path)?);
    writeln!(writer, "CSV export generated from NTQQ database files.")?;
    writeln!(writer, "db_root={}", db_root.display())?;
    writeln!(writer, "self_uid={self_uid}")?;
    writeln!(writer, "direct_files={direct_count}")?;
    writeln!(writer, "group_files={group_count}")?;
    writeln!(writer, "group_errors.csv is present when some conversations hit malformed pages.")?;
    writer.flush()?;
    Ok(())
}
