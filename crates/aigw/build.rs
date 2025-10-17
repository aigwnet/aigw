use std::{env, fs, path::Path, process::Command};

static VERSION_TEMPLATE: &str = r#"
pub static VERSION: &str = "{version}";
"#;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    let dest_path = Path::new(&out_dir).join("version.rs");
    let version_file = "./VERSION";
    let version = fs::read_to_string(version_file).unwrap();
    let s = VERSION_TEMPLATE.replace("{version}", &version);
    fs::write(dest_path, s).unwrap();

    if !fs::exists("target").unwrap() {
        fs::create_dir("target").unwrap();
    }

    // Process manpage using asciidoctor command
    fs::create_dir_all(&out_dir).unwrap();
    fs::copy("aigw.adoc", Path::new(&out_dir).join("aigw.adoc")).unwrap();
    match Command::new("asciidoctor")
        .args(&["-b", "manpage", "aigw.adoc"])
        .current_dir(&Path::new(&out_dir))
        .status()
    {
        Ok(_) => {
            Command::new("gzip")
                .args(&["aigw.1"])
                .current_dir(&Path::new(&out_dir))
                .status()
                .unwrap();
            fs::copy(Path::new(&out_dir).join("aigw.1.gz"), "target/aigw.1.gz").unwrap();
        }
        Err(err) => {
            println!("cargo:warning=Error building manpage: {}", err);
            println!("cargo:warning=The manpage will not be build. Do you have 'asciidoctor'?");
        }
    }
}
