// SPDX-License-Identifier: MPL-2.0

//! Mount options of tmpfs.
//!
//! Reference: <https://www.kernel.org/doc/html/latest/filesystems/tmpfs.html>

use super::fs::{default_max_blocks, default_max_inodes};
use crate::prelude::*;

/// Linux tmpfs options that are not implemented yet.
///
/// They are accepted and ignored so that existing mount commands keep working.
const IGNORED_OPTIONS: &[&str] = &["huge", "mpol", "noswap", "inode32", "inode64"];

/// The parsed mount options of a tmpfs instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::fs) struct TmpFsOptions {
    /// The maximum number of pages. `None` means unlimited.
    pub(in crate::fs) max_blocks: Option<usize>,
    /// The maximum number of inodes. `None` means unlimited.
    pub(in crate::fs) max_inodes: Option<usize>,
    /// The permission bits of the root directory.
    pub(in crate::fs) mode: u16,
    /// The owner of the root directory.
    pub(in crate::fs) uid: u32,
    /// The group of the root directory.
    pub(in crate::fs) gid: u32,
}

impl Default for TmpFsOptions {
    fn default() -> Self {
        Self {
            max_blocks: Some(default_max_blocks()),
            max_inodes: Some(default_max_inodes()),
            mode: 0o1777,
            uid: 0,
            gid: 0,
        }
    }
}

impl TmpFsOptions {
    /// Parses the comma-separated mount options.
    ///
    /// Later options override earlier ones, so `size` and `nr_blocks`, which
    /// set the same limit, follow the order in which they are given.
    pub(in crate::fs) fn parse(args: Option<&str>) -> Result<Self> {
        let mut options = Self::default();
        let Some(args) = args else {
            return Ok(options);
        };

        for token in args.split(',').filter(|token| !token.is_empty()) {
            let (key, value) = match token.split_once('=') {
                Some((key, value)) => (key, Some(value)),
                None => (token, None),
            };

            match (key, value) {
                ("size", Some(value)) => options.max_blocks = parse_size(value)?,
                ("nr_blocks", Some(value)) => options.max_blocks = parse_count(value)?,
                ("nr_inodes", Some(value)) => options.max_inodes = parse_count(value)?,
                ("mode", Some(value)) => options.mode = parse_mode(value)?,
                ("uid", Some(value)) => options.uid = parse_id(value)?,
                ("gid", Some(value)) => options.gid = parse_id(value)?,
                (key, _) if IGNORED_OPTIONS.contains(&key) => {
                    warn!(
                        "the tmpfs mount option `{}` is not supported and ignored",
                        key
                    );
                }
                _ => {
                    return_errno_with_message!(
                        Errno::EINVAL,
                        "unknown or invalid tmpfs mount option"
                    );
                }
            }
        }

        Ok(options)
    }
}

fn invalid_value() -> Error {
    Error::with_message(Errno::EINVAL, "invalid value of a tmpfs mount option")
}

/// Parses a number with an optional binary suffix (`k`, `m`, `g`, ...),
/// like the kernel's `memparse`.
fn parse_number(value: &str) -> Result<u64> {
    let digits_end = value
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(value.len());
    let (digits, suffix) = value.split_at(digits_end);

    let number = digits.parse::<u64>().map_err(|_| invalid_value())?;
    let shift = match suffix {
        "" => 0,
        "k" | "K" => 10,
        "m" | "M" => 20,
        "g" | "G" => 30,
        "t" | "T" => 40,
        "p" | "P" => 50,
        "e" | "E" => 60,
        _ => return Err(invalid_value()),
    };

    number.checked_mul(1u64 << shift).ok_or_else(invalid_value)
}

fn limit_from(count: u64) -> Option<usize> {
    if count == 0 {
        None
    } else {
        Some(usize::try_from(count).unwrap_or(usize::MAX))
    }
}

/// Parses `size=`, which is in bytes or a percentage of the memory, and
/// rounds it up to whole pages. Zero means unlimited.
fn parse_size(value: &str) -> Result<Option<usize>> {
    let bytes = match value.strip_suffix('%') {
        Some(percent) => {
            let percent = percent.parse::<u64>().map_err(|_| invalid_value())?;
            let bytes = crate::vm::mem_total() as u128 * percent as u128 / 100;
            u64::try_from(bytes).map_err(|_| invalid_value())?
        }
        None => parse_number(value)?,
    };

    Ok(limit_from(bytes.div_ceil(PAGE_SIZE as u64)))
}

/// Parses `nr_blocks=` and `nr_inodes=`. Zero means unlimited.
fn parse_count(value: &str) -> Result<Option<usize>> {
    Ok(limit_from(parse_number(value)?))
}

fn parse_mode(value: &str) -> Result<u16> {
    let mode = u32::from_str_radix(value, 8).map_err(|_| invalid_value())?;
    Ok((mode & 0o7777) as u16)
}

fn parse_id(value: &str) -> Result<u32> {
    value.parse::<u32>().map_err(|_| invalid_value())
}
