use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{env, fs, io};

use libbpf_cargo::SkeletonBuilder;

const BTF_VMLINUX_PATH: &str = "/sys/kernel/btf/vmlinux";

#[derive(Debug)]
struct HostOsInfo {
    id: String,
    version_id: String,
    arch: OsString,
}

fn has_kernel_btf_support() -> bool {
    Path::new(BTF_VMLINUX_PATH).is_file()
}

fn read_kernel_version() -> Option<(u32, u32, u32)> {
    let release = fs::read_to_string("/proc/sys/kernel/osrelease").ok()?;
    let version = release.split('-').next()?.trim();
    let mut parts = version.split('.').map(|s| s.parse::<u32>().ok());

    let major = parts.next()??;
    let minor = parts.next()??;
    let patch = parts.next().flatten().unwrap_or(0);

    Some((major, minor, patch))
}

fn clang_include_args(path: impl AsRef<Path>) -> [OsString; 2] {
    [
        OsStr::new("-I").to_owned(),
        path.as_ref().as_os_str().to_owned(),
    ]
}

fn ensure_command_available(command: &str, probe_args: &[&str]) -> io::Result<()> {
    match Command::new(command)
        .args(probe_args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        Ok(_) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("`{command}` not found in PATH; please install `{command}`"),
        )),
        Err(err) => Err(io::Error::new(
            err.kind(),
            format!("failed to execute `{command}` probe command: {err}"),
        )),
    }
}

fn list_bpf_sources(bpf_dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut sources = Vec::new();
    for entry in fs::read_dir(bpf_dir)? {
        let entry = entry?;
        let path = entry.path();
        let is_bpf_source = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.ends_with(".bpf.c"))
            .unwrap_or(false);

        if path.is_file() && is_bpf_source {
            sources.push(path);
        }
    }

    sources.sort();

    if sources.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("no *.bpf.c files found under {}", bpf_dir.display()),
        ));
    }

    Ok(sources)
}

fn skeleton_output_path_from_source(source: &Path) -> io::Result<PathBuf> {
    let file_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid source file name: {}", source.display()),
            )
        })?;

    let base_name = file_name.strip_suffix(".bpf.c").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("source is not a .bpf.c file: {}", source.display()),
        )
    })?;

    Ok(source.with_file_name(format!("{base_name}.skel.rs")))
}

fn generate_skeleton(source: &Path, out: &Path, clang_args: &[OsString]) {
    let mut builder = SkeletonBuilder::new();
    builder.source(source);
    builder
        .clang_args(clang_args.iter().cloned())
        .build_and_generate(out)
        .unwrap();
}

// 遍历src/bpf/ 目录下所有的 *.skel.rs  文件, 针对每一个*.skel.rs文件 在src/bpf/mod.rs 里面生成 如下内容, 例如XXX.skel.rs 生成内容如下
//
// pub mod XXX {
//   include!(concat!(
//       env!("CARGO_MANIFEST_DIR"),
//       "/src/bpf/XXX.skel.rs"
//       ));
//   }
//
//  为了简化这部分操作，每次在build的时候先清理src/mod.rs 里面所有的内容，重新生成内容
fn write_bpf_mod_rs() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let bpf_dir = manifest_dir.join("src").join("bpf");
    let mod_path = bpf_dir.join("mod.rs");

    let mut skeletons: Vec<String> = fs::read_dir(&bpf_dir)
        .unwrap()
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_file())
        .filter_map(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| name.strip_suffix(".skel.rs"))
                .map(str::to_string)
        })
        .collect();

    skeletons.sort();

    let mut content = String::new();
    for name in skeletons {
        content.push_str(&format!(
            "pub mod {name} {{\n    include!(concat!(\n        env!(\"CARGO_MANIFEST_DIR\"),\n        \"/src/bpf/{name}.skel.rs\"\n    ));\n}}\n\n"
        ));
    }

    fs::write(mod_path, content).unwrap();
}

fn system_info() -> io::Result<HostOsInfo> {
    let content = fs::read_to_string("/etc/os-release")?;
    let mut id: Option<String> = None;
    let mut version_id: Option<String> = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, raw_value)) = line.split_once('=') else {
            continue;
        };
        let value = raw_value.trim().trim_matches('"').to_string();
        match key {
            "ID" => id = Some(value),
            "VERSION_ID" => version_id = Some(value),
            _ => {}
        }
    }

    let arch = env::var_os("CARGO_CFG_TARGET_ARCH")
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "CARGO_CFG_TARGET_ARCH not set"))?;

    Ok(HostOsInfo {
        id: id.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "ID not found"))?,
        version_id: version_id
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "VERSION_ID not found"))?,
        arch,
    })
}

fn generate_vmlinux_header_from_btf(os_info: &HostOsInfo) -> io::Result<()> {
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let btf_path = manifest_path
        .join("src/bpf/btfhub_archive")
        .join(&os_info.id)
        .join(&os_info.version_id)
        .join(&os_info.arch)
        .join("vmlinux.btf");
    let out_path = manifest_path.join("src/bpf/vmlinux/vmlinux.h");

    ensure_command_available("bpftool", &["--version"])?;

    let out_file = fs::File::create(&out_path)?;
    let status = Command::new("bpftool")
        .args(["btf", "dump", "file"])
        .arg(&btf_path)
        .args(["format", "c"])
        .stdout(Stdio::from(out_file))
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            format!("bpftool failed with status: {status}"),
        ))
    }
}

fn ensure_vmlinux_header_generated() -> io::Result<()> {
    let os_info = system_info()
        .map_err(|err| io::Error::new(err.kind(), format!("failed to read system info: {err}")))?;

    generate_vmlinux_header_from_btf(&os_info)
        .map_err(|err| io::Error::new(err.kind(), format!("failed to dump vmlinux.h: {err}")))
}

fn main() {
    let manifest_dir = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set in build script"),
    );
    let bpf_dir = manifest_dir.join("src").join("bpf");
    let bpf_sources = list_bpf_sources(&bpf_dir).expect("failed to discover .bpf.c sources");

    let arch = env::var_os("CARGO_CFG_TARGET_ARCH")
        .expect("CARGO_CFG_TARGET_ARCH must be set in build script");

    let (major_version, minor_version, _patch_version) =
        read_kernel_version().expect("cannot get kernel version");

    let mut include_dirs: Vec<OsString> = vec!["src/bpf/".into(), "src/bpf/vmlinux/".into()];

    match has_kernel_btf_support() {
        true => include_dirs.push(vmlinux::include_path_root().join(&arch).into()),
        false => {
            ensure_vmlinux_header_generated().expect("failed to generate vmlinux");
            include_dirs.push("src/bpf/vmlinux".into());
        }
    }

    let mut clang_args: Vec<OsString> = include_dirs
        .into_iter()
        .flat_map(clang_include_args)
        .collect();
    clang_args.push(OsStr::new("-DLINUX_KERNEL_VERSION=50400").to_owned());

    for source in &bpf_sources {
        let out = skeleton_output_path_from_source(source)
            .expect("failed to resolve output skeleton path from source");
        generate_skeleton(source, &out, &clang_args);
    }

    write_bpf_mod_rs();

    let kernel_version_flag = if major_version < 4 || (major_version == 4 && minor_version <= 19) {
        "kernel_le_4_19"
    } else {
        "kernel_gt_4_19"
    };
    let known_cfgs = ["kernel_gt_4_19", "kernel_le_4_19"];

    for cfg in known_cfgs {
        println!("cargo::rustc-check-cfg=cfg({cfg})");
    }
    println!("cargo:rerun-if-changed={}", bpf_dir.display());
    for source in &bpf_sources {
        println!("cargo:rerun-if-changed={}", source.display());
    }
    println!("cargo:rustc-cfg={kernel_version_flag}");
}
