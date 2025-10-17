use std::{env, fs, path::Path, process::Command};

fn main() {
    if !fs::exists("target").unwrap() {
        fs::create_dir("target").unwrap();
    }

    // install packages
    match Command::new("pnpm")
        .args(&["install"])
        .current_dir("./ui")
        .status()
    {
        Ok(_) => {}
        Err(err) => {
            panic!("npm run install error. {:?}", err);
        }
    }

    // build ui
    match Command::new("pnpm")
        .args(&["run", "build"])
        .current_dir("./ui/")
        .status()
    {
        Ok(_) => {
            fs::copy(
                Path::new("./ui/apps/aigwc/dist.zip"),
                "./target/aigwc.ui.zip",
            )
            .unwrap();
        }
        Err(err) => {
            panic!("npm run build error. {:?}", err);
        }
    }

    let out_dir = env::var("OUT_DIR").unwrap();

    // Process manpage using asciidoctor command

    fs::create_dir_all(&out_dir).unwrap();
    fs::copy("aigwc.adoc", Path::new(&out_dir).join("aigwc.adoc")).unwrap();
    match Command::new("asciidoctor")
        .args(&["-b", "manpage", "aigwc.adoc"])
        .current_dir(&Path::new(&out_dir))
        .status()
    {
        Ok(_) => {
            Command::new("gzip")
                .args(&["aigwc.1"])
                .current_dir(&Path::new(&out_dir))
                .status()
                .unwrap();
            fs::copy(
                Path::new(&out_dir).join("aigwc.1.gz"),
                "./target/aigwc.1.gz",
            )
            .unwrap();
        }
        Err(err) => {
            println!("cargo:warning=Error building manpage: {}", err);
            println!("cargo:warning=The manpage will not be build. Do you have 'asciidoctor'?");
        }
    }
}
