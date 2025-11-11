use anyhow::{Context as _, anyhow};
use aya_build::Toolchain;
use std::{env, fs, path::Path, process::Command};

static VERSION_TEMPLATE: &str = r#"
pub static VERSION: &str = "{version}";
"#;

fn main() -> anyhow::Result<()> {
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

    // build ebpf
    if cfg!(target_os = "linux") {
        let cargo_metadata::Metadata { packages, .. } = cargo_metadata::MetadataCommand::new()
            .no_deps()
            .exec()
            .context("MetadataCommand::exec")?;
        let ebpf_package = packages
            .into_iter()
            .find(|cargo_metadata::Package { name, .. }| name.as_str() == "aigw-ebpf")
            .ok_or_else(|| anyhow!("aigw-ebpf package not found"))?;
        let cargo_metadata::Package {
            name,
            manifest_path,
            ..
        } = ebpf_package;
        let ebpf_package = aya_build::Package {
            name: name.as_str(),
            root_dir: manifest_path
                .parent()
                .ok_or_else(|| anyhow!("no parent for {manifest_path}"))?
                .as_str(),
            ..Default::default()
        };
        aya_build::build_ebpf([ebpf_package], Toolchain::default())
    } else {
        Ok(())
    }
}
