use memmap::MmapOptions;
use reader_writer::Reader;
use snafu::{ensure, ResultExt, Snafu};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::num::Wrapping;
use std::ops::Add;
use std::path::{Path, PathBuf};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Could not open file {}: {}", filename.display(), source))]
    OpenFile {
        filename: PathBuf,
        source: std::io::Error,
    },
    #[snafu(display("Could not write to file {}: {}", filename.display(), source))]
    WriteFile {
        filename: PathBuf,
        source: std::io::Error,
    },
    #[snafu(display("Unresolved symbol: {}", symbol_name))]
    UnresolvedSymbol { symbol_name: String },
    #[snafu(display("Invalid patch: {}", patch))]
    InvalidPatch { patch: String },
}

#[derive(Debug)]
struct FunctionCallPatchDef {
    orig_sym: String,
    orig_sym_addr: u32,
    new_sym: String,
}

#[derive(Debug)]
enum FunctionCallPatchType {
    Addr32,
    Rel24,
}

#[derive(Debug)]
struct FunctionCallPatch {
    address: u32,
    typ: FunctionCallPatchType,
    new_symbol: String,
}

type Result<T, E = Error> = std::result::Result<T, E>;

pub fn generate_patches<'a>(
    extern_sym_table: &HashMap<String, u32>,
    dol_file_name: impl AsRef<Path>,
    output_file_name: impl AsRef<Path>,
    patch_defs: impl Iterator<Item = impl AsRef<str>>,
) -> Result<()> {
    let patch_defs = parse_patches(extern_sym_table, patch_defs)?;

    let dol_file_name = dol_file_name.as_ref();
    let f = File::open(dol_file_name).with_context(|| OpenFile {
        filename: dol_file_name.to_path_buf(),
    })?;
    let mmap = unsafe {
        MmapOptions::new().map(&f).with_context(|| OpenFile {
            filename: dol_file_name.to_path_buf(),
        })?
    };
    let mut reader = Reader::new(mmap.as_ref());
    let dol: structs::Dol = reader.read(());

    let mut patches: Vec<FunctionCallPatch> = Vec::new();

    for patch_def in patch_defs {
        println!(
            "Patching calls from {} (0x{:08x}) to {}",
            patch_def.orig_sym, patch_def.orig_sym_addr, patch_def.new_sym
        );
        // Text section patches
        for seg in dol.text_segments.iter() {
            let loadaddr = &seg.load_addr;
            let opcode_bx = 18;
            let contents = &seg.contents;
            for offset in (0..contents.len()).step_by(4) {
                // TODO: find out if there's a better way of doing this in reader_writer
                // unwrap is safe because we check the addr above
                let instruction = (contents.get(offset + 3).unwrap().into_owned() as i32)
                    | ((contents.get(offset + 2).unwrap().into_owned() as i32) << 8)
                    | ((contents.get(offset + 1).unwrap().into_owned() as i32) << 16)
                    | ((contents.get(offset + 0).unwrap().into_owned() as i32) << 24);
                let addr = offset as u32 + seg.load_addr;
                let opcode = (instruction >> 26) & 0x3F;
                if opcode == opcode_bx {
                    let li = extend_sign_bit(instruction & 0x3FFFFFC, 24);
                    let aa = (instruction >> 1) & 0x1;
                    let target = if aa == 0 {
                        (Wrapping(li as u32) + Wrapping(addr)).0
                    } else {
                        li as u32
                    };
                    if target == patch_def.orig_sym_addr {
                        println!("Found call to {} at {:08x}", patch_def.orig_sym, addr);
                        patches.push(FunctionCallPatch {
                            address: addr,
                            typ: FunctionCallPatchType::Rel24,
                            new_symbol: patch_def.new_sym.clone(),
                        });
                    }
                }
            }
        }

        // Data section patches
        for seg in dol.data_segments.iter() {
            let loadaddr = &seg.load_addr;
            let opcode_bx = 18;
            let contents = &seg.contents;
            for offset in (0..contents.len()).step_by(4) {
                // TODO: find out if there's a better way of doing this in reader_writer
                // unwrap is safe because we check the addr above
                let value = (contents.get(offset + 3).unwrap().into_owned() as u32)
                    | ((contents.get(offset + 2).unwrap().into_owned() as u32) << 8)
                    | ((contents.get(offset + 1).unwrap().into_owned() as u32) << 16)
                    | ((contents.get(offset + 0).unwrap().into_owned() as u32) << 24);
                let addr = offset as u32 + seg.load_addr;

                if value == patch_def.orig_sym_addr {
                    println!("Found data ref to {} at {:08x}", patch_def.orig_sym, addr);
                    patches.push(FunctionCallPatch {
                        address: addr,
                        typ: FunctionCallPatchType::Addr32,
                        new_symbol: patch_def.new_sym.clone(),
                    });
                }
            }
        }
    }

    for patch in patches {
        println!(
            "Patch: {:08x} -> {} ({:?})",
            patch.address, patch.new_symbol, patch.typ
        )
    }

    return Ok(());
}

fn parse_patches(
    extern_sym_table: &HashMap<String, u32>,
    patches: impl Iterator<Item = impl AsRef<str>>,
) -> Result<Vec<FunctionCallPatchDef>> {
    return patches
        .map(|patch| {
            let split: Vec<&str> = patch.as_ref().split(' ').collect();
            ensure!(
                split.len() == 2,
                InvalidPatch {
                    patch: patch.as_ref()
                }
            );
            let orig_symbol = split[0];
            let target_symbol = split[1];
            ensure!(
                extern_sym_table.contains_key(orig_symbol),
                UnresolvedSymbol {
                    symbol_name: orig_symbol
                }
            );
            let orig_addr = extern_sym_table[orig_symbol];
            Ok(FunctionCallPatchDef {
                orig_sym: String::from(orig_symbol),
                orig_sym_addr: orig_addr,
                new_sym: String::from(target_symbol),
            })
        })
        .collect();
}

fn extend_sign_bit(value: i32, num_bits: i32) -> i32 {
    let sign_bit = 1 << (num_bits - 1);
    return (value & (sign_bit - 1)) - (value & sign_bit);
}
