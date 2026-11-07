//! Direct executable-format foundations. Parsed bytes remain inert: this crate
//! does not execute guest code or translate applications to TIR or .trx.

pub mod loader;\npub mod engine;

pub mod decoder;
pub mod guest;
pub mod macos;
pub mod windows;


use core::fmt;

pub const MAX_IMAGE_SIZE: usize = 512 * 1024 * 1024;
pub const MAX_SECTIONS: usize = 96;
pub const MAX_COMMANDS: usize = 4096;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Truncated(&'static str), BadMagic(&'static str), Unsupported(&'static str),
    Architecture(u32), Invalid(&'static str), Overflow, Limit(&'static str), Utf8(&'static str),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated(s)=>write!(f,"truncated executable: {s}"),
            Self::BadMagic(s)=>write!(f,"invalid executable magic: {s}"),
            Self::Unsupported(s)=>write!(f,"unsupported executable: {s}"),
            Self::Architecture(a)=>write!(f,"unsupported architecture: {a:#x}"),
            Self::Invalid(s)=>write!(f,"invalid executable: {s}"),
            Self::Overflow=>f.write_str("executable offset or size overflow"),
            Self::Limit(s)=>write!(f,"executable resource limit: {s}"),
            Self::Utf8(s)=>write!(f,"invalid UTF-8 metadata: {s}"),
        }
    }
}
impl std::error::Error for Error {}

fn range(data:&[u8], off:u64, len:u64, what:&'static str)->Result<&[u8],Error>{
    let a=usize::try_from(off).map_err(|_|Error::Overflow)?;
    let n=usize::try_from(len).map_err(|_|Error::Overflow)?;
    let b=a.checked_add(n).ok_or(Error::Overflow)?;
    data.get(a..b).ok_or(Error::Truncated(what))
}
fn r16(d:&[u8],o:usize,w:&'static str)->Result<u16,Error>{let b=d.get(o..o.checked_add(2).ok_or(Error::Overflow)?).ok_or(Error::Truncated(w))?;Ok(u16::from_le_bytes([b[0],b[1]]))}
fn r32(d:&[u8],o:usize,w:&'static str)->Result<u32,Error>{let b=d.get(o..o.checked_add(4).ok_or(Error::Overflow)?).ok_or(Error::Truncated(w))?;Ok(u32::from_le_bytes(b.try_into().map_err(|_|Error::Truncated(w))?))}
fn r64(d:&[u8],o:usize,w:&'static str)->Result<u64,Error>{let b=d.get(o..o.checked_add(8).ok_or(Error::Overflow)?).ok_or(Error::Truncated(w))?;Ok(u64::from_le_bytes(b.try_into().map_err(|_|Error::Truncated(w))?))}
fn cstr(d:&[u8],o:usize,n:usize,w:&'static str)->Result<String,Error>{
    let tail=d.get(o..).ok_or(Error::Truncated(w))?;
    let end=tail.iter().take(n).position(|b|*b==0).ok_or(Error::Invalid(w))?;
    Ok(core::str::from_utf8(&tail[..end]).map_err(|_|Error::Utf8(w))?.to_owned())
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Format{Pe32Plus,MachO64}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum Architecture{X86_64}
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct Section{pub name:String,pub address:u64,pub memory_size:u64,pub file_offset:u64,pub file_size:u64,pub executable:bool,pub writable:bool}
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct Description{pub format:Format,pub architecture:Architecture,pub preferred_base:u64,pub entry_point:u64,pub image_size:u64,pub sections:Vec<Section>,pub dependencies:Vec<String>}
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct InertImage{pub description:Description,pub bytes:Vec<u8>}

pub mod pe{
    use super::*;
    const AMD64:u16=0x8664;
    pub fn inspect(d:&[u8])->Result<Description,Error>{
        if !d.starts_with(b"MZ"){return Err(Error::BadMagic("DOS MZ"))}
        let pe=r32(d,0x3c,"e_lfanew")? as usize;
        if d.get(pe..pe.checked_add(4).ok_or(Error::Overflow)?)!=Some(b"PE\0\0"){return Err(Error::BadMagic("PE signature"))}
        let c=pe.checked_add(4).ok_or(Error::Overflow)?;
        let machine=r16(d,c,"COFF machine")?;
        if machine!=AMD64{return Err(Error::Architecture(machine as u32))}
        let count=r16(d,c+2,"section count")? as usize;
        if count==0||count>MAX_SECTIONS{return Err(Error::Limit("PE section count"))}
        let opt_size=r16(d,c+16,"optional-header size")? as usize;
        let opt=c.checked_add(20).ok_or(Error::Overflow)?;
        let oh=d.get(opt..opt.checked_add(opt_size).ok_or(Error::Overflow)?).ok_or(Error::Truncated("PE optional header"))?;
        if r16(oh,0,"optional-header magic")?!=0x20b{return Err(Error::Unsupported("PE32 (32-bit) image"))}
        if opt_size<112{return Err(Error::Truncated("PE32+ optional header"))}
        let entry=r32(oh,16,"entry RVA")? as u64;
        let base=r64(oh,24,"image base")?;
        let image=r32(oh,56,"SizeOfImage")? as u64;
        let headers=r32(oh,60,"SizeOfHeaders")? as u64;
        let align=r32(oh,32,"section alignment")?;
        if image==0||image>MAX_IMAGE_SIZE as u64{return Err(Error::Limit("PE image size"))}
        if headers>image||headers>d.len() as u64||align==0||entry>=image{return Err(Error::Invalid("PE image/header alignment or entry point"))}
        let table=opt.checked_add(opt_size).ok_or(Error::Overflow)?;
        range(d,table as u64,(count.checked_mul(40).ok_or(Error::Overflow)?) as u64,"section table")?;
        let mut sections=Vec::new();
        sections.try_reserve_exact(count).map_err(|_|Error::Limit("section metadata allocation"))?;
        for i in 0..count{
            let p=table+i*40;
            let raw=&d[p..p+8];let n=raw.iter().position(|b|*b==0).unwrap_or(8);
            let name=String::from_utf8_lossy(&raw[..n]).into_owned();
            let vs=r32(d,p+8,"section virtual size")? as u64;
            let va=r32(d,p+12,"section RVA")? as u64;
            let rs=r32(d,p+16,"section raw size")? as u64;
            let rp=r32(d,p+20,"section raw pointer")? as u64;
            let flags=r32(d,p+36,"section characteristics")?;
            if va.checked_add(vs.max(rs)).ok_or(Error::Overflow)? > image{return Err(Error::Invalid("section outside SizeOfImage"))}
            if rs>0{range(d,rp,rs,"section bytes")?;}
            sections.push(Section{name,address:va,memory_size:vs.max(rs),file_offset:rp,file_size:rs,executable:flags&0x2000_0000!=0,writable:flags&0x8000_0000!=0});
        }
        for i in 0..sections.len(){for b in sections.iter().skip(i+1){
            let a=&sections[i];let ae=a.address.checked_add(a.memory_size).ok_or(Error::Overflow)?;
            let be=b.address.checked_add(b.memory_size).ok_or(Error::Overflow)?;
            if a.address<be&&b.address<ae{return Err(Error::Invalid("overlapping virtual sections"))}
        }}
        let dirs=r32(oh,108,"data-directory count")?.min(16) as usize;
        let mut deps=Vec::new();
        if dirs>1{
            let irva=r32(oh,120,"import directory RVA")?;
            let isize=r32(oh,124,"import directory size")?;
            if irva!=0||isize!=0{
                if irva==0||isize<20{return Err(Error::Invalid("import directory"))}
                let mut raw=None;
                if irva as u64+20<=headers{raw=Some(irva as usize)}
                for s in &sections{if irva as u64>=s.address&&irva as u64+20<=s.address+s.file_size{raw=Some((s.file_offset+irva as u64-s.address) as usize);}}
                let mut at=raw.ok_or(Error::Invalid("import directory RVA not file-backed"))?;
                let end=at.checked_add(isize as usize).ok_or(Error::Overflow)?;
                let mut term=false;
                while at.checked_add(20).ok_or(Error::Overflow)?<=end{
                    let dsc=d.get(at..at+20).ok_or(Error::Truncated("import descriptor"))?;
                    let name=r32(dsc,12,"import name RVA")?;
                    let thunk=r32(dsc,16,"import address table")?;
                    if r32(dsc,0,"import lookup table")?==0&&name==0&&thunk==0{term=true;break}
                    let mut no=None;
                    if name as u64+1<=headers{no=Some(name as usize)}
                    for s in &sections{if name as u64>=s.address&&name as u64<s.address+s.file_size{no=Some((s.file_offset+name as u64-s.address) as usize)}}
                    let lib=cstr(d,no.ok_or(Error::Invalid("import name RVA not file-backed"))?,4096,"import library name")?;
                    if lib.is_empty(){return Err(Error::Invalid("empty import library"))}
                    if !deps.contains(&lib){deps.push(lib)}
                    if deps.len()>MAX_COMMANDS{return Err(Error::Limit("import libraries"))}
                    at+=20;
                }
                if !term{return Err(Error::Invalid("unterminated import descriptor table"))}
            }
        }
        Ok(Description{format:Format::Pe32Plus,architecture:Architecture::X86_64,preferred_base:base,entry_point:base.checked_add(entry).ok_or(Error::Overflow)?,image_size:image,sections,dependencies:deps})
    }
    /// Maps the original PE headers/sections into inert memory at the preferred base.
    pub fn map_preferred(d:&[u8])->Result<InertImage,Error>{
        let desc=inspect(d)?;
        let pe=r32(d,0x3c,"e_lfanew")? as usize;let c=pe+4;
        let os=r16(d,c+16,"optional-header size")? as usize;let opt=c+20;
        let hs=r32(d,opt+60,"SizeOfHeaders")? as usize;
        let table=opt+os;
        let mut bytes=Vec::new();bytes.try_reserve_exact(desc.image_size as usize).map_err(|_|Error::Limit("mapped image allocation"))?;bytes.resize(desc.image_size as usize,0);
        bytes[..hs].copy_from_slice(&d[..hs]);
        for i in 0..desc.sections.len(){
            let p=table+i*40;let size=r32(d,p+16,"raw size")? as usize;
            let src=r32(d,p+20,"raw pointer")? as usize;let dst=r32(d,p+12,"section RVA")? as usize;
            let se=src.checked_add(size).ok_or(Error::Overflow)?;let de=dst.checked_add(size).ok_or(Error::Overflow)?;
            bytes.get_mut(dst..de).ok_or(Error::Invalid("mapped section range"))?.copy_from_slice(d.get(src..se).ok_or(Error::Truncated("section data"))?);
        }
        Ok(InertImage{description:desc,bytes})
    }
}

pub mod macho{
    use super::*;
    const CPU_X86_64:u32=0x0100_0007;
    const LC_SEGMENT_64:u32=0x19;const LC_LOAD_DYLIB:u32=0x0c;const LC_LOAD_WEAK_DYLIB:u32=0x8000_0018;const LC_REEXPORT_DYLIB:u32=0x8000_001f;const LC_LOAD_UPWARD_DYLIB:u32=0x8000_0023;const LC_MAIN:u32=0x8000_0028;
    pub fn inspect(d:&[u8])->Result<Description,Error>{
        if d.len()<32{return Err(Error::Truncated("Mach-O header"))}
        if r32(d,0,"magic")?!=0xfeed_facf{return Err(Error::Unsupported("only little-endian Mach-O 64 is supported"))}
        let cpu=r32(d,4,"CPU type")?;if cpu!=CPU_X86_64{return Err(Error::Architecture(cpu))}
        let kind=r32(d,12,"file type")?;if kind!=2&&kind!=6{return Err(Error::Unsupported("Mach-O is not executable/dylinker"))}
        let count=r32(d,16,"load command count")? as usize;let bytes=r32(d,20,"load command bytes")? as usize;
        if count>MAX_COMMANDS||bytes>d.len().saturating_sub(32){return Err(Error::Limit("Mach-O load commands"))}
        let end=32usize.checked_add(bytes).ok_or(Error::Overflow)?;let mut at=32usize;
        let mut sections=Vec::new();let mut deps=Vec::new();let mut entry_file=None;
        for _ in 0..count{
            if at+8>end{return Err(Error::Truncated("load command"))}
            let cmd=r32(d,at,"command")?;let size=r32(d,at+4,"command size")? as usize;
            if size<8||at.checked_add(size).ok_or(Error::Overflow)?>end{return Err(Error::Invalid("load command size"))}
            if cmd==LC_SEGMENT_64{
                if size<72{return Err(Error::Truncated("segment command"))}
                let n=&d[at+8..at+24];let nn=n.iter().position(|b|*b==0).unwrap_or(16);
                let name=String::from_utf8_lossy(&n[..nn]).into_owned();
                let va=r64(d,at+24,"segment vmaddr")?;let vs=r64(d,at+32,"segment vmsize")?;
                let fo=r64(d,at+40,"segment fileoff")?;let fs=r64(d,at+48,"segment filesize")?;
                let prot=r32(d,at+56,"segment protections")?;let ns=r32(d,at+64,"segment section count")? as usize;
                if 72usize.checked_add(ns.checked_mul(80).ok_or(Error::Overflow)?).ok_or(Error::Overflow)?>size{return Err(Error::Invalid("segment section table"))}
                if fs>0{range(d,fo,fs,"segment file bytes")?;}
                va.checked_add(vs).ok_or(Error::Overflow)?;
                sections.push(Section{name,address:va,memory_size:vs,file_offset:fo,file_size:fs,executable:prot&4!=0,writable:prot&2!=0});
            }else if cmd==LC_LOAD_DYLIB||cmd==LC_LOAD_WEAK_DYLIB||cmd==LC_REEXPORT_DYLIB||cmd==LC_LOAD_UPWARD_DYLIB{
                if size<24{return Err(Error::Truncated("dylib command"))}
                let off=r32(d,at+8,"dylib name offset")? as usize;
                if off<8||off>=size{return Err(Error::Invalid("dylib name offset"))}
                let name=cstr(d,at+off,size-off,"dylib name")?;if name.is_empty(){return Err(Error::Invalid("empty dylib name"))}
                if !deps.contains(&name){deps.push(name)}
            }else if cmd==LC_MAIN{
                if size<24{return Err(Error::Truncated("LC_MAIN"))}
                entry_file=Some(r64(d,at+8,"entry file offset")?);
            }
            at+=size;
        }
        if at!=end{return Err(Error::Invalid("sizeofcmds mismatch"))}
        let file=entry_file.ok_or(Error::Invalid("LC_MAIN missing"))?;
        let mut entry=None;let mut base=u64::MAX;let mut span=0u64;
        for s in &sections{
            base=base.min(s.address);span=span.max(s.address.checked_add(s.memory_size).ok_or(Error::Overflow)?);
            if file>=s.file_offset&&file<s.file_offset.checked_add(s.file_size).ok_or(Error::Overflow)?{
                entry=Some(s.address.checked_add(file-s.file_offset).ok_or(Error::Overflow)?)
            }
        }
        let image=span.saturating_sub(base);if image>MAX_IMAGE_SIZE as u64{return Err(Error::Limit("Mach-O virtual image span"))}
        Ok(Description{format:Format::MachO64,architecture:Architecture::X86_64,preferred_base:if base==u64::MAX{0}else{base},entry_point:entry.ok_or(Error::Invalid("entry point outside file-backed segment"))?,image_size:image,sections,dependencies:deps})
    }
}

pub fn inspect(data:&[u8])->Result<Description,Error>{
    if data.starts_with(b"MZ"){pe::inspect(data)}
    else if data.starts_with(&[0xcf,0xfa,0xed,0xfe]){macho::inspect(data)}
    else{Err(Error::Unsupported("expected PE32+ or little-endian Mach-O 64"))}
}

/// Resolve CFBundleExecutable from bounded XML Info.plist against a directory listing.
/// Binary plists, XML entities and path traversal are rejected.
pub fn select_app_executable<'a>(plist:&[u8],members:&'a [&'a str])->Result<&'a str,Error>{
    if plist.len()>1024*1024{return Err(Error::Limit("Info.plist"))}
    let xml=core::str::from_utf8(plist).map_err(|_|Error::Utf8("Info.plist"))?;
    let k="<key>CFBundleExecutable</key>";let p=xml.find(k).ok_or(Error::Invalid("CFBundleExecutable missing"))?;
    let rest=&xml[p+k.len()..];let s=rest.find("<string>").ok_or(Error::Invalid("CFBundleExecutable is not a string"))?+8;
    let e=rest[s..].find("</string>").ok_or(Error::Invalid("unterminated CFBundleExecutable"))?;
    let name=&rest[s..s+e];
    if name.is_empty()||name.contains('/')||name.contains('\\')||name.contains("..")||name.contains('&'){return Err(Error::Invalid("unsafe bundle executable name"))}
    members.iter().copied().find(|m|*m==name).ok_or(Error::Invalid("bundle executable absent from member listing"))
}

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct LaunchAssessment{pub description:Description,pub blockers:Vec<&'static str>,pub launchable:bool}
/// Parsing never implies launch support. This currently returns explicit backend blockers.
pub fn assess_launch(data:&[u8])->Result<LaunchAssessment,Error>{
    let description=inspect(data)?;let mut blockers=vec![
        "isolated guest process/address-space backend is not integrated",
        "x86-64 instruction decoder, execution and fault boundary are not implemented",
    ];
    match description.format{
        Format::Pe32Plus=>{blockers.push("PE imports require a verified DLL catalog, platform dependency policy and kernel-backed Windows API exports");blockers.push("Windows process, syscall, runtime and API compatibility services are not implemented");}
        Format::MachO64=>{blockers.push("Mach-O relocation, dyld rebasing, binding and dependency resolution are not implemented");blockers.push("macOS process, syscall, runtime and framework compatibility services are not implemented");}
    }
    Ok(LaunchAssessment{description,blockers,launchable:false})
}


#[cfg(test)]
mod tests {
    use super::*;

    fn put16(d: &mut [u8], at: usize, v: u16) { d[at..at+2].copy_from_slice(&v.to_le_bytes()); }
    fn put32(d: &mut [u8], at: usize, v: u32) { d[at..at+4].copy_from_slice(&v.to_le_bytes()); }
    fn put64(d: &mut [u8], at: usize, v: u64) { d[at..at+8].copy_from_slice(&v.to_le_bytes()); }

    fn pe_fixture() -> Vec<u8> {
        let mut d = vec![0u8; 0x400];
        d[0..2].copy_from_slice(b"MZ");
        put32(&mut d, 0x3c, 0x80);
        d[0x80..0x84].copy_from_slice(b"PE\0\0");
        let coff = 0x84;
        put16(&mut d, coff, 0x8664);
        put16(&mut d, coff+2, 1);
        put16(&mut d, coff+16, 0xf0);
        let opt = coff+20;
        put16(&mut d, opt, 0x20b);
        put32(&mut d, opt+16, 0x1000);
        put64(&mut d, opt+24, 0x1400_0000_0);
        put32(&mut d, opt+32, 0x1000);
        put32(&mut d, opt+36, 0x200);
        put32(&mut d, opt+56, 0x2000);
        put32(&mut d, opt+60, 0x200);
        put32(&mut d, opt+108, 0);
        let sec = opt+0xf0;
        d[sec..sec+5].copy_from_slice(b".text");
        put32(&mut d, sec+8, 0x100);
        put32(&mut d, sec+12, 0x1000);
        put32(&mut d, sec+16, 0x200);
        put32(&mut d, sec+20, 0x200);
        put32(&mut d, sec+36, 0x6000_0020);
        d[0x200] = 0x90;
        d
    }

    fn macho_fixture() -> Vec<u8> {
        let mut d = vec![0u8; 0x300];
        put32(&mut d, 0, 0xfeed_facf);
        put32(&mut d, 4, 0x0100_0007);
        put32(&mut d, 12, 2);
        put32(&mut d, 16, 2);
        put32(&mut d, 20, 96);
        let seg = 32;
        put32(&mut d, seg, 0x19);
        put32(&mut d, seg+4, 72);
        d[seg+8..seg+14].copy_from_slice(b"__TEXT");
        put64(&mut d, seg+24, 0x1_0000_0000);
        put64(&mut d, seg+32, 0x1000);
        put64(&mut d, seg+40, 0);
        put64(&mut d, seg+48, 0x300);
        put32(&mut d, seg+56, 5);
        put32(&mut d, seg+64, 0);
        let main = seg+72;
        put32(&mut d, main, 0x8000_0028);
        put32(&mut d, main+4, 24);
        put64(&mut d, main+8, 0x200);
        d
    }

    #[test]
    fn pe32_plus_inspection_and_inert_mapping() {
        let input = pe_fixture();
        let parsed = pe::inspect(&input).unwrap();
        assert_eq!(parsed.format, Format::Pe32Plus);
        assert_eq!(parsed.architecture, Architecture::X86_64);
        assert_eq!(parsed.entry_point, 0x1_4000_1000);
        assert_eq!(parsed.sections[0].name, ".text");
        let mapped = pe::map_preferred(&input).unwrap();
        assert_eq!(mapped.bytes.len(), 0x2000);
        assert_eq!(mapped.bytes[0x1000], 0x90);
        assert!(!assess_launch(&input).unwrap().launchable);
    }

    #[test]
    fn macho64_inspection_and_entry_point() {
        let parsed = macho::inspect(&macho_fixture()).unwrap();
        assert_eq!(parsed.format, Format::MachO64);
        assert_eq!(parsed.entry_point, 0x1_0000_0200);
        assert_eq!(parsed.sections[0].name, "__TEXT");
    }

    #[test]
    fn rejects_truncated_and_wrong_architecture_inputs() {
        assert!(matches!(pe::inspect(b"MZ"), Err(Error::Truncated(_))));
        let mut input = pe_fixture();
        put16(&mut input, 0x84, 0x014c);
        assert_eq!(pe::inspect(&input), Err(Error::Architecture(0x014c)));
        assert!(matches!(macho::inspect(b"\xcf\xfa\xed\xfe"), Err(Error::Truncated(_))));
    }

    #[test]
    fn selects_bundle_executable_without_path_traversal() {
        let plist = br#"<plist><dict><key>CFBundleExecutable</key><string>Trion</string></dict></plist>"#;
        assert_eq!(select_app_executable(plist, &["Resources", "Trion"]).unwrap(), "Trion");
        let bad = br#"<plist><dict><key>CFBundleExecutable</key><string>../Trion</string></dict></plist>"#;
        assert!(select_app_executable(bad, &["../Trion"]).is_err());
    }
}
