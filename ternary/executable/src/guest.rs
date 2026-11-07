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
        let mut cpu = CpuState::default();
        cpu.rflags = RFLAGS_RESERVED_ONE;
        Ok(Self { id, state: ProcessState::Created, address_space: AddressSpace::new(), cpu, instruction_budget })
    }
    pub fn set_entry(&mut self, entry: u64, stack: u64) -> Result<(), Error> {
        if self.state != ProcessState::Created { return Err(Error::Invalid("entry can only be set before start")); }
        self.address_space.find(entry, 1)?;
        let return_slot = stack.checked_sub(8).ok_or(Error::Overflow)?;
        self.address_space.write(return_slot, &0u64.to_le_bytes())?;
        self.cpu.rip = entry;
        self.cpu.rsp = return_slot;
        self.state = ProcessState::Ready;
        Ok(())
    }
    /// This in-crate model remains a bounded development interpreter; native execution
    /// is performed by the separate Redox process/address-space loader.
    pub fn execution_backend_available(&self) -> bool { false }
}


pub const RFLAGS_CF: u64 = 1 << 0;
pub const RFLAGS_ZF: u64 = 1 << 6;
pub const RFLAGS_PF: u64 = 1 << 2;
pub const RFLAGS_SF: u64 = 1 << 7;
pub const RFLAGS_OF: u64 = 1 << 11;
pub const RFLAGS_RESERVED_ONE: u64 = 1 << 1;
pub const SYSCALL_RFLAGS_MASK: u64 = (1 << 8) | (1 << 9) | (1 << 10) | (1 << 14);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutionEvent {
    Continue,
    Syscall { number: u64, arguments: [u64; 6], return_rip: u64 },
    Breakpoint { rip: u64 },
    Halted,
    BudgetExhausted,
}

impl AddressSpace {
    /// Fetch a single instruction without crossing a mapping boundary. Execute
    /// permission is checked independently from read permission.
    pub fn fetch_instruction(&self, address: u64, output: &mut [u8]) -> Result<usize, Error> {
        if output.is_empty() { return Err(Error::Invalid("empty instruction buffer")); }
        let mapping = self.find(address, 1)?;
        if !mapping.permissions.execute {
            return Err(Error::Invalid("instruction fetch from non-executable mapping"));
        }
        let offset = usize::try_from(address - mapping.base).map_err(|_| Error::Overflow)?;
        let available = mapping.bytes.len().saturating_sub(offset).min(output.len());
        output[..available].copy_from_slice(&mapping.bytes[offset..offset + available]);
        Ok(available)
    }
}

impl CpuState {
    fn register(&self, index: u8) -> Result<u64, Error> {
        match index {
            0=>Ok(self.rax),1=>Ok(self.rcx),2=>Ok(self.rdx),3=>Ok(self.rbx),
            4=>Ok(self.rsp),5=>Ok(self.rbp),6=>Ok(self.rsi),7=>Ok(self.rdi),
            8=>Ok(self.r8),9=>Ok(self.r9),10=>Ok(self.r10),11=>Ok(self.r11),
            12=>Ok(self.r12),13=>Ok(self.r13),14=>Ok(self.r14),15=>Ok(self.r15),
            _=>Err(Error::Invalid("x86 register index")),
        }
    }
    fn set_register(&mut self, index: u8, value: u64) -> Result<(), Error> {
        match index {
            0=>self.rax=value,1=>self.rcx=value,2=>self.rdx=value,3=>self.rbx=value,
            4=>self.rsp=value,5=>self.rbp=value,6=>self.rsi=value,7=>self.rdi=value,
            8=>self.r8=value,9=>self.r9=value,10=>self.r10=value,11=>self.r11=value,
            12=>self.r12=value,13=>self.r13=value,14=>self.r14=value,15=>self.r15=value,
            _=>return Err(Error::Invalid("x86 register index")),
        }
        Ok(())
    }
    fn condition(&self, code: u8) -> bool {
        let cf=self.rflags&RFLAGS_CF!=0;
        let zf=self.rflags&RFLAGS_ZF!=0;
        let sf=self.rflags&RFLAGS_SF!=0;
        let pf=self.rflags&RFLAGS_PF!=0;
        let of=self.rflags&RFLAGS_OF!=0;
        match code {
            0=>of,1=>!of,2=>cf,3=>!cf,4=>zf,5=>!zf,6=>cf||zf,7=>!cf&&!zf,
            8=>sf,9=>!sf,10=>pf,11=>!pf,12=>sf!=of,13=>sf==of,
            14=>zf||(sf!=of),15=>!zf&&(sf==of),_=>false,
        }
    }
}

impl GuestProcess {
    /// Execute one instruction from the original guest image. This is a bounded
    /// interpreter for the explicitly decoded subset, not native host execution.
    pub fn step(&mut self) -> Result<ExecutionEvent, Error> {
        let was_runnable = matches!(self.state, ProcessState::Ready | ProcessState::Running);
        let result = self.step_inner();
        if was_runnable && result.is_err() {
            self.state = ProcessState::Faulted;
        }
        result
    }

    fn step_inner(&mut self) -> Result<ExecutionEvent, Error> {
        if !matches!(self.state, ProcessState::Ready | ProcessState::Running) {
            return Err(Error::Invalid("guest process is not runnable"));
        }
        if self.instruction_budget == 0 { return Ok(ExecutionEvent::BudgetExhausted); }
        let rip=self.cpu.rip;
        let mut bytes=[0u8; crate::decoder::MAX_INSTRUCTION_BYTES];
        let count=self.address_space.fetch_instruction(rip,&mut bytes)?;
        let decoded=crate::decoder::decode(&bytes[..count])?;
        let next=rip.checked_add(decoded.length as u64).ok_or(Error::Overflow)?;
        use crate::decoder::Instruction as I;
        match decoded.instruction {
            I::Nop=>self.cpu.rip=next,
            I::MovRegImm64{register,value}=>{self.cpu.set_register(register,value)?;self.cpu.rip=next;}
            I::PushReg(register)=>{
                let value=self.cpu.register(register)?;
                let new_rsp=self.cpu.rsp.checked_sub(8).ok_or(Error::Overflow)?;
                self.address_space.write(new_rsp,&value.to_le_bytes())?;
                self.cpu.rsp=new_rsp;self.cpu.rip=next;
            }
            I::PopReg(register)=>{
                let mut data=[0u8;8];self.address_space.read(self.cpu.rsp,&mut data)?;
                let value=u64::from_le_bytes(data);
                let new_rsp=self.cpu.rsp.checked_add(8).ok_or(Error::Overflow)?;
                self.cpu.set_register(register,value)?;
                if register!=4 {self.cpu.rsp=new_rsp;}
                self.cpu.rip=next;
            }
            I::RelativeCall(displacement)=>{
                let new_rsp=self.cpu.rsp.checked_sub(8).ok_or(Error::Overflow)?;
                self.address_space.write(new_rsp,&next.to_le_bytes())?;
                self.cpu.rsp=new_rsp;
                self.cpu.rip=add_relative(next,displacement)?;
            }
            I::RelativeJump(displacement)=>self.cpu.rip=add_relative(next,displacement)?,
            I::ConditionalJump{condition,displacement}=>{
                self.cpu.rip=if self.cpu.condition(condition){add_relative(next,displacement)?}else{next};
            }
            I::Ret=>{
                let mut data=[0u8;8];self.address_space.read(self.cpu.rsp,&mut data)?;
                self.cpu.rsp=self.cpu.rsp.checked_add(8).ok_or(Error::Overflow)?;
                let target=u64::from_le_bytes(data);
                if target==0 {
                    self.instruction_budget-=1;
                    self.state=ProcessState::Exited(0);
                    return Ok(ExecutionEvent::Halted);
                }
                self.cpu.rip=target;
            }
            I::Syscall=>{
                // Intel SYSCALL saves the next instruction and user flags in RCX/R11.
                // The host broker handles the transition; guest syscall numbers are never
                // forwarded directly to the host kernel.
                self.cpu.rcx=next;
                self.cpu.r11=self.cpu.rflags;
                self.cpu.rflags=(self.cpu.rflags & !SYSCALL_RFLAGS_MASK) | RFLAGS_RESERVED_ONE;
                self.cpu.rip=next;
                self.instruction_budget-=1;
                self.state=ProcessState::Blocked;
                return Ok(ExecutionEvent::Syscall{
                    number:self.cpu.rax,
                    arguments:[self.cpu.rdi,self.cpu.rsi,self.cpu.rdx,self.cpu.r10,self.cpu.r8,self.cpu.r9],
                    return_rip:next,
                });
            }
            I::Int3=>{
                self.cpu.rip=next;
                self.instruction_budget-=1;
                self.state=ProcessState::Blocked;
                return Ok(ExecutionEvent::Breakpoint{rip});
            }
            I::Hlt=>{
                return Err(Error::Unsupported("HLT is privileged and faults in a userspace guest"));
            }
        }
        self.instruction_budget-=1;
        self.state=ProcessState::Running;
        Ok(ExecutionEvent::Continue)
    }

    /// Run until a syscall, breakpoint, halt, fault, or budget exhaustion.
    /// The caller must broker syscalls and explicitly resume a blocked process.
    pub fn run(&mut self) -> Result<ExecutionEvent, Error> {
        loop {
            match self.step()? {
                ExecutionEvent::Continue=>continue,
                event=>return Ok(event),
            }
        }
    }

    pub fn resume_with_syscall_result(&mut self, result: i64) -> Result<(), Error> {
        if self.state!=ProcessState::Blocked {return Err(Error::Invalid("guest is not blocked at a syscall boundary"));}
        self.cpu.rax=result as u64;
        self.cpu.rflags=self.cpu.r11 | RFLAGS_RESERVED_ONE;
        self.state=ProcessState::Ready;
        Ok(())
    }
}

fn add_relative(next: u64, displacement: i32) -> Result<u64, Error> {
    if displacement>=0 {next.checked_add(displacement as u64).ok_or(Error::Overflow)}
    else {next.checked_sub(displacement.unsigned_abs() as u64).ok_or(Error::Overflow)}
}


#[cfg(test)]
mod execution_tests {
    use super::*;

    #[test]
    fn executes_original_x86_bytes_and_returns_from_the_entry_point() {
        let mut process = GuestProcess::new(7, 32).unwrap();
        process.address_space.map(0x400000, PAGE_SIZE, Permissions::READ_WRITE).unwrap();
        // mov rax, 42; ret (the loader-installed zero return sentinel exits the process)
        let code = [0x48,0xb8,42,0,0,0,0,0,0,0,0xc3];
        process.address_space.write(0x400000, &code).unwrap();
        process.address_space.protect(0x400000, Permissions::READ_EXECUTE).unwrap();
        process.address_space.map(0x700000, PAGE_SIZE, Permissions::READ_WRITE).unwrap();
        process.set_entry(0x400000, 0x701000).unwrap();
        assert_eq!(process.run().unwrap(), ExecutionEvent::Halted);
        assert_eq!(process.cpu.rax, 42);
        assert_eq!(process.state, ProcessState::Exited(0));
    }

    #[test]
    fn syscall_is_a_broker_boundary_not_a_host_syscall() {
        let mut process = GuestProcess::new(8, 8).unwrap();
        process.address_space.map(0x400000, PAGE_SIZE, Permissions::READ_WRITE).unwrap();
        process.address_space.write(0x400000, &[0x0f,0x05]).unwrap();
        process.address_space.protect(0x400000, Permissions::READ_EXECUTE).unwrap();
        process.address_space.map(0x700000, PAGE_SIZE, Permissions::READ_WRITE).unwrap();
        process.set_entry(0x400000, 0x701000).unwrap();
        process.cpu.rax = 0x1234;
        assert_eq!(process.run().unwrap(), ExecutionEvent::Syscall {
            number: 0x1234, arguments: [0;6], return_rip: 0x400002
        });
        process.resume_with_syscall_result(5).unwrap();
        assert_eq!(process.cpu.rax, 5);
    }

    #[test]
    fn instruction_fetch_requires_execute_permission() {
        let mut process = GuestProcess::new(9, 8).unwrap();
        process.address_space.map(0x400000, PAGE_SIZE, Permissions::READ_WRITE).unwrap();
        process.address_space.write(0x400000, &[0xf4]).unwrap();
        process.address_space.map(0x700000, PAGE_SIZE, Permissions::READ_WRITE).unwrap();
        process.set_entry(0x400000, 0x701000).unwrap();
        assert!(process.run().is_err());
        assert_eq!(process.state, ProcessState::Faulted);
    }
}
