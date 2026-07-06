use crate::error::{DecodeError, Result};
use crate::model::{CanopyArchive, RuleSet};
use crate::page;
use crate::reader::Reader;
use crate::store::ScratchStack;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    PushI8,
    PushU16,
    Add,
    Sub,
    Mul,
    Div,
    LoadPageEnergy,
    LoadDeviceCount,
    Remember,
    Compact,
    SavedDelta,
    JumpIfZero,
    Halt,
}

impl Opcode {
    pub fn from_byte(byte: u8) -> Result<Self> {
        match byte {
            0x01 => Ok(Self::PushI8),
            0x02 => Ok(Self::PushU16),
            0x10 => Ok(Self::Add),
            0x11 => Ok(Self::Sub),
            0x12 => Ok(Self::Mul),
            0x13 => Ok(Self::Div),
            0x20 => Ok(Self::LoadPageEnergy),
            0x21 => Ok(Self::LoadDeviceCount),
            0x30 => Ok(Self::Remember),
            0x31 => Ok(Self::Compact),
            0x32 => Ok(Self::SavedDelta),
            0x40 => Ok(Self::JumpIfZero),
            0xff => Ok(Self::Halt),
            other => Err(DecodeError::InvalidOpcode(other)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuleProgram {
    pub name: String,
    pub code: Vec<u8>,
    pub labels: Vec<String>,
}

pub fn parse_rules_segment(data: &[u8]) -> Result<Vec<RuleSet>> {
    let mut r = Reader::new(data);
    let count = r.read_var_usize()?;
    if count > 512 {
        return Err(DecodeError::InvalidField("rule count"));
    }
    let mut out = Vec::with_capacity(count);
    for idx in 0..count {
        let name = r.read_string().unwrap_or_else(|_| format!("rule-{idx}"));
        let label_count = (r.read_u8().unwrap_or(0) as usize).min(32);
        let mut labels = Vec::with_capacity(label_count);
        for label_idx in 0..label_count {
            labels.push(
                r.read_string()
                    .unwrap_or_else(|_| format!("label-{label_idx}")),
            );
        }
        let len = r.read_var_usize()?;
        out.push(RuleSet {
            name,
            labels,
            bytecode: r.read_bytes(len)?.to_vec(),
        });
    }
    Ok(out)
}

pub fn compile_rule(set: &RuleSet) -> Result<RuleProgram> {
    let mut pc = 0usize;
    while pc < set.bytecode.len() {
        let opcode = Opcode::from_byte(set.bytecode[pc])?;
        pc += 1;
        match opcode {
            Opcode::PushI8 => pc += 1,
            Opcode::PushU16 => pc += 2,
            Opcode::JumpIfZero => pc += 1,
            _ => {}
        }
        if pc > set.bytecode.len() {
            return Err(DecodeError::Truncated {
                at: set.bytecode.len(),
                needed: pc,
                available: set.bytecode.len(),
            });
        }
    }
    Ok(RuleProgram {
        name: set.name.clone(),
        code: set.bytecode.clone(),
        labels: set.labels.clone(),
    })
}

pub fn evaluate_rules(archive: &CanopyArchive) -> Result<i64> {
    let mut total = 0i64;
    for set in &archive.rules {
        total = total.wrapping_add(execute_rule(&compile_rule(set)?, archive)?);
    }
    Ok(total)
}

pub fn execute_rule(program: &RuleProgram, archive: &CanopyArchive) -> Result<i64> {
    let mut stack = ScratchStack::new();
    let mut pc = 0usize;
    let mut fuel = 4096usize;
    while pc < program.code.len() && fuel > 0 {
        fuel -= 1;
        let opcode = Opcode::from_byte(program.code[pc])?;
        pc += 1;
        match opcode {
            Opcode::PushI8 => {
                let v = *program.code.get(pc).ok_or(DecodeError::Truncated {
                    at: pc,
                    needed: 1,
                    available: 0,
                })? as i8 as i64;
                pc += 1;
                stack.push(v);
            }
            Opcode::PushU16 => {
                let lo = *program.code.get(pc).ok_or(DecodeError::Truncated {
                    at: pc,
                    needed: 2,
                    available: 0,
                })?;
                let hi = *program.code.get(pc + 1).ok_or(DecodeError::Truncated {
                    at: pc,
                    needed: 2,
                    available: 1,
                })?;
                pc += 2;
                stack.push(u16::from_le_bytes([lo, hi]) as i64);
            }
            Opcode::Add => binop(&mut stack, |a, b| a.wrapping_add(b)),
            Opcode::Sub => binop(&mut stack, |a, b| a.wrapping_sub(b)),
            Opcode::Mul => binop(&mut stack, |a, b| a.wrapping_mul(b)),
            Opcode::Div => binop(
                &mut stack,
                |a, b| if b == 0 { a } else { a.wrapping_div(b) },
            ),
            Opcode::LoadPageEnergy => {
                let idx = stack.pop().unwrap_or(0).unsigned_abs() as usize;
                let energy = archive
                    .pages
                    .get(idx % archive.pages.len().max(1))
                    .map(page::page_energy)
                    .unwrap_or(0);
                stack.push(energy);
            }
            Opcode::LoadDeviceCount => stack.push(archive.devices().len() as i64),
            Opcode::Remember => stack.remember_top(),
            Opcode::Compact => {
                let keep = stack.pop().unwrap_or(4).unsigned_abs() as usize % 16;
                stack.compact_for_window(keep);
            }
            Opcode::SavedDelta => stack.push(stack.saved_delta()),
            Opcode::JumpIfZero => {
                let offset = *program.code.get(pc).ok_or(DecodeError::Truncated {
                    at: pc,
                    needed: 1,
                    available: 0,
                })? as i8;
                pc += 1;
                if stack.pop().unwrap_or(0) == 0 {
                    if offset.is_negative() {
                        pc = pc.saturating_sub(offset.unsigned_abs() as usize);
                    } else {
                        pc = pc.saturating_add(offset as usize).min(program.code.len());
                    }
                }
            }
            Opcode::Halt => break,
        }
    }
    Ok(stack
        .pop()
        .unwrap_or(0)
        .wrapping_add(stack.saved_delta())
        .wrapping_add(stack.len() as i64))
}

fn binop(stack: &mut ScratchStack, f: impl FnOnce(i64, i64) -> i64) {
    let b = stack.pop().unwrap_or(0);
    let a = stack.pop().unwrap_or(0);
    stack.push(f(a, b));
}
