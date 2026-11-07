//! Checked PE32+ relocation application over an inert mapped image.

use crate::{Error, InertImage, Format};

fn u16le(b:&[u8],o:usize)->Result<u16,Error>{
    let e=o.checked_add(2).ok_or(Error::Overflow)?;
    let x=b.get(o..e).ok_or(Error::Truncated("PE relocation entry"))?;
    Ok(u16::from_le_bytes([x[0],x[1]]))
}
fn u32le(b:&[u8],o:usize)->Result<u32,Error>{
    let e=o.checked_add(4).ok_or(Error::Overflow)?;
    let x=b.get(o..e).ok_or(Error::Truncated("PE relocation block"))?;
    Ok(u32::from_le_bytes(x.try_into().map_err(|_|Error::Truncated("PE relocation block"))?))
}
fn u64le(b:&[u8],o:usize)->Result<u64,Error>{
    let e=o.checked_add(8).ok_or(Error::Overflow)?;
    let x=b.get(o..e).ok_or(Error::Truncated("PE DIR64 target"))?;
    Ok(u64::from_le_bytes(x.try_into().map_err(|_|Error::Truncated("PE DIR64 target"))?))
}

/// Apply IMAGE_REL_BASED_DIR64 relocations to a PE32+ image already mapped at its
/// preferred base. Unsupported relocation types fail closed. A zero relocation
/// directory is accepted only when the requested base equals the preferred base.
pub fn rebase_pe32_plus(image:&mut InertImage, actual_base:u64)->Result<(),Error>{
    if image.description.format!=Format::Pe32Plus {
        return Err(Error::Unsupported("PE relocation requested for non-PE image"));
    }
    let preferred=image.description.preferred_base;
    if actual_base==preferred { return Ok(()); }
    actual_base.checked_add(image.description.image_size).ok_or(Error::Overflow)?;
    preferred.checked_add(image.description.image_size).ok_or(Error::Overflow)?;
    let bytes=&image.bytes;
    if bytes.len()<0x40 { return Err(Error::Truncated("mapped PE headers")); }
    let pe=u32le(bytes,0x3c)? as usize;
    let coff=pe.checked_add(4).ok_or(Error::Overflow)?;
    let optional=coff.checked_add(20).ok_or(Error::Overflow)?;
    let optional_size=u16le(bytes,coff+16)? as usize;
    if optional_size<112 { return Err(Error::Truncated("PE32+ optional header")); }
    let optional_end=optional.checked_add(optional_size).ok_or(Error::Overflow)?;
    if optional_end>bytes.len() { return Err(Error::Truncated("PE optional header")); }
    if u16le(bytes,optional)?!=0x20b { return Err(Error::Unsupported("PE32 relocation")); }
    let directory_count=u32le(bytes,optional+108)?;
    if directory_count<=5 || optional_size<160 {
        return Err(Error::Unsupported("PE image has no base relocation directory"));
    }
    let reloc=optional.checked_add(112+5*8).ok_or(Error::Overflow)?;
    let rva=u32le(bytes,reloc)? as usize;
    let size=u32le(bytes,reloc+4)? as usize;
    if rva==0 || size==0 { return Err(Error::Unsupported("PE image has no base relocation directory")); }
    let end=rva.checked_add(size).ok_or(Error::Overflow)?;
    if end>bytes.len() { return Err(Error::Truncated("PE base relocation directory")); }
    let delta=actual_base as i128-preferred as i128;
    let mut at=rva;
    let mut count=0usize;
    let mut writes:Vec<(usize,u64)>=Vec::new();
    while at<end {
        if end-at<8 { return Err(Error::Invalid("partial PE relocation block")); }
        let page=u32le(bytes,at)? as usize;
        let block_size=u32le(bytes,at+4)? as usize;
        if block_size<8 || block_size%2!=0 || at.checked_add(block_size).ok_or(Error::Overflow)?>end {
            return Err(Error::Invalid("invalid PE relocation block size"));
        }
        if page%0x1000!=0 || page>=bytes.len() {
            return Err(Error::Invalid("invalid PE relocation page RVA"));
        }
        let entries=(block_size-8)/2;
        for i in 0..entries {
            count=count.checked_add(1).ok_or(Error::Overflow)?;
            if count>1_000_000 { return Err(Error::Limit("PE relocation count")); }
            let item=u16le(bytes,at+8+i*2)?;
            let kind=item>>12;
            let offset=(item&0x0fff) as usize;
            if kind==0 { continue; }
            if kind!=10 { return Err(Error::Unsupported("PE relocation type outside DIR64 subset")); }
            let target=page.checked_add(offset).ok_or(Error::Overflow)?;
            let target_end=target.checked_add(8).ok_or(Error::Overflow)?;
            if target_end>bytes.len() { return Err(Error::Truncated("PE relocation target")); }
            if writes.iter().any(|(old,_)| *old==target) {
                return Err(Error::Invalid("duplicate PE relocation target"));
            }
            let old=u64le(bytes,target)?;
            let adjusted=if delta>=0 { old.checked_add(delta as u64) } else { old.checked_sub((-delta) as u64) }
                .ok_or(Error::Overflow)?;
            writes.push((target,adjusted));
        }
        at=at.checked_add(block_size).ok_or(Error::Overflow)?;
    }
    if at!=end { return Err(Error::Invalid("PE relocation directory size mismatch")); }
    let new_entry=if delta>=0 {
        image.description.entry_point.checked_add(delta as u64)
    } else {
        image.description.entry_point.checked_sub((-delta) as u64)
    }.ok_or(Error::Overflow)?;

    // Commit only after every relocation has been validated.
    for (target,value) in writes {
        image.bytes[target..target+8].copy_from_slice(&value.to_le_bytes());
    }
    image.description.entry_point=new_entry;
    image.description.preferred_base=actual_base;
    Ok(())
}


/// A successfully resolved PE import. The IAT slot is an RVA in the mapped image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedImport {
    pub library: String,
    pub symbol: crate::windows::ImportSymbol,
    pub iat_rva: u32,
    pub address: u64,
}

/// Resolve every PE32+ import and populate its IAT in one commit-like operation:
/// all descriptors and symbols are checked first, and no IAT slot is changed if
/// any import is malformed, unsupported, or unresolved.
pub fn resolve_pe32_plus_imports(
    image: &mut InertImage,
    exports: &crate::windows::ExportTable,
) -> Result<Vec<ResolvedImport>, Error> {
    if image.description.format != Format::Pe32Plus {
        return Err(Error::Unsupported("PE imports requested for non-PE image"));
    }
    let bytes = &image.bytes;
    if bytes.len() < 0x40 { return Err(Error::Truncated("mapped PE headers")); }
    let pe = u32le(bytes, 0x3c)? as usize;
    let coff = pe.checked_add(4).ok_or(Error::Overflow)?;
    let optional = coff.checked_add(20).ok_or(Error::Overflow)?;
    let optional_size = u16le(bytes, coff + 16)? as usize;
    if optional_size < 112 { return Err(Error::Truncated("PE32+ optional header")); }
    let optional_end = optional.checked_add(optional_size).ok_or(Error::Overflow)?;
    if optional_end > bytes.len() { return Err(Error::Truncated("PE optional header")); }
    if u16le(bytes, optional)? != 0x20b {
        return Err(Error::Unsupported("PE32 (32-bit) imports"));
    }
    let directory_count = u32le(bytes, optional + 108)?;
    if directory_count <= 1 { return Ok(Vec::new()); }
    if optional_size < 128 { return Err(Error::Truncated("PE import data directory")); }
    let import_rva = u32le(bytes, optional + 120)? as usize;
    let import_size = u32le(bytes, optional + 124)? as usize;
    if import_rva == 0 && import_size == 0 { return Ok(Vec::new()); }
    if import_rva == 0 || import_size < 20 {
        return Err(Error::Invalid("PE import directory range"));
    }
    let directory_end = import_rva.checked_add(import_size).ok_or(Error::Overflow)?;
    if directory_end > bytes.len() { return Err(Error::Truncated("PE import directory")); }

    fn read_name(bytes: &[u8], at: usize, limit: usize, what: &'static str) -> Result<String, Error> {
        if at >= bytes.len() { return Err(Error::Truncated(what)); }
        let end_limit = at.checked_add(limit).ok_or(Error::Overflow)?.min(bytes.len());
        let tail = &bytes[at..end_limit];
        let end = tail.iter().position(|b| *b == 0).ok_or(Error::Invalid(what))?;
        if end == 0 { return Err(Error::Invalid(what)); }
        core::str::from_utf8(&tail[..end]).map(str::to_owned).map_err(|_| Error::Utf8(what))
    }

    const ORDINAL_FLAG64: u64 = 1u64 << 63;
    const MAX_DESCRIPTORS: usize = 4096;
    const MAX_THUNKS_PER_LIBRARY: usize = 16_384;
    let mut resolved = Vec::new();
    let mut writes: Vec<(usize, u64)> = Vec::new();
    let mut descriptor_at = import_rva;
    let mut terminated = false;
    let mut descriptor_count = 0usize;

    while descriptor_at.checked_add(20).ok_or(Error::Overflow)? <= directory_end {
        descriptor_count += 1;
        if descriptor_count > MAX_DESCRIPTORS {
            return Err(Error::Limit("PE import descriptor count"));
        }
        let original_thunk = u32le(bytes, descriptor_at)? as usize;
        let name_rva = u32le(bytes, descriptor_at + 12)? as usize;
        let first_thunk = u32le(bytes, descriptor_at + 16)? as usize;
        if original_thunk == 0 && name_rva == 0 && first_thunk == 0 {
            terminated = true;
            break;
        }
        if name_rva == 0 || first_thunk == 0 {
            return Err(Error::Invalid("incomplete PE import descriptor"));
        }
        let library = read_name(bytes, name_rva, 4096, "PE import library name")?;
        let lookup = if original_thunk == 0 { first_thunk } else { original_thunk };
        let mut thunk_index = 0usize;
        let mut thunk_terminated = false;
        loop {
            if thunk_index >= MAX_THUNKS_PER_LIBRARY {
                return Err(Error::Limit("PE import thunk count"));
            }
            let offset = thunk_index.checked_mul(8).ok_or(Error::Overflow)?;
            let lookup_at = lookup.checked_add(offset).ok_or(Error::Overflow)?;
            let iat_at = first_thunk.checked_add(offset).ok_or(Error::Overflow)?;
            let value = u64le(bytes, lookup_at)?;
            if value == 0 {
                thunk_terminated = true;
                break;
            }
            let symbol = if value & ORDINAL_FLAG64 != 0 {
                if value & !(ORDINAL_FLAG64 | 0xffff) != 0 {
                    return Err(Error::Invalid("reserved bits in PE ordinal import"));
                }
                crate::windows::ImportSymbol::Ordinal((value & 0xffff) as u16)
            } else {
                let name_at = usize::try_from(value).map_err(|_| Error::Overflow)?;
                let _hint = u16le(bytes, name_at)?;
                crate::windows::ImportSymbol::Name(read_name(
                    bytes,
                    name_at.checked_add(2).ok_or(Error::Overflow)?,
                    crate::windows::MAX_SYMBOL_NAME,
                    "PE import-by-name symbol",
                )?)
            };
            let address = exports.resolve(&crate::windows::Import {
                library: library.clone(),
                symbol: symbol.clone(),
            })?;
            let iat_end = iat_at.checked_add(8).ok_or(Error::Overflow)?;
            if iat_end > bytes.len() {
                return Err(Error::Truncated("PE import address table"));
            }
            writes.push((iat_at, address));
            resolved.push(ResolvedImport {
                library: library.clone(),
                symbol,
                iat_rva: u32::try_from(iat_at).map_err(|_| Error::Overflow)?,
                address,
            });
            if resolved.len() > crate::windows::MAX_IMPORTS {
                return Err(Error::Limit("PE total import count"));
            }
            thunk_index += 1;
        }
        if !thunk_terminated {
            return Err(Error::Invalid("unterminated PE import thunk table"));
        }
        descriptor_at = descriptor_at.checked_add(20).ok_or(Error::Overflow)?;
    }
    if !terminated {
        return Err(Error::Invalid("unterminated PE import descriptor table"));
    }

    for (at, address) in writes {
        let end = at.checked_add(8).ok_or(Error::Overflow)?;
        image.bytes.get_mut(at..end).ok_or(Error::Truncated("PE IAT slot"))?
            .copy_from_slice(&address.to_le_bytes());
    }
    Ok(resolved)
}
