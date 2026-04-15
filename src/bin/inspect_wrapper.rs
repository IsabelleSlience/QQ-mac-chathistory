use anyhow::{Context, Result};
use clap::Parser;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "inspect-wrapper")]
#[command(about = "Statically inspect wrapper.node for interesting NTQQ/SQLCipher markers")]
struct Args {
    /// Path to wrapper.node. If omitted, common macOS QQ locations will be searched.
    #[arg(long)]
    wrapper: Option<PathBuf>,

    /// Print every matched string instead of only grouped counts.
    #[arg(long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let wrapper = match args.wrapper {
        Some(path) => path,
        None => auto_detect_wrapper()?,
    };

    let bytes = fs::read(&wrapper)
        .with_context(|| format!("read wrapper file {}", wrapper.display()))?;
    let strings = extract_ascii_strings(&bytes, 4);

    let interesting = [
        "nt_sqlite3_key_v2",
        "sqlcipher",
        "sqlite3_key",
        "codec",
        "set_pass",
        "wrapper.node",
        "nt_msg.db",
        "profile_info.db",
        "group_msg_table",
        "c2c_msg_table",
        "recent_contact_v3_table",
    ];

    let mut matches: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for needle in interesting {
        let found: Vec<String> = strings
            .iter()
            .filter(|s| s.contains(needle))
            .cloned()
            .collect();
        if !found.is_empty() {
            matches.insert(needle, found);
        }
    }

    println!("wrapper={}", wrapper.display());
    println!("ascii_strings={}", strings.len());
    println!("markers_found={}", matches.len());
    println!();

    for (needle, found) in &matches {
        println!("{needle}: {}", found.len());
        if args.verbose {
            for item in found {
                println!("  {item}");
            }
        }
    }

    if matches.is_empty() {
        println!("No known markers found. This may indicate a different QQ build or stronger symbol stripping.");
    } else {
        println!();
        println!("Next step suggestions:");
        println!("- confirm the database path layout under nt_db");
        println!("- look for call sites around nt_sqlite3_key_v2 / codec markers in your disassembler");
        println!("- validate any recovered key against nt_msg.db in read-only mode");
    }

    Ok(())
}

fn auto_detect_wrapper() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME is not set")?;
    let candidates = [
        Path::new(&home).join(
            "Applications/QQ.app/Contents/Resources/app/wrapper.node",
        ),
        Path::new(&home).join(
            "Library/Containers/com.tencent.qq/Data/Library/Application Support/QQ/versions",
        ),
        PathBuf::from("/Applications/QQ.app/Contents/Resources/app/wrapper.node"),
    ];

    for candidate in candidates {
        if candidate.is_file() {
            return Ok(candidate);
        }
        if candidate.is_dir() {
            if let Some(found) = find_wrapper_under(&candidate)? {
                return Ok(found);
            }
        }
    }

    anyhow::bail!("could not auto-detect wrapper.node; pass --wrapper explicitly")
}

fn find_wrapper_under(root: &Path) -> Result<Option<PathBuf>> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = find_wrapper_under(&path)? {
                return Ok(Some(found));
            }
        } else if path.file_name().and_then(|s| s.to_str()) == Some("wrapper.node") {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

fn extract_ascii_strings(bytes: &[u8], min_len: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = Vec::new();

    for &b in bytes {
        if b.is_ascii_graphic() || b == b' ' {
            buf.push(b);
        } else {
            if buf.len() >= min_len {
                out.push(String::from_utf8_lossy(&buf).to_string());
            }
            buf.clear();
        }
    }
    if buf.len() >= min_len {
        out.push(String::from_utf8_lossy(&buf).to_string());
    }
    out
}
