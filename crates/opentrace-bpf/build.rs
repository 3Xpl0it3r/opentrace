use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::{env, fs, io};

use libbpf_cargo::SkeletonBuilder;

fn clang_include_args(path: impl AsRef<Path>) -> [OsString; 2] {
    [
        OsStr::new("-I").to_owned(),
        path.as_ref().as_os_str().to_owned(),
    ]
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

fn main() {
    let manifest_dir = PathBuf::from(
        env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set in build script"),
    );
    let bpf_dir = manifest_dir.join("src").join("bpf");
    let bpf_sources = list_bpf_sources(&bpf_dir).expect("failed to discover .bpf.c sources");

    let arch = env::var_os("CARGO_CFG_TARGET_ARCH")
        .expect("CARGO_CFG_TARGET_ARCH must be set in build script");

    let mut include_dirs: Vec<OsString> = vec!["src/bpf/".into()];

    // 配置了OPENTRACE_BPF_INCLUDE意味着当前操作系统不支持btf
    // 在Makefile里面做了判断，当操作系统不支持btf的时候会在cargobuild只是添加环境变量来指定vmlinux,h
    // 如果支持则使用vmlinux 库
    // 当然在bpf目录下仍然保留了vmlinux目录是为了编码防止lsp报错
    if let Some(extra) = env::var_os("OPENTRACE_BPF_INCLUDE") {
        for part in env::split_paths(&extra) {
            if !part.as_os_str().is_empty() {
                include_dirs.push(part.into_os_string());
            }
        }
    } else {
        include_dirs.push(vmlinux::include_path_root().join(&arch).into());
    }

    let mut clang_args: Vec<OsString> = include_dirs
        .into_iter()
        .flat_map(clang_include_args)
        .collect();
    clang_args.push(OsStr::new("-DLINUX_KERNEL_CODE=50400").to_owned());

    for source in &bpf_sources {
        let out = skeleton_output_path_from_source(source)
            .expect("failed to resolve output skeleton path from source");
        generate_skeleton(source, &out, &clang_args);
    }

    write_bpf_mod_rs();

    let known_cfgs = ["kernel_gt_4_19", "kernel_le_4_19"];

    for cfg in known_cfgs {
        println!("cargo::rustc-check-cfg=cfg({cfg})");
    }
    println!("cargo:rerun-if-changed={}", bpf_dir.display());
    for source in &bpf_sources {
        println!("cargo:rerun-if-changed={}", source.display());
    }
    println!("cargo:rerun-if-env-changed=OPENTRACE_BPF_INCLUDE");
}
