//! Safe guest-process model primitives. This module does not execute host instructions.
//! Native execution requires a separately reviewed architecture-specific context switch.

use crate::Error;
use std::collections::BTreeMap;

pub const PAGE_SIZE: u64 = 4096;
pub const MAX_GUEST_MEMORY: u64 = 512 * 1024 * 1024;
pub const MAX_MAPPINGS: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Permissions { pub read: bool, pub write: bool, pub execute: bool }
impl Permissions {
    pub const READ: Self = Self { read: true, write: false, execute: false };
    pub const READ_WRITE: Self = Self { read: true, write: true, execute: false };
    pub const READ_EXECUTE: Self = Self { read: true, write: false, execute: true };
    pub fn validate(self) -> Result<Self, Error> {
        if self.write && self.execute { return Err(Error::Invalid("W^X violation")); }
        if !self.read && (self.write || self.execute) { return Err(Error::Invalid("mapping must be readable")); }
        Ok(self)
    }
}

#[derive(Clone, Debug)]
pub struct Mapping { pub base: u64, pub permissions: Permissions, bytes: Vec<u8> }
impl Mapping {
    pub fn len(&self) -> u64 { self.bytes.len() as u64 }
    pub fn is_empty(&self) -> bool { self.bytes.is_empty() }
    fn end(&self) -> Result<u64, Error> { self.base.checked_add(self.len()).ok_or(Error::Overflow) }
}

#[derive(Clone, Debug, Default)]
pub struct AddressSpace { mappings: BTreeMap<u64, Mapping>, committed: u64 }
impl AddressSpace {
    pub fn new() -> Self { Self::default() }
    pub fn committed_bytes(&self) -> u64 { self.committed }
    pub fn mappings(&self) -> impl Iterator<Item=&Mapping> { self.mappings.values() }

    pub fn map(&mut self, base: u64, size: u64, permissions: Permissions) -> Result<(), Error> {
        permissions.validate()?;
        if size == 0 || base % PAGE_SIZE != 0 || size % PAGE_SIZE != 0 {
            return Err(Error::Invalid("mapping must be nonzero and page aligned"));
        }
        let end = base.checked_add(size).ok_or(Error::Overflow)?;
        let next = self.committed.checked_add(size).ok_or(Error::Overflow)?;
        if next > MAX_GUEST_MEMORY { return Err(Error::Limit("guest address-space quota")); }
        if self.mappings.len() >= MAX_MAPPINGS { return Err(Error::Limit("guest mapping count")); }
        if let Some((_, prev)) = self.mappings.range(..=base).next_back() {
            if prev.end()? > base { return Err(Error::Invalid("overlapping guest mapping")); }
        }
        if let Some((&next_base, _)) = self.mappings.range(base..).next() {
            if next_base < end { return Err(Error::Invalid("overlapping guest mapping")); }
        }
        let len = usize::try_from(size).map_err(|_| Error::Overflow)?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(len).map_err(|_| Error::Limit("guest mapping allocation"))?;
        bytes.resize(len, 0);
        self.mappings.insert(base, Mapping { base, permissions, bytes });
        self.committed = next;
        Ok(())
    }

    pub fn write(&mut self, address: u64, input: &[u8]) -> Result<(), Error> {
        let (m, off) = self.locate_mut(address, input.len() as u64)?;
        if !m.permissions.write { return Err(Error::Invalid("guest mapping is not writable")); }
        let end = off.checked_add(input.len()).ok_or(Error::Overflow)?;
        m.bytes.get_mut(off..end).ok_or(Error::Invalid("guest write range"))?.copy_from_slice(input);
        Ok(())
    }

    pub fn read(&self, address: u64, output: &mut [u8]) -> Result<(), Error> {
        let m = self.find(address, output.len() as u64)?;
        if !m.permissions.read { return Err(Error::Invalid("guest mapping is not readable")); }
        let off = usize::try_from(address - m.base).map_err(|_| Error::Overflow)?;
        let end = off.checked_add(output.len()).ok_or(Error::Overflow)?;
        output.copy_from_slice(m.bytes.get(off..end).ok_or(Error::Invalid("guest read range"))?);
        Ok(())
    }

    pub fn protect(&mut self, base: u64, permissions: Permissions) -> Result<(), Error> {
        permissions.validate()?;
        let m = self.mappings.get_mut(&base).ok_or(Error::Invalid("mapping base not found"))?;
        m.permissions = permissions;
        Ok(())
    }

    pub fn unmap(&mut self, base: u64) -> Result<(), Error> {
        let m = self.mappings.remove(&base).ok_or(Error::Invalid("mapping base not found"))?;
        self.committed = self.committed.checked_sub(m.len()).ok_or(Error::Overflow)?;
        Ok(())
    }

    fn find(&self, address: u64, len: u64) -> Result<&Mapping, Error> {
        let (_, m) = self.mappings.range(..=address).next_back().ok_or(Error::Invalid("unmapped guest address"))?;
        let end = address.checked_add(len).ok_or(Error::Overflow)?;
        if address < m.base || end > m.end()? { return Err(Error::Invalid("guest access crosses mapping boundary")); }
        Ok(m)
    }
    fn locate_mut(&mut self, address: u64, len: u64) -> Result<(&mut Mapping, usize), Error> {
        let base = *self.mappings.range(..=address).next_back().ok_or(Error::Invalid("unmapped guest address"))?.0;
        let m = self.mappings.get_mut(&base).ok_or(Error::Invalid("unmapped guest address"))?;
        let end = address.checked_add(len).ok_or(Error::Overflow)?;
        if end > m.end()? { return Err(Error::Invalid("guest access crosses mapping boundary")); }
        Ok((m, usize::try_from(address - base).map_err(|_| Error::Overflow)?))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessState { Created, Ready, Running, Blocked, Exited(i32), Faulted }
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CpuState {
    pub rax:u64,pub rbx:u64,pub rcx:u64,pub rdx:u64,pub rsi:u64,pub rdi:u64,
    pub rbp:u64,pub rsp:u64,pub r8:u64,pub r9:u64,pub r10:u64,pub r11:u64,
    pub r12:u64,pub r13:u64,pub r14:u64,pub r15:u64,pub rip:u64,rflags:u64,
}
#[derive(Clone, Debug)]
pub struct GuestProcess {
    pub id: u64,
    pub state: ProcessState,
    pub address_space: AddressSpace,
    pub cpu: CpuState,
    pub instruction_budget: u64,
}
impl GuestProcess {
    pub fn new(id: u64, instruction_budget: u64) -> Result<Self, Error> {
        if id == 0 || instruction_budget == 0 { return Err(Error::Invalid("guest process identity or budget")); }
        Ok(Self { id, state: ProcessState::Created, address_space: AddressSpace::new(), cpu: CpuState::default(), instruction_budget })
    }
    pub fn set_entry(&mut self, entry: u64, stack: u64) -> Result<(), Error> {
        if self.state != ProcessState::Created { return Err(Error::Invalid("entry can only be set before start")); }
        self.address_space.find(entry, 1)?;
        self.address_space.find(stack.checked_sub(1).ok_or(Error::Overflow)?, 1)?;
        self.cpu.rip = entry; self.cpu.rsp = stack; self.state = ProcessState::Ready; Ok(())
    }
    /// Deliberately does not transition to Running: a host execution backend is not yet wired.
    pub fn execution_backend_available(&self) -> bool { false }
}
