use std::{
    os::unix::fs::MetadataExt,
    path::PathBuf,
    time::{Duration, UNIX_EPOCH},
};

use dyn_fmt::AsStrFormatExt;
use http::Uri;

use crate::{version::VERSION, SERVER};

const ROW: &str = r#"
        <div class="row p-1">
            <div class="col-6 ps-5"><label class="{} me-2"></label><a href="{}">{}</a></div>
            <div class="col-4 text-center">{}</div>
            <div class="col-2">{}</div>
        </div>
"#;

const HTML: &str = r#"
<!doctype html>
<html lang="en" class="h-100" data-bs-theme="auto">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Index of /</title>
    <link href="/.__aigw__reserved/assets/dist/css/bootstrap.min.css" rel="stylesheet">
    <style>
        main > .container {{
            padding: 60px 15px 0;
        }}
        
        .server {{
            text-align:center; font-size: 16px;
        }}
    
        .directory {{
            width: 16px; height: 16px;
            background-size: 16px;
            background-image: url(data:image/png;base64,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);
        }}

        .file {{
            width: 16px; height: 16px;
            background-size: 16px;
            background-image: url(data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAEgAAABICAMAAABiM0N1AAADAFBMVEVHcEz8/Pz19vcAAAAAAAAAAAD6+vv+/v4AAAD9/f34+fn7+/v39/j9/v7x8/Tv8fL09fbp7O7q7O7s7vDk5+rw8fPt7/Dh5Ojd4eTn6uzv8PL5+fr29/jt7/Lz9Pb3+Pn5+vry8/X7/Pz8/f3j5uro6u36+/v4+Pn8/P3r7e/g4+fb3uLx8vPi5unn6ezp6+7f4ubJzNAAAADc4OPx8vTm6evb3+Ta3uPv8fQAAADp6+0AAADl6Ovd4OQAAAAAAAAAAAAAAAD4+frBydHy8/QAAADR1tyyvccAAAAAAAAAAAD7+/yKiooAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAC7xMyotMC/yNCap7Tz9PXl6OwAAADQ09clJic7PD18jZ7q7e/b4OWbqLV8g4mQlJdfaXLN09lLUVYAAAAAAABsbGzJycl2dnfY2t3M0NWDkJ3///8AAAD+/v78/Pzb3+P8/P35+vr4+Pn29/j9/f34+fry8/T+///6+vv8/f39/f79/v719vfz9fbw8vPu8PL09vfy9PXr7e/s7vDZ3eHp6+7JzNDu8PHa3uL09fbo6+3j5ung4+fc4OTl6Ovi5eje4ubl5+rm6eudoe/e4eXy8/Wdn6GgoaPq7O/O1NrU2d/o6u2foaLi5OjR1tyqqqv5+frV29/W2+ACfPHJ0NebnZ+Zm52JiYkCAgJ8fHzt7/Hn6uzX3eLY3eG3v8jx8vTL0tianJ6Xmp3XxvLq6urGzdTX3OC9xs7w8fKnp6iho6SoqKmkpqitt8CgrLfHztXT2N3Gz9W5wsuvusSjpKXe4eSkpabc3+Gwu8alsbyVl5qsrKy9x88rLC2UoK3n6Ojz9PWst8O/x8+WpLGirbmImKaCg4MKCgp4enxdXmC8v8OusbSOnKvGys3Ey9F4ipvk5ebp6+3l6euksLtqcnt0e4Kos76Eio+Lj5SQn63w8vRTXmmGlaRyhJVFUV6AkaE0NDREREQMDAzb29u/v7/6+/tldYZxgZC6wcfT1tm7ogn0AAAAs3RSTlMA/v4yCur+/v7+/v7+/v7+/v7+/v/+/v/+/v7+/v7+/v7+/v7+/v7+/v7+/v7+/v7//rH+/v7+/v4I/gb+/rz50w3+/v4D/v4C6RP+/thMPmNU5Xr7GP7+/v7+/qf+/v7+/v7+/v7+//61pv7+/v7+/v///////////////////////////////////////////////////////////////////////////////////////uRswsQAAAYMSURBVFjDtZh5VBNXFMaFYjJRG5YsJAELKFEZEkCJ0GIQBMQl7ktVrAvd6N5mQmSLGwwKASMQUM6IbEXksCOgniK4gFTUtu77Lra1+75v8+CNgLyEpOf4nd/MN5M/vnNvXublzB027Mlo+tKnzOk5C2NmLw5dQmAEAY5+9F1OsrMoZ94ibChZlLQghPjtzT8ufvBIFy9+ip/66J1/3+3+p73jwnt///U2QUyaMmTO/BDsl9/ZajUNONgaDSf2z7CElGfTVu6o2H+uq+Peg1F1dHdDJU0Jxc63sdkcW8EjSSRCXrRU6SzaF3m8dcbDC2vD2HVDd7eQqH1Lo/GgtAFaLThovA36xoq9nWUxld2tTe3XLjwIU2vq6O7MJ71A/MrR6IKNRn9jz9EDndVYURxZVNl9pam94+ew2HJ23RBrN3MuNodjg4sek7+/d+Ch4s7I/S3Lm9qvu7hQLhzQnZnfk90sYs/YFGqwTvlr9XRRMcfDm+65lxuouMQ6s2tnN5w4a0P64X4AaL3QUYZDxWVFx69cZZUb9FQcp8bc2tkNx86OEeAo+VGiAP3ezqKWl1guer2ecmHPMdMdXdEeFZmHkjOeQwXTRcV0gIoMBqpcPYfubprpoJFktjwbAI0BZFHaxs6HvUF6qpxdU4tNnWaqtT0sAR8pT7n8fXxHcMUyN5a3Xt/bXQ1mIglUxCJzlbkAaAzKnXw5XZTxslTjByoygLWrqSWm2pkIcifzJ+cDoDHQJyU/Oy/nXIua9Db0dpdkQ68dKgm0FqYQ+4gB0BjAabKSL+9cJmAH9TanF7mq0N3RFR0d0ZwlzQJAYwCnkgaf3OzwjxPYl/gio5F+kvCE+BsEMTgJBLk1Z/hmAKAx9Lq0IR9v7Vqp6dlj1PQeEx91oxZ7ffrg1o6OUjiZk2+QWF7ZdSLyxYDAfXvBQ9N94jVsuB2iIlddmlcaABoD44WOWQ14WUt464kZlx+u+PaHtT/+hAw67KpL56UDoDE8uvQqzJDm850pQ3FMy/Nd165e/x4VhB225Qp5QgA0hr5LnpdTkJgv3xFYXEQnfbLiVXRFtrptwm0AaAx9l/X1vLTCoM+zcW1FEb37fvUFOshBlypJBUBj6H8nrE+Pzsj3zDFWlH24vOlLdGsOEfZDS7KN5+Uo5juLDkVWhr+MrOhYlI4UkABoDAPv2iR0UVm5cqqxc//XpoJ0sh6gMQy8IwVtqcK0z3yy6a/8G2RrpyMiuJZIoRDQf3iOPp544CvIik5zd2312AqAxuDx2Ae7dAoylVdY4ul3Bh0k06VEpQCgMUQ9/sElDx1pX58mzj6IbO2kwMPBQj0T5aEQjHZSHkBWdFKg2+KwBQCNwWHLILZvp5eYV1KFDpLsSrZNBkBjsE1G4JCyVZhRgGztiCTC1ho52Ecjg4gjQl2CawIAGoNrApJkWVopuiIhd5Q1cpXxkEHEbp4u3i0eAI3BLR4NN323qaCk8UkAaAzjk9Bw09Gt7fbijrBKXC90RaXRzXEucQBoDC5xaBToL5soddJtcN8AgMbgvgGNohDdWqmvgmWVFL7oigqCmlUsFQAaA0uFRoH+ZRMF0u9U41QAaAzjVGhkWehHpEAsG2mVZGITFfmQ68asA0BjGLMOjawBHVQ1mdxosxEAjcFmIxpZPnIbwaqUMhurJFOi96OqnWTihEQANIYJiWgEuSaC+OSmsZsA0BjGbkIj4JsIkpObOZsB0Bg4m9GYCMIOyO05Vslejt78D+S1aTgaADQGjgaNJA8dVO2cam1QNbK1alzytFWS4NXoivDU9ez1AGgM7PVoJDnooIN4qrrfW3YfbDWa0Tnov+yDlJBtlYQUKmgWcYYqsa4iKXUGm2U36OWYuCXyi7UmKBanbmFzZw56XcfW+AcHWdNZULD/GuwN1ADhrtEoVVlakarE6H+3FluIGmmsXhUQIFI6WiSlKCBg1WosdApyyHL+jrdWS6P1HoC3Fsmd80TIfBNjn5u3j92fmJmZOXEAEzMHc//Y7ZtYyAJTgyg4uTI50RrIonnmRmOYhVoSunj2/x/W9dPS6U9oXvgfJx4zkSjlJuIAAAAASUVORK5CYII=);
        }}
    </style>
</head>
<body class="d-flex flex-column h-100">
<header>
    <nav class="navbar navbar-expand-md navbar-dark fixed-top bg-dark">
        <div class="container">
            <span class="navbar-brand">Index of /</span>
        </div>
    </nav>
</header>
<main class="flex-shrink-0">
    <div class="container">
        <div class="row p-1"><div class="col">
            <label class="directory me-2"></label><a href="../">../</a></div>
        </div>
        {}
    </div>
</main>

<footer class="footer mt-auto py-3 bg-body-tertiary">
    <div class="container">
        <span class="text-body-secondary">{}/{}</span>
    </div>
</footer>
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
                    rows += &ROW.format(&[
                        "file",
                        href.as_str(),
                        &file_display_name,
                        &gmt_create,
                        &file_size,
                    ]);
                } else {
                    rows += &ROW.format(&[
                        "directory",
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
