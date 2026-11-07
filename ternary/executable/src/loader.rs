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
    if image.description.format!=Format::Pe32Plus { return Err(Error::Unsupported("PE relocation requested for non-PE image")); }
    let preferred=image.description.preferred_base;
    if actual_base==preferred { return Ok(()); }
    if image.bytes.len()<0x40 { return Err(Error::Truncated("mapped PE headers")); }
    let pe=u32le(&image.bytes,0x3c)? as usize;
    let coff=pe.checked_add(4).ok_or(Error::Overflow)?;
    let opt=coff.checked_add(20).ok_or(Error::Overflow)?;
    let directory=opt.checked_add(112).ok_or(Error::Overflow)?;
    let reloc=directory.checked_add(5*8).ok_or(Error::Overflow)?;
    let rva=u32le(&image.bytes,reloc)? as usize;
    let size=u32le(&image.bytes,reloc+4)? as usize;
    if rva==0 || size==0 { return Err(Error::Unsupported("PE image has no base relocation directory")); }
    let end=rva.checked_add(size).ok_or(Error::Overflow)?;
    if end>image.bytes.len() { return Err(Error::Truncated("PE base relocation directory")); }
    let delta=actual_base as i128-preferred as i128;
    let mut at=rva;
    let mut count=0usize;
    while at<end {
        if end-at<8 { return Err(Error::Invalid("partial PE relocation block")); }
        let page=u32le(&image.bytes,at)? as usize;
        let block_size=u32le(&image.bytes,at+4)? as usize;
        if block_size<8 || block_size%2!=0 || at.checked_add(block_size).ok_or(Error::Overflow)?>end {
            return Err(Error::Invalid("invalid PE relocation block size"));
        }
        let entries=(block_size-8)/2;
        for i in 0..entries {
            count=count.checked_add(1).ok_or(Error::Overflow)?;
            if count>1_000_000 { return Err(Error::Limit("PE relocation count")); }
            let item=u16le(&image.bytes,at+8+i*2)?;
            let kind=item>>12;
            let offset=(item&0x0fff) as usize;
            if kind==0 { continue; }
            if kind!=10 { return Err(Error::Unsupported("PE relocation type outside DIR64 subset")); }
            let target=page.checked_add(offset).ok_or(Error::Overflow)?;
            let old=u64le(&image.bytes,target)?;
            let adjusted=if delta>=0 { old.checked_add(delta as u64) } else { old.checked_sub((-delta) as u64) }
                .ok_or(Error::Overflow)?;
            let dst=image.bytes.get_mut(target..target.checked_add(8).ok_or(Error::Overflow)?)
                .ok_or(Error::Truncated("PE relocation target"))?;
            dst.copy_from_slice(&adjusted.to_le_bytes());
        }
        at=at.checked_add(block_size).ok_or(Error::Overflow)?;
    }
    if at!=end { return Err(Error::Invalid("PE relocation directory size mismatch")); }
    image.description.entry_point=if delta>=0 {
        image.description.entry_point.checked_add(delta as u64)
    } else { image.description.entry_point.checked_sub((-delta) as u64) }.ok_or(Error::Overflow)?;
    image.description.preferred_base=actual_base;
    Ok(())
}
