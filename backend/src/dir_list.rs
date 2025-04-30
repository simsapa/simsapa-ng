use time;
use walkdir::WalkDir;
use std::time::{SystemTime, UNIX_EPOCH};
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

/// Represents a single directory entry with its metadata
pub struct DirInfo {
    name: String,
    size: u64,
    is_directory: bool,
    modified: Option<u64>,
    relative_path: String,
}

impl fmt::Display for DirInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

pub fn generate_directory_listing(root_path: &str, max_depth: usize) -> Result<Vec<(PathBuf, DirInfo)>, Box<dyn Error>> {
    let root_path = Path::new(root_path);

    let mut entries: Vec<(PathBuf, DirInfo)> = WalkDir::new(root_path)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|entry| {
            let metadata = entry.metadata().unwrap();
            let mut relative_path = entry.path().strip_prefix(root_path)
                .unwrap_or_else(|_| entry.path())
                .to_string_lossy()
                .into_owned();

            if relative_path.len() == 0 {
                relative_path.push_str(".");
            }

            (
                entry.path().to_path_buf(),
                DirInfo {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    size: metadata.len(),
                    is_directory: metadata.is_dir(),
                    modified: metadata.modified().ok().map(|t|
                        t.duration_since(UNIX_EPOCH).unwrap().as_secs()),
                    relative_path,
                }
            )
        })
        .collect();

    entries.sort_by_key(|(_, info)| info.relative_path.clone());

    Ok(entries)
}

pub fn generate_html_directory_listing(root_path: &str, max_depth: usize) -> Result<String, Box<dyn Error>> {
    let entries = generate_directory_listing(root_path, max_depth)?;

    let mut res = String::from(
        "<table border='1' style='font-size: 0.7em'>
            <tr>
                <th>Path</th>
                <th>Size</th>
                <th>Modified</th>
                <th>Type</th>
            </tr>"
    );

    for (_, info) in entries {
        let modified = info.modified.map_or("N/A".to_string(), |t| format_timestamp(t));

        let entry_type = if info.is_directory { "Dir" } else { "File" };

        let row = format!(
            "<tr>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
            </tr>",
            escape_html(&info.relative_path),
            format_size(info.size),
            modified,
            entry_type,
        );

        res.push_str(&row);
    }

    res.push_str("</table>");

    Ok(res)
}

pub fn generate_plain_directory_listing(root_path: &str, max_depth: usize) -> Result<String, Box<dyn Error>> {
    let entries = generate_directory_listing(root_path, max_depth)?;

    let mut res = String::from("| Path | Size | Modified | Type |\n|------+------+----------+------|\n");

    for (_, info) in entries {
        let modified = info.modified.map_or("N/A".to_string(), |t| format_timestamp(t));

        let entry_type = if info.is_directory { "Dir" } else { "File" };

        let row = format!(
            "| {} | {} | {} | {} |\n",
            info.relative_path,
            format_size(info.size),
            modified.trim(),
            entry_type,
        );

        res.push_str(&row);
    }

    Ok(res)
}

fn format_timestamp(seconds: u64) -> String {
    let dt = UNIX_EPOCH + std::time::Duration::from_secs(seconds);
    format_system_time(dt)
}

// https://users.rust-lang.org/t/how-to-get-year-month-day-etc-from-systemtime/84588
fn format_system_time(t: SystemTime) -> String {
    let utc = time::OffsetDateTime::UNIX_EPOCH + time::Duration::try_from(t.duration_since(std::time::UNIX_EPOCH).unwrap()).unwrap();
    let local = utc.to_offset(time::UtcOffset::local_offset_at(utc).unwrap());

    let mut buffer = Vec::new();
    local.format_into(
        &mut buffer,
        time::macros::format_description!(
            "[day]-[month repr:short]-[year] [hour]:[minute]:[second]\n"
        ),
    ).unwrap();

    String::from_utf8(buffer).unwrap()
}

/// Helper function to format file size in human-readable units
fn format_size(size: u64) -> String {
    match size {
        0 => "0 bytes".to_string(),
        1..=1024 => format!("{} bytes", size),
        // 1024*1024
        1025..=1048576 => format!("{:.1} KB", size as f64 / 1024.0),
        _ => format!("{:.1} MB", size as f64 / (1024.0 * 1024.0)),
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
     .replace('\'', "&#x27;")
}
