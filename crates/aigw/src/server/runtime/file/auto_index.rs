use std::{
    os::unix::fs::MetadataExt,
    path::PathBuf,
    time::{Duration, UNIX_EPOCH},
};

use dyn_fmt::AsStrFormatExt;
use http::Uri;

use crate::{SERVER, version::VERSION};

const ROW_DIR: &str = r#"
<tr class="current-item">
    <td>
        <div class="item">
            <svg class="icon" viewBox="0 0 24 24">
                <path fill="\#FFA000" d="M10 4H4c-1.11 0-2 .89-2 2v12c0 1.11.89 2 2 2h16c1.11 0 2-.89 2-2V8c0-1.11-.89-2-2-2h-8l-2-2z"/>
            </svg>
            <a href="{}">{}</a>
        </div>
    </td>
    <td>{}</td>
    <td>{}</td>
</tr>
"#;

const ROW_FILE: &str = r#"
<tr class="current-item">
    <td>
        <div class="item">
            <svg class="icon" viewBox="0 0 24 24">
                <path fill="\#4CAF50" d="M14,2H6A2,2 0 0,0 4,4V20A2,2 0 0,0 6,22H18A2,2 0 0,0 20,20V8L14,2M18,20H6V4H13V9H18V20Z"/>
            </svg>
            <a href="{}">{}</a>
        </div>
    </td>
    <td>{}</td>
    <td>{}</td>
</tr>
"#;

const HTML: &str = r#"
<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Index of /</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }

        body {
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: #333;
            min-height: 100vh;
            display: flex;
            flex-direction: column;
        }

        .container {
            flex: 1;
            display: flex;
            justify-content: center;
            align-items: flex-start;
            padding: 2rem;
        }

        .index-container {
            width: 100%;
            max-width: 800px;
            background: white;
            border-radius: 20px;
            box-shadow: 0 20px 40px rgba(0, 0, 0, 0.1);
            padding: 2rem;
            overflow: hidden;
        }

        h1 {
            font-size: 2rem;
            font-weight: 700;
            margin-bottom: 1.5rem;
            color: #2d3748;
        }

        table {
            width: 100%;
            border-collapse: collapse;
            font-family: monospace;
        }

        th, td {
            padding: 0.75rem 1rem;
            text-align: left;
            vertical-align: top;
        }

        th {
            font-weight: bold;
            color: #4a5568;
            border-bottom: 1px solid #e2e8f0;
        }

        .item {
            display: flex;
            align-items: center;
            gap: 12px;
        }

        .icon {
            width: 18px;
            height: 18px;
        }

        a {
            text-decoration: none;
            color: #000;
        }

        a:hover {
            text-decoration: underline;
        }

        .current-item td:first-child {
            padding-left: 2rem;
        }

        .server-info {
            text-align: center;
            padding: 1rem;
            color: rgba(255, 255, 255, 0.8);
            font-size: 0.9rem;
            font-family: monospace;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="index-container">
            <h1>Index of /</h1>
            <table>
                <thead>
                    <tr>
                        <th>Name</th>
                        <th>Last modified</th>
                        <th>Size</th>
                    </tr>
                </thead>
                <tbody>
                    <tr>
                        <td>
                            <div class="item">
                                <svg class="icon" viewBox="0 0 24 24">
                                    <path fill="\#FFA000" d="M10 4H4c-1.11 0-2 .89-2 2v12c0 1.11.89 2 2 2h16c1.11 0 2-.89 2-2V8c0-1.11-.89-2-2-2h-8l-2-2z"/>
                                </svg>
                                <a href="../">../</a>
                            </div>
                        </td>
                        <td></td>
                        <td></td>
                    </tr>
                    {}
                </tbody>
            </table>
        </div>
    </div>

    <div class="server-info">{}/{}</div>
</body>
</html>
"#;

pub async fn build_auto_index(uri: &Uri, path: &PathBuf) -> String {
    let mut rows = String::new();

    if let Ok(mut entries) = tokio::fs::read_dir(&path).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();

            if let Ok(m) = path.metadata() {
                let file_name = path
                    .file_name()
                    .map_or("", |s| s.to_str().map_or("", |s| s));
                if m.is_file() && file_name.starts_with(".") {
                    continue;
                }

                let mut href = if !uri.path().eq("/") {
                    uri.path().to_owned() + file_name
                } else {
                    file_name.to_owned()
                };
                if m.is_dir() {
                    href += "/";
                }

                let file_display_name = smart_truncate(file_name, 60);
                let gmt_create =
                    httpdate::fmt_http_date(UNIX_EPOCH + Duration::from_secs(m.ctime() as u64));
                let file_size = m.len().to_string();

                if m.is_file() {
                    rows += &ROW_FILE.format(&[
                        "file",
                        href.as_str(),
                        &file_display_name,
                        &gmt_create,
                        &file_size,
                    ]);
                } else {
                    rows += &ROW_DIR.format(&[
                        href.as_str(),
                        &file_display_name,
                        &gmt_create,
                        &file_size,
                    ]);
                }
            }
        }
    }

    HTML.format(&[&rows, SERVER, VERSION])
}

fn smart_truncate(s: &str, width: usize) -> String {
    if s.len() > width {
        s[0..width].to_string() + "..."
    } else {
        s.to_string()
    }
}
