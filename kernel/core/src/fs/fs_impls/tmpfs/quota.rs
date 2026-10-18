// SPDX-License-Identifier: MPL-2.0

//! Resource accounting of tmpfs.

use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{prelude::*, vm::page_cache::PageCharger};

/// Tracks the pages used by a tmpfs instance against its limit.
pub(in crate::fs) struct TmpFsQuota {
    /// The maximum number of pages. `None` means unlimited.
    max_blocks: Option<usize>,
    used_blocks: AtomicUsize,
}

impl TmpFsQuota {
    pub(in crate::fs) fn new(max_blocks: Option<usize>) -> Self {
        Self {
            max_blocks,
            used_blocks: AtomicUsize::new(0),
        }
    }

    /// Returns the number of pages that are currently charged.
    pub(in crate::fs) fn used_blocks(&self) -> usize {
        self.used_blocks.load(Ordering::Relaxed)
    }
}

impl PageCharger for TmpFsQuota {
    fn try_charge(&self, nr_pages: usize) -> Result<()> {
        let Some(max_blocks) = self.max_blocks else {
            self.used_blocks.fetch_add(nr_pages, Ordering::Relaxed);
            return Ok(());
        };

        self.used_blocks
            .try_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                used.checked_add(nr_pages).filter(|new| *new <= max_blocks)
            })
            .map(|_| ())
            .map_err(|_| Error::with_message(Errno::ENOSPC, "the tmpfs has no space left"))
    }

    fn uncharge(&self, nr_pages: usize) {
        self.used_blocks.fetch_sub(nr_pages, Ordering::Relaxed);
    }
}
