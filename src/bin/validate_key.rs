use anyhow::{Context, Result};
use clap::Parser;
use qq_mac_export_tools::{
    nt_db_candidates, open_encrypted_db, resolve_nt_db_root, table_exists, ts_to_local_string,
};
use rusqlite::Connection;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "validate-key")]
#[command(about = "Validate a candidate NTQQ database key against a macOS QQ nt_db directory")]
struct Args {
    #[arg(long)]
    key: String,

    #[arg(long)]
    db_root: Option<PathBuf>,

    /// Print extra diagnostics for each successfully opened database.
    #[arg(long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let db_root = resolve_nt_db_root(args.db_root.as_deref())?;

    println!("db_root={}", db_root.display());
    println!();

    let candidates = nt_db_candidates(&db_root);
    let mut opened = 0usize;

    for (label, path) in candidates {
        if !path.exists() {
            println!("{label}: missing ({})", path.display());
            continue;
        }

        match open_encrypted_db(&path, &args.key) {
            Ok(conn) => {
                opened += 1;
                println!("{label}: ok");
                println!("  path={}", path.display());
                print_db_summary(label, &conn, args.verbose)?;
            }
            Err(err) => {
                println!("{label}: failed");
                println!("  path={}", path.display());
                println!("  error={err}");
            }
        }
        println!();
    }

    println!("opened_databases={opened}");
    if opened == 0 {
        println!("validation_result=failed");
        println!("next_step=check whether the key is wrong, the db root is wrong, or the database is in an unexpected format");
    } else {
        println!("validation_result=partial_or_success");
        println!("next_step=use query_conversation or export_latest_csv on the same db_root with this key");
    }

    Ok(())
}

fn print_db_summary(label: &str, conn: &Connection, verbose: bool) -> Result<()> {
    match label {
        "nt_msg" => print_nt_msg_summary(conn, verbose)?,
        "profile_info" => print_profile_summary(conn, verbose)?,
        "group_info" => print_group_info_summary(conn, verbose)?,
        "recent_contact" => print_recent_contact_summary(conn, verbose)?,
        _ => {}
    }
    Ok(())
}

fn print_nt_msg_summary(conn: &Connection, verbose: bool) -> Result<()> {
    let has_c2c = table_exists(conn, "c2c_msg_table")?;
    let has_group = table_exists(conn, "group_msg_table")?;
    println!("  has_c2c_msg_table={has_c2c}");
    println!("  has_group_msg_table={has_group}");

    if has_c2c {
        let (count, min_ts, max_ts) = query_time_range(conn, "c2c_msg_table")
            .context("query c2c_msg_table time range")?;
        println!("  c2c_count={count}");
        if let Some(ts) = min_ts {
            println!("  c2c_min_ts={ts}");
            println!("  c2c_min_iso={}", ts_to_local_string(ts));
        }
        if let Some(ts) = max_ts {
            println!("  c2c_max_ts={ts}");
            println!("  c2c_max_iso={}", ts_to_local_string(ts));
        }
    }

    if has_group {
        let (count, min_ts, max_ts) = query_time_range(conn, "group_msg_table")
            .context("query group_msg_table time range")?;
        println!("  group_count={count}");
        if let Some(ts) = min_ts {
            println!("  group_min_ts={ts}");
            println!("  group_min_iso={}", ts_to_local_string(ts));
        }
        if let Some(ts) = max_ts {
            println!("  group_max_ts={ts}");
            println!("  group_max_iso={}", ts_to_local_string(ts));
        }
    }

    if verbose {
        let mut stmt = conn.prepare(
            r#"
            SELECT name
            FROM sqlite_master
            WHERE type='table'
            ORDER BY name ASC
            LIMIT 50
            "#,
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut names = Vec::new();
        for row in rows {
            names.push(row?);
        }
        println!("  sample_tables={}", names.join(", "));
    }

    Ok(())
}

fn print_profile_summary(conn: &Connection, verbose: bool) -> Result<()> {
    let has_v6 = table_exists(conn, "profile_info_v6")?;
    println!("  has_profile_info_v6={has_v6}");
    if has_v6 {
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM profile_info_v6", [], |row| row.get(0))?;
        println!("  profile_count={count}");
    }
    if verbose {
        let has_v2 = table_exists(conn, "profile_info_v2")?;
        println!("  has_profile_info_v2={has_v2}");
    }
    Ok(())
}

fn print_group_info_summary(conn: &Connection, _verbose: bool) -> Result<()> {
    let count = count_first_existing(conn, &["group_info_v2", "group_info_table", "group_info"])?;
    if let Some((table, count)) = count {
        println!("  group_table={table}");
        println!("  group_count={count}");
    } else {
        println!("  group_table=not_detected");
    }
    Ok(())
}

fn print_recent_contact_summary(conn: &Connection, _verbose: bool) -> Result<()> {
    let has_recent = table_exists(conn, "recent_contact_v3_table")?;
    println!("  has_recent_contact_v3_table={has_recent}");
    if has_recent {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM recent_contact_v3_table",
            [],
            |row| row.get(0),
        )?;
        println!("  recent_contact_count={count}");
    }
    Ok(())
}

fn query_time_range(conn: &Connection, table: &str) -> Result<(i64, Option<i64>, Option<i64>)> {
    let sql = format!(
        r#"SELECT COUNT(*), MIN("40050"), MAX("40050") FROM {table}"#
    );
    let out = conn.query_row(&sql, [], |row| {
        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
    })?;
    Ok(out)
}

fn count_first_existing(conn: &Connection, tables: &[&str]) -> Result<Option<(String, i64)>> {
    for table in tables {
        if table_exists(conn, table)? {
            let sql = format!("SELECT COUNT(*) FROM {table}");
            let count: i64 = conn.query_row(&sql, [], |row| row.get(0))?;
            return Ok(Some((table.to_string(), count)));
        }
    }
    Ok(None)
}
