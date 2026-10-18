// SPDX-License-Identifier: MPL-2.0

//! Temporary file system (tmpfs) based on ramfs.

pub(super) use fs::TmpFs;
use fs::TmpFsType;
pub(super) use options::TmpFsOptions;

mod fs;
mod options;

pub(super) const TMPFS_MAGIC: u64 = 0x0102_1994;

pub(super) fn init() {
    crate::fs::vfs::registry::register(&TmpFsType).unwrap();
}
