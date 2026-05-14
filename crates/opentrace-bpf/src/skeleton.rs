// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
use std::mem::MaybeUninit;

// Copyright 2026 opentrace Project Authors. Licensed under Apache-2.0.
use libbpf_rs::OpenObject;

pub type CollectorObject = MaybeUninit<OpenObject>;

pub fn open_object_storage() -> CollectorObject {
    MaybeUninit::uninit()
}
