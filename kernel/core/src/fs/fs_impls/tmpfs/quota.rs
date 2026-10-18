// SPDX-License-Identifier: MPL-2.0

//! Resource accounting of tmpfs.

use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{prelude::*, vm::page_cache::PageCharger};

/// Tracks the pages and inodes used by a tmpfs instance against its limits.
pub(in crate::fs) struct TmpFsQuota {
    /// The maximum number of pages. `None` means unlimited.
    max_blocks: Option<usize>,
    used_blocks: AtomicUsize,
    /// The maximum number of inodes. `None` means unlimited.
    max_inodes: Option<usize>,
    used_inodes: AtomicUsize,
}

impl TmpFsQuota {
    pub(in crate::fs) fn new(max_blocks: Option<usize>, max_inodes: Option<usize>) -> Self {
        Self {
            max_blocks,
            used_blocks: AtomicUsize::new(0),
            max_inodes,
            // The root inode is counted from the beginning, as Linux does.
            used_inodes: AtomicUsize::new(1),
        }
    }

    /// Returns the number of pages that are currently charged.
    pub(in crate::fs) fn used_blocks(&self) -> usize {
        self.used_blocks.load(Ordering::Relaxed)
    }

    /// Returns the number of inodes that are currently charged.
    pub(in crate::fs) fn used_inodes(&self) -> usize {
        self.used_inodes.load(Ordering::Relaxed)
    }

    /// Charges one inode, failing with `ENOSPC` if the limit is exceeded.
    pub(in crate::fs) fn try_charge_inode(&self) -> Result<()> {
        try_charge(&self.used_inodes, self.max_inodes, 1)
    }

    pub(in crate::fs) fn uncharge_inode(&self) {
        self.used_inodes.fetch_sub(1, Ordering::Relaxed);
    }
}

impl PageCharger for TmpFsQuota {
    fn try_charge(&self, nr_pages: usize) -> Result<()> {
        try_charge(&self.used_blocks, self.max_blocks, nr_pages)
    }

    fn uncharge(&self, nr_pages: usize) {
        self.used_blocks.fetch_sub(nr_pages, Ordering::Relaxed);
    }
}

fn try_charge(used: &AtomicUsize, max: Option<usize>, nr: usize) -> Result<()> {
    let Some(max) = max else {
        used.fetch_add(nr, Ordering::Relaxed);
        return Ok(());
    };

    used.try_update(Ordering::AcqRel, Ordering::Acquire, |used| {
        used.checked_add(nr).filter(|new| *new <= max)
    })
    .map(|_| ())
    .map_err(|_| Error::with_message(Errno::ENOSPC, "the tmpfs has no space left"))
}
