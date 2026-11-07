use redox_scheme::scheme::SchemeSync;
use redox_scheme::{CallerCtx, OpenResult};
use scheme_utils::FpathWriter;
use syscall::data::Stat;
use syscall::error::{Error, Result, EACCES, EBADF, ENOENT};
use syscall::flag::{MODE_FILE, O_DIRECTORY};
use syscall::schemev2::NewFdFlags;

const ROOT_ID: usize = 1;
const CAPABILITIES_ID: usize = 2;
const RUNTIME_POLICY_ID: usize = 3;
const HARDWARE_EVIDENCE_ID: usize = 4;

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
        if dirfd != ROOT_ID || flags & O_DIRECTORY != 0 {
            return Err(Error::new(EACCES));
        }

        let id = match path.trim_matches('/') {
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
        self.content(id)?;
        Ok(0)
    }

    fn fsize(&mut self, id: usize, _ctx: &CallerCtx) -> Result<u64> {
        Ok(self.content(id)?.len() as u64)
    }

    fn fpath(&mut self, id: usize, buf: &mut [u8], _ctx: &CallerCtx) -> Result<usize> {
        let path = match id {
            CAPABILITIES_ID => "trion:capabilities",
            RUNTIME_POLICY_ID => "trion:runtime-policy",
            HARDWARE_EVIDENCE_ID => "trion:hardware-evidence",
            _ => return Err(Error::new(EBADF)),
        };
        FpathWriter::with(buf, path, |_| Ok(()))
    }

    fn fstat(&mut self, id: usize, stat: &mut Stat, _ctx: &CallerCtx) -> Result<()> {
        let content = self.content(id)?;
        stat.st_mode = MODE_FILE | 0o444;
        stat.st_size = content.len() as u64;
        stat.st_blocks = content.len().div_ceil(512) as u64;
        stat.st_blksize = 4096;
        stat.st_nlink = 1;
        Ok(())
    }

    fn fsync(&mut self, id: usize, _ctx: &CallerCtx) -> Result<()> {
        self.content(id)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_handles_are_read_only_and_offset_aware() {
        let mut scheme = CapabilityScheme::new(
            "capability-report".into(),
            "runtime-policy".into(),
            "hardware-evidence".into(),
        );
        let mut output = [0u8; 8];
        let caller = CallerCtx {
            pid: 1,
            uid: 0,
            gid: 0,
        };
        let read = scheme.read(CAPABILITIES_ID, &mut output, 3, 0, &caller).unwrap();
        assert_eq!(&output[..read], b"abilit");
        assert_eq!(scheme.fsize(CAPABILITIES_ID, &caller).unwrap(), 17);
        assert_eq!(
            scheme.write(CAPABILITIES_ID, b"overwrite", 0, 0, &caller)
                .unwrap_err().errno,
            EACCES
        );
    }

    #[test]
    fn unknown_handles_are_rejected() {
        let mut scheme = CapabilityScheme::new(String::new(), String::new(), String::new());
        let caller = CallerCtx {
            pid: 1,
            uid: 0,
            gid: 0,
        };
        assert_eq!(
            scheme.read(99, &mut [0; 1], 0, 0, &caller).unwrap_err().errno,
            EBADF
        );
    }
}
