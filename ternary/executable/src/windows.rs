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
        if self.symbols.contains_key(&key) {
            return Err(Error::Invalid("duplicate Windows export"));
        }
        self.symbols.insert(key, address);
        Ok(())
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

/// Only APIs wired to the restricted native guest runtime are marked implemented.
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
    let implemented=matches!(name,"ExitProcess"|"GetCurrentProcessId"|"GetCurrentThreadId");
    ApiDecision{
        name:name.to_owned(),
        class,
        implemented,
        diagnostic:if implemented {
            "Implemented by the restricted PE32+ guest runtime"
        } else {
            "Windows API shim is classified but not connected to a verified kernel service"
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_restricted_guest_apis_are_marked_implemented() {
        for name in ["ExitProcess", "GetCurrentProcessId", "GetCurrentThreadId"] {
            assert!(classify_api(name).implemented, "{name}");
        }
        for name in ["CreateThread", "CreateFileW", "VirtualAlloc", "WriteConsoleW"] {
            assert!(!classify_api(name).implemented, "{name}");
        }
    }
}
