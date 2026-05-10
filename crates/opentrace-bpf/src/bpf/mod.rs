pub mod perf {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/bpf/perf.skel.rs"
    ));
}

pub mod skbdrop {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/bpf/skbdrop.skel.rs"
    ));
}

