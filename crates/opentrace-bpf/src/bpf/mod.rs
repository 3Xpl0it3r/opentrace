pub mod perf_profile {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/bpf/perf_profile.skel.rs"
    ));
}

pub mod skbdrop {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/bpf/skbdrop.skel.rs"
    ));
}

pub mod socket_tcp {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/bpf/socket_tcp.skel.rs"
    ));
}

