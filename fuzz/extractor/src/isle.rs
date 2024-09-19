// Step 1.1: Extract parsed ISLE rules

use std::{
    fs,
    env,
    path::Path, path::PathBuf
};

use cranelift_isle::{error::Errors, lexer, parser, ast};
use enum_iterator::Sequence;

// Compile the given files into Rust source code.
fn parse_files<P: AsRef<Path>>(
    inputs: impl IntoIterator<Item = P>,
) -> Result<ast::Defs, Errors> {
    let lexer = lexer::Lexer::from_files(inputs)?;
    let defs = parser::parse(lexer);
    return defs;
}

fn find_isle_files_rec(dir: &Path, isle_files: &mut Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    assert!(dir.is_dir());
    for dir_entry in fs::read_dir(dir)? {
        let dir_entry_path = dir_entry?.path();
        if dir_entry_path.is_dir() {
            let _ = find_isle_files_rec(&dir_entry_path, isle_files);
        }
        else {
            let extension = dir_entry_path.extension().unwrap();
            if extension == "isle" {
                isle_files.push(dir_entry_path.clone());
            }
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Copy, Sequence)]
pub enum ISLEParseOptions {
    Opt,
    Lower,
    X64,
    ARM64,
    S390X,
    RISCV64,
    TestOpt,
    TestLower,
}

impl ISLEParseOptions {
    pub fn is_lower(&self) -> bool {
        match self {
            Self::Opt | Self::TestOpt => false,
            _ => true,
        }
    }
}

// pub fn generate_meta() -> Result<(), cranelift_codegen_meta::error::Error> {
//     let isas = cranelift_codegen_meta::isa::Isa::all();
//     let cwd = env::current_dir().unwrap().to_string_lossy().to_string();
//     let out_dir = Path::new(&cwd).join("meta");

//     let _ = create_dir(&out_dir);
//     let out_dir_str = out_dir.to_str().unwrap();
//     cranelift_codegen_meta::generate(isas, &out_dir_str, &out_dir_str)
// }

pub fn run_parse_opt(opt: ISLEParseOptions) -> Result<ast::Defs, Errors> {
    let cwd = env::current_dir().unwrap().to_string_lossy().to_string();
    // let src_out = Path::new(&cwd).join("meta");
    let src_root = Path::new(&cwd)
                    .parent().expect("Crate in wrong directory - cannot find wasmtime")
                    .parent().expect("Crate in wrong directory - cannot find wasmtime")
                    .join("targets")
                    .join("wasmtime")
                    .join("wasmtime")
                    .join("cranelift")
                    .join("codegen")
                    .join("src");
    let src_opts = src_root.join("opts");
    let src_x64 = src_root.join("isa").join("x64");
    let src_arm64 = src_root.join("isa").join("aarch64");
    let src_s390x = src_root.join("isa").join("s390x");
    let src_riscv64 = src_root.join("isa").join("riscv64");
    
    // let clif_lower_isle = src_out.join("clif_lower.isle");
    // let clif_opt_isle = src_out.join("clif_opt.isle");
    let prelude_isle = src_root.join("prelude.isle");
    let prelude_opt_isle = src_root.join("prelude_opt.isle");
    let prelude_lower_isle = src_root.join("prelude_lower.isle");

    let mut isle_files = vec![prelude_isle];
    match opt {
        ISLEParseOptions::Opt => {
            isle_files.push(prelude_opt_isle);
            // isle_files.push(clif_opt_isle);
            let _ = find_isle_files_rec(src_opts.as_path(), &mut isle_files);
            parse_files(isle_files)
        },
        ISLEParseOptions::Lower => {
            isle_files.push(prelude_lower_isle);
            // isle_files.push(clif_lower_isle);
            let _ = find_isle_files_rec(src_x64.as_path(), &mut isle_files);
            let _ = find_isle_files_rec(src_arm64.as_path(), &mut isle_files);
            let _ = find_isle_files_rec(src_s390x.as_path(), &mut isle_files);
            let _ = find_isle_files_rec(src_riscv64.as_path(), &mut isle_files);
            parse_files(isle_files)
        },
        ISLEParseOptions::X64 => {
            isle_files.push(prelude_lower_isle);
            // isle_files.push(clif_lower_isle);
            let _ = find_isle_files_rec(src_x64.as_path(), &mut isle_files);
            parse_files(isle_files)
        },
        ISLEParseOptions::ARM64 => {
            isle_files.push(prelude_lower_isle);
            // isle_files.push(clif_lower_isle);
            let _ = find_isle_files_rec(src_arm64.as_path(), &mut isle_files);
            parse_files(isle_files)
        },
        ISLEParseOptions::S390X => {
            isle_files.push(prelude_lower_isle);
            // isle_files.push(clif_lower_isle);
            let _ = find_isle_files_rec(src_s390x.as_path(), &mut isle_files);
            parse_files(isle_files)
        },
        ISLEParseOptions::RISCV64 => {
            isle_files.push(prelude_lower_isle);
            // isle_files.push(clif_lower_isle);
            let _ = find_isle_files_rec(src_riscv64.as_path(), &mut isle_files);
            parse_files(isle_files)
        },
        ISLEParseOptions::TestOpt | ISLEParseOptions::TestLower => {
            let src_test = Path::new(&cwd).join("test");
            let _ = find_isle_files_rec(src_test.as_path(), &mut isle_files);
            parse_files(isle_files)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_run_parse() {
        // generate_meta().unwrap();
        let parsed = run_parse_opt(ISLEParseOptions::Lower).unwrap();
        println!("{:#?}", parsed.defs);
    }

    #[test]
    fn test_run_parse_rules() {
        // generate_meta().unwrap();
        let parsed = run_parse_opt(ISLEParseOptions::Opt).unwrap();
        for def in parsed.defs {
            match def {
                ast::Def::Rule(rule) => println!("{:#?}", rule),
                _ => continue
            }
        }
    }
}