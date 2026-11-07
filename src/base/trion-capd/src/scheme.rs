use redox_scheme::scheme::SchemeSync;
use redox_scheme::{CallerCtx, OpenResult};
use scheme_utils::FpathWriter;
use syscall::data::Stat;
use syscall::dirent::{DirEntry, DirentBuf, DirentIter, DirentKind};
use syscall::error::{Error, Result, EACCES, EBADF, EISDIR, ENOENT, ENOTDIR};
use syscall::flag::{MODE_DIR, MODE_FILE, O_DIRECTORY, O_RDWR, O_WRONLY};
use syscall::schemev2::NewFdFlags;

const ROOT_ID: usize = 1;
const CAPABILITIES_ID: usize = 2;
const RUNTIME_POLICY_ID: usize = 3;
const HARDWARE_EVIDENCE_ID: usize = 4;
const FILE_NAMES: &[&str] = &["capabilities", "runtime-policy", "hardware-evidence"];

pub struct CapabilityScheme {
    capabilities: Vec<u8>,
    runtime_policy: Vec<u8>,
    hardware_evidence: Vec<u8>,
}

impl CapabilityScheme {
    pub fn new(
        capabilities: String,
        runtime_policy: String,
        hardware_evidence: String,
    ) -> Self {
        Self {
            capabilities: capabilities.into_bytes(),
            runtime_policy: runtime_policy.into_bytes(),
            hardware_evidence: hardware_evidence.into_bytes(),
        }
    }

    fn content(&self, id: usize) -> Result<&[u8]> {
        match id {
            CAPABILITIES_ID => Ok(&self.capabilities),
            RUNTIME_POLICY_ID => Ok(&self.runtime_policy),
            HARDWARE_EVIDENCE_ID => Ok(&self.hardware_evidence),
            _ => Err(Error::new(EBADF)),
        }
    }
}

impl SchemeSync for CapabilityScheme {
    fn scheme_root(&mut self) -> Result<usize> {
        Ok(ROOT_ID)
    }

    fn openat(
        &mut self,
        dirfd: usize,
        path: &str,
        flags: usize,
        _fcntl_flags: u32,
        _ctx: &CallerCtx,
    ) -> Result<OpenResult> {
        if dirfd != ROOT_ID {
            return Err(Error::new(EACCES));
        }

        let path = path.trim_matches('/');
        if path.is_empty() {
            return if flags & O_DIRECTORY != 0 {
                Ok(OpenResult::ThisScheme {
                    number: ROOT_ID,
                    flags: NewFdFlags::POSITIONED,
                })
            } else {
                Err(Error::new(EISDIR))
            };
        }

        if flags & O_DIRECTORY != 0 {
            return Err(Error::new(ENOTDIR));
        }
        if flags & (O_WRONLY | O_RDWR) != 0 {
            return Err(Error::new(EACCES));
        }

        let id = match path {
            "capabilities" => CAPABILITIES_ID,
            "runtime-policy" => RUNTIME_POLICY_ID,
            "hardware-evidence" => HARDWARE_EVIDENCE_ID,
            _ => return Err(Error::new(ENOENT)),
        };

        Ok(OpenResult::ThisScheme {
            number: id,
            flags: NewFdFlags::POSITIONED,
        })
    }

    fn read(
        &mut self,
        id: usize,
        buf: &mut [u8],
        offset: u64,
        _flags: u32,
        _ctx: &CallerCtx,
    ) -> Result<usize> {
        if id == ROOT_ID {
            return Err(Error::new(EISDIR));
        }
        let content = self.content(id)?;
        let Ok(start) = usize::try_from(offset) else {
            return Ok(0);
        };
        if start >= content.len() {
            return Ok(0);
        }
        let count = buf.len().min(content.len() - start);
        buf[..count].copy_from_slice(&content[start..start + count]);
        Ok(count)
    }

    fn write(
        &mut self,
        _id: usize,
        _buf: &[u8],
        _offset: u64,
        _flags: u32,
        _ctx: &CallerCtx,
    ) -> Result<usize> {
        Err(Error::new(EACCES))
    }

    fn fcntl(&mut self, id: usize, _cmd: usize, _arg: usize, _ctx: &CallerCtx) -> Result<usize> {
        if id == ROOT_ID {
            return Ok(0);
        }
        self.content(id)?;
        Ok(0)
    }

    fn fsize(&mut self, id: usize, _ctx: &CallerCtx) -> Result<u64> {
        if id == ROOT_ID {
            return Ok(FILE_NAMES.len() as u64);
        }
        Ok(self.content(id)?.len() as u64)
    }

    fn getdents<'buf>(
        &mut self,
        id: usize,
        mut buf: DirentBuf<&'buf mut [u8]>,
        opaque_offset: u64,
    ) -> Result<DirentBuf<&'buf mut [u8]>> {
        if id != ROOT_ID {
            return Err(Error::new(ENOTDIR));
        }
        let Ok(offset) = usize::try_from(opaque_offset) else {
            return Ok(buf);
        };

        for (index, name) in FILE_NAMES.iter().enumerate().skip(offset) {
            buf.entry(DirEntry {
                inode: index as u64,
                name,
                kind: DirentKind::Unspecified,
                next_opaque_id: index as u64 + 1,
            })?;
        }
        Ok(buf)
    }

    fn fpath(&mut self, id: usize, buf: &mut [u8], _ctx: &CallerCtx) -> Result<usize> {
        let path = match id {
            ROOT_ID => "trion:",
            CAPABILITIES_ID => "trion:capabilities",
            RUNTIME_POLICY_ID => "trion:runtime-policy",
            HARDWARE_EVIDENCE_ID => "trion:hardware-evidence",
            _ => return Err(Error::new(EBADF)),
        };
        FpathWriter::with(buf, path, |_| Ok(()))
    }

    fn fstat(&mut self, id: usize, stat: &mut Stat, _ctx: &CallerCtx) -> Result<()> {
        if id == ROOT_ID {
            stat.st_mode = MODE_DIR | 0o555;
            stat.st_size = FILE_NAMES.len() as u64;
            stat.st_blksize = 4096;
            stat.st_blocks = 0;
            stat.st_nlink = 2;
            return Ok(());
        }

        let content = self.content(id)?;
        stat.st_mode = MODE_FILE | 0o444;
        stat.st_size = content.len() as u64;
        stat.st_blocks = content.len().div_ceil(512) as u64;
        stat.st_blksize = 4096;
        stat.st_nlink = 1;
        Ok(())
    }

    fn fsync(&mut self, id: usize, _ctx: &CallerCtx) -> Result<()> {
        if id == ROOT_ID {
            return Ok(());
        }
        self.content(id)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caller() -> CallerCtx {
        CallerCtx {
            pid: 1,
            uid: 0,
            gid: 0,
        }
    }

    #[test]
    fn report_handles_are_read_only_and_offset_aware() {
        let mut scheme = CapabilityScheme::new(
            "capability-report".into(),
            "runtime-policy".into(),
            "hardware-evidence".into(),
        );
        let mut output = [0u8; 8];
        let ctx = caller();
        let read = scheme
            .read(CAPABILITIES_ID, &mut output, 3, 0, &ctx)
            .unwrap();
        assert_eq!(&output[..read], b"ability-");
        assert_eq!(scheme.fsize(CAPABILITIES_ID, &ctx).unwrap(), 17);
        assert_eq!(
            scheme
                .write(CAPABILITIES_ID, b"overwrite", 0, 0, &ctx)
                .unwrap_err()
                .errno,
            EACCES
        );
    }

    #[test]
    fn directory_lists_only_the_three_read_only_reports() {
        let mut scheme = CapabilityScheme::new(String::new(), String::new(), String::new());
        let mut bytes = [0u8; 256];
        let header_size = std::mem::size_of::<syscall::dirent::DirentHeader>() as u16;
        let buffer = DirentBuf::new(&mut bytes, header_size).unwrap();
        let entries = scheme.getdents(ROOT_ID, buffer, 0).unwrap();
        let written = entries.finalize();
        let entry_count = DirentIter::new(&bytes[..written])
            .map(|entry| entry.unwrap())
            .count();
        assert_eq!(entry_count, FILE_NAMES.len());
    }

    #[test]
    fn open_rejects_write_access_and_unknown_paths() {
        let mut scheme = CapabilityScheme::new(String::new(), String::new(), String::new());
        let ctx = caller();
        assert!(matches!(
            scheme.openat(ROOT_ID, "capabilities", O_WRONLY, 0, &ctx),
            Err(error) if error.errno == EACCES
        ));
        assert!(matches!(
            scheme.openat(ROOT_ID, "missing", 0, 0, &ctx),
            Err(error) if error.errno == ENOENT
        ));
    }

    #[test]
    fn unknown_handles_are_rejected() {
        let mut scheme = CapabilityScheme::new(String::new(), String::new(), String::new());
        assert_eq!(
            scheme
                .read(99, &mut [0; 1], 0, 0, &caller())
                .unwrap_err()
                .errno,
            EBADF
        );
    }
}
