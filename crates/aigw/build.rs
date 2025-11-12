use std::{
    borrow::Cow,
    env,
    ffi::OsString,
    fs,
    io::{BufRead as _, BufReader},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};

use anyhow::{Context as _, Result, anyhow};
use cargo_metadata::{Artifact, CompilerMessage, Message, Target};

static VERSION_TEMPLATE: &str = r#"
pub static VERSION: &str = "{version}";
"#;

#[derive(Default)]
pub struct Package<'a> {
    pub name: &'a str,
    pub root_dir: &'a str,
    pub no_default_features: bool,
    pub features: &'a [&'a str],
}

fn target_arch_fixup(target_arch: Cow<'_, str>) -> Cow<'_, str> {
    if target_arch.starts_with("riscv64") {
        "riscv64".into()
    } else {
        target_arch
    }
}

/// Build binary artifacts produced by `packages`.
///
/// This would be better expressed as one or more [artifact-dependencies][bindeps] but issues such
/// as:
///
/// * <https://github.com/rust-lang/cargo/issues/12374>
/// * <https://github.com/rust-lang/cargo/issues/12375>
/// * <https://github.com/rust-lang/cargo/issues/12385>
///
/// prevent their use for the time being.
///
/// [bindeps]: https://doc.rust-lang.org/nightly/cargo/reference/unstable.html?highlight=feature#artifact-dependencies
pub fn build_ebpf<'a>(
    packages: impl IntoIterator<Item = Package<'a>>,
    toolchain: Toolchain<'a>,
) -> Result<()> {
    let out_dir = env::var_os("OUT_DIR").ok_or(anyhow!("OUT_DIR not set"))?;
    let out_dir = PathBuf::from(out_dir);

    let endian =
        env::var_os("CARGO_CFG_TARGET_ENDIAN").ok_or(anyhow!("CARGO_CFG_TARGET_ENDIAN not set"))?;
    let target = if endian == "big" {
        "bpfeb"
    } else if endian == "little" {
        "bpfel"
    } else {
        return Err(anyhow!("unsupported endian={endian:?}"));
    };

    const TARGET_ARCH: &str = "CARGO_CFG_TARGET_ARCH";
    let bpf_target_arch =
        env::var_os(TARGET_ARCH).unwrap_or_else(|| panic!("{TARGET_ARCH} not set"));
    let bpf_target_arch = bpf_target_arch
        .into_string()
        .unwrap_or_else(|err| panic!("OsString::into_string({TARGET_ARCH}): {err:?}"));
    let bpf_target_arch = target_arch_fixup(bpf_target_arch.into());
    let target = format!("{target}-unknown-none");

    for Package {
        name,
        root_dir,
        no_default_features,
        features,
    } in packages
    {
        // We have a build-dependency on `name`, so cargo will automatically rebuild us if `name`'s
        // *library* target or any of its dependencies change. Since we depend on `name`'s *binary*
        // targets, that only gets us half of the way. This stanza ensures cargo will rebuild us on
        // changes to the binaries too, which gets us the rest of the way.
        println!("cargo:rerun-if-changed={root_dir}");

        let mut cmd = Command::new("rustup");
        cmd.args([
            "run",
            toolchain.as_str(),
            "cargo",
            "build",
            "--package",
            name,
            "-Z",
            "build-std=core",
            "--bins",
            "--message-format=json",
            "--release",
            "--target",
            &target,
        ]);
        if no_default_features {
            cmd.arg("--no-default-features");
        }
        cmd.args(["--features", &features.join(",")]);

        {
            const SEPARATOR: &str = "\x1f";

            let mut rustflags = OsString::new();

            for s in [
                "--cfg=bpf_target_arch=\"",
                &bpf_target_arch,
                "\"",
                SEPARATOR,
                "-Cdebuginfo=2",
                SEPARATOR,
                "-Clink-arg=--btf",
            ] {
                rustflags.push(s);
            }

            cmd.env("CARGO_ENCODED_RUSTFLAGS", rustflags);
        }

        // Workaround to make sure that the correct toolchain is used.
        for key in ["RUSTC", "RUSTC_WORKSPACE_WRAPPER"] {
            cmd.env_remove(key);
        }

        // Workaround for https://github.com/rust-lang/cargo/issues/6412 where cargo flocks itself.
        let target_dir = out_dir.join(name);
        cmd.arg("--target-dir").arg(&target_dir);

        let mut child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("failed to spawn {cmd:?}"))?;
        let Child { stdout, stderr, .. } = &mut child;

        // Trampoline stdout to cargo warnings.
        let stderr = stderr.take().expect("stderr");
        let stderr = BufReader::new(stderr);
        let stderr = std::thread::spawn(move || {
            for line in stderr.lines() {
                let _line = line.expect("read line");
                //println!("cargo:warning={line}");
            }
        });

        let stdout = stdout.take().expect("stdout");
        let stdout = BufReader::new(stdout);
        let mut executables = Vec::new();
        for message in Message::parse_stream(stdout) {
            #[expect(clippy::collapsible_match)]
            match message.expect("valid JSON") {
                Message::CompilerArtifact(Artifact {
                    executable,
                    target: Target { name, .. },
                    ..
                }) => {
                    if let Some(executable) = executable {
                        executables.push((name, executable.into_std_path_buf()));
                    }
                }
                Message::CompilerMessage(CompilerMessage { message, .. }) => {
                    for _line in message.rendered.unwrap_or_default().split('\n') {
                        //println!("cargo:warning={line}");
                    }
                }
                Message::TextLine(_line) => {
                    //println!("cargo:warning={line}");
                }
                _ => {}
            }
        }

        let status = child
            .wait()
            .with_context(|| format!("failed to wait for {cmd:?}"))?;
        if !status.success() {
            return Err(anyhow!("{cmd:?} failed: {status:?}"));
        }

        match stderr.join().map_err(std::panic::resume_unwind) {
            Ok(()) => {}
            Err(err) => match err {},
        }

        for (name, binary) in executables {
            let dst = PathBuf::from(env::var_os("CARGO_TARGET_DIR").unwrap_or("target".into()))
                .join(name.clone() + ".ebpf");
            let _: u64 = fs::copy(&binary, &dst)
                .with_context(|| format!("failed to copy {binary:?} to {dst:?}"))?;

            let dst = out_dir.join(&name);

            let _: u64 = fs::copy(&binary, &dst)
                .with_context(|| format!("failed to copy {binary:?} to {dst:?}"))?;
        }
    }
    Ok(())
}

/// The toolchain to use for building eBPF programs.
#[derive(Default)]
pub enum Toolchain<'a> {
    /// The latest nightly toolchain i.e. `nightly`.
    #[default]
    Nightly,
    /// A custom toolchain e.g. `nightly-2021-01-01`.
    ///
    /// The toolchain specifier is passed to `rustup run` and therefore should _not_ have a preceding `+`.
    Custom(&'a str),
}

impl<'a> Toolchain<'a> {
    fn as_str(&self) -> &'a str {
        match self {
            Toolchain::Nightly => "nightly",
            Toolchain::Custom(toolchain) => toolchain,
        }
    }
}

/// Emit cfg flags that describe the desired BPF target architecture.
pub fn emit_bpf_target_arch_cfg() {
    // The presence of this environment variable indicates that `--cfg
    // bpf_target_arch="..."` was passed to the compiler, so we don't need to
    // emit it again. Note that we cannot *set* this environment variable - it
    // is set by cargo.
    const BPF_TARGET_ARCH: &str = "CARGO_CFG_BPF_TARGET_ARCH";
    println!("cargo:rerun-if-env-changed={BPF_TARGET_ARCH}");

    // Users may directly set this environment variable in situations where
    // using RUSTFLAGS to set `--cfg bpf_target_arch="..."` is not possible or
    // not ergonomic. In contrast to RUSTFLAGS this mechanism reuses the target
    // cache for all values, producing many more invalidations.
    const AYA_BPF_TARGET_ARCH: &str = "AYA_BPF_TARGET_ARCH";
    println!("cargo:rerun-if-env-changed={AYA_BPF_TARGET_ARCH}");

    const HOST: &str = "HOST";
    println!("cargo:rerun-if-env-changed={HOST}");

    if std::env::var_os(BPF_TARGET_ARCH).is_none() {
        let host = std::env::var_os(HOST).unwrap_or_else(|| panic!("{HOST} not set"));
        let host = host
            .into_string()
            .unwrap_or_else(|err| panic!("OsString::into_string({HOST}): {err:?}"));
        let host = host.as_str();

        let bpf_target_arch = if let Some(bpf_target_arch) = std::env::var_os(AYA_BPF_TARGET_ARCH) {
            bpf_target_arch
                .into_string()
                .unwrap_or_else(|err| {
                    panic!("OsString::into_string({AYA_BPF_TARGET_ARCH}): {err:?}")
                })
                .into()
        } else {
            target_arch_fixup(
                host.split_once('-')
                    .map_or(host, |(arch, _rest)| arch)
                    .into(),
            )
        };
        println!("cargo:rustc-cfg=bpf_target_arch=\"{bpf_target_arch}\"");
    }

    print!("cargo::rustc-check-cfg=cfg(bpf_target_arch, values(");
    for value in [
        "aarch64",
        "arm",
        "loongarch64",
        "mips",
        "powerpc64",
        "riscv64",
        "s390x",
        "x86_64",
    ] {
        print!("\"{value}\",");
    }
    println!("))");
}

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
                .args(&["-f", "aigw.1"])
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
    let ebpf_package = Package {
        name: name.as_str(),
        root_dir: manifest_path
            .parent()
            .ok_or_else(|| anyhow!("no parent for {manifest_path}"))?
            .as_str(),
        ..Default::default()
    };
    build_ebpf([ebpf_package], Toolchain::default())
}
