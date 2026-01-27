use chrono::DateTime;
use crate::model::archive::Archive;
use crate::model::path_detail::PathDetailType;

pub struct HtmlDirectoryRenderer;

impl HtmlDirectoryRenderer {
    pub fn render(archive: &Archive, path: String) -> String {
        let mut output = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Index of /"#.to_string();
        output.push_str(&path);
        output.push_str(r#"</title>
    <style>
        :root {
            --bg-color: #f4f4f9;
            --text-color: #333;
            --header-bg: #fff;
            --row-hover: #f0f0f0;
            --border-color: #ddd;
            --accent-color: #007bff;
        }
        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
            background-color: var(--bg-color);
            color: var(--text-color);
            margin: 0;
            padding: 20px;
        }
        .container {
            max-width: 1000px;
            margin: 0 auto;
            background: var(--header-bg);
            padding: 20px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        h1 {
            font-size: 1.5rem;
            margin-bottom: 20px;
            border-bottom: 2px solid var(--accent-color);
            padding-bottom: 10px;
        }
        table {
            width: 100%;
            border-collapse: collapse;
        }
        th, td {
            padding: 12px 15px;
            text-align: left;
            border-bottom: 1px solid var(--border-color);
        }
        th {
            background-color: #fafafa;
            font-weight: 600;
            text-transform: uppercase;
            font-size: 0.85rem;
            color: #666;
        }
        tr:hover {
            background-color: var(--row-hover);
        }
        a {
            color: var(--accent-color);
            text-decoration: none;
            display: flex;
            align-items: center;
        }
        a:hover {
            text-decoration: underline;
        }
        a:visited {
            color: #551A8B;
        }
        .icon {
            margin-right: 10px;
            width: 18px;
            height: 18px;
            flex-shrink: 0;
        }
        .size, .mtime {
            color: #666;
            font-size: 0.9rem;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>Index of /"#);
        output.push_str(&path);
        output.push_str(r#"</h1>
        <table>
            <thead>
                <tr>
                    <th>Name</th>
                    <th>Last Modified</th>
                    <th>Size</th>
                </tr>
            </thead>
            <tbody>
"#);

        for path_detail in archive.list_dir(path) {
            let mtime_datetime = DateTime::from_timestamp_millis(i64::try_from(path_detail.modified)
                .unwrap_or(0) * 1000);
            let mtime_iso = mtime_datetime.map(|dt| dt.format("%Y-%m-%dT%H:%M:%S%:z").to_string())
                .unwrap_or_else(|| "N/A".to_string());
            
            let icon = match path_detail.path_type {
                PathDetailType::DIRECTORY => r#"<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z"/></svg>"#,
                PathDetailType::FILE => {
                    let ext = path_detail.path.split('.').last().unwrap_or("").to_lowercase();
                    match ext.as_str() {
                        "pdf" => r#"<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><path d="M9 15l3-3 3 3"/><path d="M12 12v9"/></svg>"#,
                        "jpg" | "jpeg" | "png" | "gif" | "svg" | "webp" => r#"<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect width="18" height="18" x="3" y="3" rx="2" ry="2"/><circle cx="9" cy="9" r="2"/><path d="m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21"/></svg>"#,
                        "zip" | "tar" | "gz" | "7z" | "rar" => r#"<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 10V4a2 2 0 0 0-2-2H5a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V14"/><path d="M12 2v18"/><path d="M12 7h3"/><path d="M9 11h3"/><path d="M12 15h3"/><path d="M9 19h3"/></svg>"#,
                        _ => r#"<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/></svg>"#,
                    }
                }
            };

            output.push_str("                <tr>\n");
            output.push_str(&format!("                    <td><a href=\"{}\">{} {}</a></td>\n", path_detail.path, icon, path_detail.display));
            output.push_str(&format!("                    <td class=\"mtime\">{}</td>\n", mtime_iso));
            output.push_str(&format!("                    <td class=\"size\">{}</td>\n", format_size(path_detail.size)));
            output.push_str("                </tr>\n");
        }

        output.push_str(r#"            </tbody>
        </table>
    </div>
</body>
</html>"#);
        output
    }
}

fn format_size(size: u64) -> String {
    if size == 0 { return "-".to_string(); }
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_idx = 0;
    while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }
    format!("{:.1} {}", size, UNITS[unit_idx])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::archive::Archive;
    use std::collections::HashMap;
    use autonomi::data::DataAddress;
    use xor_name::XorName;

    #[test]
    fn test_render_empty_archive() {
        let archive = Archive::new(HashMap::new(), vec![]);
        let output = HtmlDirectoryRenderer::render(&archive, "test/".to_string());
        assert!(output.contains("Index of /test/"));
        assert!(output.contains("<table>"));
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "-");
        assert_eq!(format_size(512), "512.0 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
    }

    #[test]
    fn test_visited_links_css() {
        let archive = Archive::new(HashMap::new(), vec![]);
        let output = HtmlDirectoryRenderer::render(&archive, "".to_string());
        assert!(output.contains("a:visited {"));
        assert!(output.contains("color: #551A8B;"));
    }
}
