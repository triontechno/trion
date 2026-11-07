//! Bounded x86-64 instruction decoding primitives.
//! This is a conservative structural decoder, not an emulator or execution engine.
//! Unknown encodings fail closed; it never rewrites or translates guest instructions.

use crate::Error;

pub const MAX_INSTRUCTION_BYTES: usize = 15;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instruction {
    Nop, Ret, Int3, Syscall, Hlt,
    PushReg(u8), PopReg(u8),
    MovRegImm64 { register: u8, value: u64 },
    RelativeCall(i32), RelativeJump(i32), ConditionalJump { condition: u8, displacement: i32 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decoded { pub instruction: Instruction, pub length: u8 }

fn read_i32(bytes:&[u8], at:usize)->Result<i32,Error>{
    let end=at.checked_add(4).ok_or(Error::Overflow)?;
    let b=bytes.get(at..end).ok_or(Error::Truncated("x86 displacement"))?;
    Ok(i32::from_le_bytes(b.try_into().map_err(|_|Error::Truncated("x86 displacement"))?))
}
fn read_u64(bytes:&[u8], at:usize)->Result<u64,Error>{
    let end=at.checked_add(8).ok_or(Error::Overflow)?;
    let b=bytes.get(at..end).ok_or(Error::Truncated("x86 immediate"))?;
    Ok(u64::from_le_bytes(b.try_into().map_err(|_|Error::Truncated("x86 immediate"))?))
}
/// Decode only the explicit safe subset. Prefixes outside this subset are rejected.
pub fn decode(bytes:&[u8])->Result<Decoded,Error>{
    if bytes.is_empty(){return Err(Error::Truncated("x86 opcode"))}
    let b=bytes[0];
    let simple=|instruction| Ok(Decoded{instruction,length:1});
    match b {
        0x90=>simple(Instruction::Nop),
        0xc3=>simple(Instruction::Ret),
        0xcc=>simple(Instruction::Int3),
        0xf4=>simple(Instruction::Hlt),
        0x50..=0x57=>simple(Instruction::PushReg(b-0x50)),
        0x58..=0x5f=>simple(Instruction::PopReg(b-0x58)),
        0x40..=0x4f=>{
            let opcode=*bytes.get(1).ok_or(Error::Truncated("REX register opcode"))?;
            let extension=(b&1)<<3;
            match opcode {
                0x50..=0x57=>Ok(Decoded{instruction:Instruction::PushReg((opcode-0x50)|extension),length:2}),
                0x58..=0x5f=>Ok(Decoded{instruction:Instruction::PopReg((opcode-0x58)|extension),length:2}),
                _=>Err(Error::Unsupported("REX-prefixed opcode outside declared decoder subset")),
            }
        }
        0xe8=>Ok(Decoded{instruction:Instruction::RelativeCall(read_i32(bytes,1)?),length:5}),
        0xe9=>Ok(Decoded{instruction:Instruction::RelativeJump(read_i32(bytes,1)?),length:5}),
        0x0f=>{
            let second=*bytes.get(1).ok_or(Error::Truncated("x86 two-byte opcode"))?;
            match second {
                0x05=>Ok(Decoded{instruction:Instruction::Syscall,length:2}),
                0x80..=0x8f=>Ok(Decoded{instruction:Instruction::ConditionalJump{condition:second-0x80,displacement:read_i32(bytes,2)?},length:6}),
                _=>Err(Error::Unsupported("x86-64 opcode outside declared decoder subset")),
            }
        }
        0x48..=0x4f=>{
            let rex=b;
            let opcode=*bytes.get(1).ok_or(Error::Truncated("REX opcode"))?;
            if !(0xb8..=0xbf).contains(&opcode) || rex & 8 == 0 {
                return Err(Error::Unsupported("REX-prefixed opcode outside declared decoder subset"));
            }
            let reg=(opcode-0xb8) | ((rex&1)<<3);
            Ok(Decoded{instruction:Instruction::MovRegImm64{register:reg,value:read_u64(bytes,2)?},length:10})
        }
        _=>Err(Error::Unsupported("x86-64 opcode outside declared decoder subset")),
    }
}
