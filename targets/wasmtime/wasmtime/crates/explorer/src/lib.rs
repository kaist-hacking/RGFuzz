use anyhow::Result;
use capstone::arch::BuildsCapstone;
use serde_derive::Serialize;
use std::{io::Write, str::FromStr};

pub fn generate(
    config: &wasmtime::Config,
    target: Option<&str>,
    wasm: &[u8],
    dest: &mut dyn Write,
) -> Result<()> {
    let target = match target {
        None => target_lexicon::Triple::host(),
        Some(target) => target_lexicon::Triple::from_str(target)?,
    };

    let wat = annotate_wat(wasm)?;
    let wat_json = serde_json::to_string(&wat)?;
    let asm = annotate_asm(config, &target, wasm)?;
    let asm_json = serde_json::to_string(&asm)?;

    let index_css = include_str!("./index.css");
    let index_js = include_str!("./index.js");

    write!(
        dest,
        r#"
<!DOCTYPE html>
<html>
  <head>
    <title>Wasmtime Compiler Explorer</title>
    <style>
      {index_css}
    </style>
  </head>
  <body class="hbox">
    <pre id="wat"></pre>
    <div id="asm"></div>
    <script>
      window.WAT = {wat_json};
      window.ASM = {asm_json};
    </script>
    <script>
      {index_js}
    </script>
  </body>
</html>
        "#
    )?;
    Ok(())
}

#[derive(Serialize, Clone, Copy, Debug)]
struct WasmOffset(u32);

#[derive(Serialize, Debug)]
struct AnnotatedWat {
    chunks: Vec<AnnotatedWatChunk>,
}

#[derive(Serialize, Debug)]
struct AnnotatedWatChunk {
    wasm_offset: Option<WasmOffset>,
    wat: String,
}

fn annotate_wat(wasm: &[u8]) -> Result<AnnotatedWat> {
    let mut printer = wasmprinter::Printer::new();
    let chunks = printer
        .offsets_and_lines(wasm)?
        .map(|(offset, wat)| AnnotatedWatChunk {
            wasm_offset: offset.map(|o| WasmOffset(u32::try_from(o).unwrap())),
            wat: wat.to_string(),
        })
        .collect();
    Ok(AnnotatedWat { chunks })
}

#[derive(Serialize, Debug)]
struct AnnotatedAsm {
    functions: Vec<AnnotatedFunction>,
}

#[derive(Serialize, Debug)]
struct AnnotatedFunction {
    instructions: Vec<AnnotatedInstruction>,
}

#[derive(Serialize, Debug)]
struct AnnotatedInstruction {
    wasm_offset: Option<WasmOffset>,
    address: u32,
    bytes: Vec<u8>,
    mnemonic: Option<String>,
    operands: Option<String>,
}

fn annotate_asm(
    config: &wasmtime::Config,
    target: &target_lexicon::Triple,
    wasm: &[u8],
) -> Result<AnnotatedAsm> {
    let engine = wasmtime::Engine::new(config)?;
    let module = wasmtime::Module::new(&engine, wasm)?;

    let text = module.text();
    let address_map: Vec<_> = module
        .address_map()
        .ok_or_else(|| anyhow::anyhow!("address maps must be enabled in the config"))?
        .collect();

    let mut address_map_iter = address_map.into_iter().peekable();
    let mut current_entry = address_map_iter.next();
    let mut wasm_offset_for_address = |start: usize, address: u32| -> Option<WasmOffset> {
        // Consume any entries that happened before the current function for the
        // first instruction.
        while current_entry.map_or(false, |cur| cur.0 < start) {
            current_entry = address_map_iter.next();
        }

        // Next advance the address map up to the current `address` specified,
        // including it.
        while address_map_iter.peek().map_or(false, |next_entry| {
            u32::try_from(next_entry.0).unwrap() <= address
        }) {
            current_entry = address_map_iter.next();
        }
        current_entry.and_then(|entry| entry.1.map(WasmOffset))
    };

    let functions = module
        .function_locations()
        .into_iter()
        .map(|(start, len)| {
            let body = &text[start..][..len];

            let mut cs = match target.architecture {
                target_lexicon::Architecture::Aarch64(_) => capstone::Capstone::new()
                    .arm64()
                    .mode(capstone::arch::arm64::ArchMode::Arm)
                    .build()
                    .map_err(|e| anyhow::anyhow!("{e}"))?,
                target_lexicon::Architecture::Riscv64(_) => capstone::Capstone::new()
                    .riscv()
                    .mode(capstone::arch::riscv::ArchMode::RiscV64)
                    .build()
                    .map_err(|e| anyhow::anyhow!("{e}"))?,
                target_lexicon::Architecture::S390x => capstone::Capstone::new()
                    .sysz()
                    .mode(capstone::arch::sysz::ArchMode::Default)
                    .build()
                    .map_err(|e| anyhow::anyhow!("{e}"))?,
                target_lexicon::Architecture::X86_64 => capstone::Capstone::new()
                    .x86()
                    .mode(capstone::arch::x86::ArchMode::Mode64)
                    .build()
                    .map_err(|e| anyhow::anyhow!("{e}"))?,
                _ => anyhow::bail!("Unsupported target: {target}"),
            };

            // This tells capstone to skip over anything that looks like data,
            // such as inline constant pools and things like that. This also
            // additionally is required to skip over trapping instructions on
            // AArch64.
            cs.set_skipdata(true).unwrap();

            let instructions = cs
                .disasm_all(body, start as u64)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let instructions = instructions
                .iter()
                .map(|inst| {
                    let address = u32::try_from(inst.address()).unwrap();
                    let wasm_offset = wasm_offset_for_address(start, address);
                    Ok(AnnotatedInstruction {
                        wasm_offset,
                        address,
                        bytes: inst.bytes().to_vec(),
                        mnemonic: inst.mnemonic().map(ToString::to_string),
                        operands: inst.op_str().map(ToString::to_string),
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(AnnotatedFunction { instructions })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(AnnotatedAsm { functions })
}
