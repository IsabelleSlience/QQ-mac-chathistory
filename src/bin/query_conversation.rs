use anyhow::Result;
use clap::Parser;
use qq_mac_export_tools::{open_encrypted_db, resolve_nt_db_root, ts_to_local_string};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "query-conversation")]
#[command(about = "Inspect a single C2C conversation in an NTQQ Mac database")]
struct Args {
    #[arg(long)]
    key: String,

    #[arg(long)]
    db_root: Option<PathBuf>,

    conversation_id: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let db_root = resolve_nt_db_root(args.db_root.as_deref())?;
    let conn = open_encrypted_db(&db_root.join("nt_msg.db"), &args.key)?;

    let mut stmt = conn.prepare(
        r#"
        SELECT COUNT(*), MIN("40050"), MAX("40050")
        FROM c2c_msg_table
        WHERE "40021" = ?
        "#,
    )?;
    let (count, min_ts, max_ts): (i64, Option<i64>, Option<i64>) =
        stmt.query_row([args.conversation_id.as_str()], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;

    println!("conversation_id={}", args.conversation_id);
    println!("count={count}");
    println!("min_ts={:?}", min_ts);
    println!("max_ts={:?}", max_ts);
    if let Some(ts) = min_ts {
        println!("min_iso={}", ts_to_local_string(ts));
    }
    if let Some(ts) = max_ts {
        println!("max_iso={}", ts_to_local_string(ts));
    }

    let mut stmt = conn.prepare(
        r#"
        SELECT "40001", "40050", "40020", "40033"
        FROM c2c_msg_table
        WHERE "40021" = ?
        ORDER BY "40050" ASC, "40001" ASC
        LIMIT 5
        "#,
    )?;
    let mut rows = stmt.query([args.conversation_id.as_str()])?;
    while let Some(row) = rows.next()? {
        let id: i64 = row.get(0)?;
        let send_time: i64 = row.get(1)?;
        let sender_uid: String = row.get(2)?;
        let sender_uin: Option<i64> = row.get(3)?;
        println!(
            "row id={id} send_time={} sender_uid={} sender_uin={:?}",
            ts_to_local_string(send_time),
            sender_uid,
            sender_uin
        );
    }

    Ok(())
}
