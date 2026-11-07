//! Explicit Windows ABI compatibility boundary. Unsupported APIs fail closed.

use crate::Error;
use std::collections::BTreeMap;

pub const MAX_IMPORTS: usize = 16_384;
pub const MAX_SYMBOL_NAME: usize = 4096;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImportSymbol { Name(String), Ordinal(u16) }
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Import { pub library: String, pub symbol: ImportSymbol }
#[derive(Clone, Debug, Default)]
pub struct ExportTable { symbols: BTreeMap<(String, ImportSymbol), u64> }
impl ExportTable {
    pub fn insert(&mut self, library:String, symbol:ImportSymbol, address:u64)->Result<(),Error>{
        if library.is_empty() || library.len()>MAX_SYMBOL_NAME { return Err(Error::Invalid("Windows import library name")); }
        if let ImportSymbol::Name(n)=&symbol { if n.is_empty()||n.len()>MAX_SYMBOL_NAME { return Err(Error::Invalid("Windows import symbol name")); } }
        if address==0 { return Err(Error::Invalid("null Windows export address")); }
        let key = (library.to_ascii_lowercase(), symbol);
        if self.symbols.insert(key,address).is_some(){return Err(Error::Invalid("duplicate Windows export"));} Ok(())
    }
    pub fn resolve(&self, import:&Import)->Result<u64,Error>{
        self.symbols.get(&(import.library.to_ascii_lowercase(),import.symbol.clone()))
            .copied().ok_or(Error::Unsupported("unresolved Windows import; API is outside the declared compatibility profile"))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApiClass { Process, Thread, VirtualMemory, File, Synchronization, Time, Console, Unsupported }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiDecision { pub name:String, pub class:ApiClass, pub implemented:bool, pub diagnostic:&'static str }

/// No API is advertised as implemented until a concrete kernel-backed service exists.
pub fn classify_api(name:&str)->ApiDecision{
    let class=match name {
        "ExitProcess"|"GetCurrentProcessId"|"GetCurrentThreadId"=>ApiClass::Process,
        "CreateThread"|"ExitThread"=>ApiClass::Thread,
        "VirtualAlloc"|"VirtualFree"|"VirtualProtect"=>ApiClass::VirtualMemory,
        "CreateFileW"|"ReadFile"|"WriteFile"|"CloseHandle"=>ApiClass::File,
        "CreateMutexW"|"WaitForSingleObject"|"ReleaseMutex"=>ApiClass::Synchronization,
        "GetSystemTimeAsFileTime"|"QueryPerformanceCounter"=>ApiClass::Time,
        "WriteConsoleW"|"GetConsoleMode"=>ApiClass::Console,
        _=>ApiClass::Unsupported,
    };
    ApiDecision{name:name.to_owned(),class,implemented:false,diagnostic:"Windows API shim is classified but not connected to a verified kernel service"}
}
