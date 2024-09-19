use std::fs;
use structopt::StructOpt;
use wasmer::{imports, Instance, Module, TypedFunction, Store, EngineBuilder, Features, Function, FunctionEnvMut, FunctionEnv};
use wasmer_compiler_singlepass::Singlepass;
use wasmer_compiler_cranelift::{Cranelift, CraneliftOptLevel};
use wasmer_compiler_llvm::{LLVM, LLVMOptLevel};
use wasmer::CompilerConfig;
use crc::{Crc, CRC_32_ISO_HDLC};
use std::sync::{Arc, Mutex};


/// Run a wasm program using Wasmer
#[derive(StructOpt)]
#[structopt(name = "load-wasmer", about = "Uses Wasmer to load and execute a WebAssembly program.")]

struct Cli {
    /// Compiler. Options are "singlepass", "cranelift", or "llvm".
    #[structopt(long, required(true))]
    compiler: String,

    /// Select if the compiler should apply optimizations.
    #[structopt(long = "optimize")]
    should_optimize: bool,

    /// The path to the wasm program (.wasm).
    #[structopt(required(true))]
    program: std::path::PathBuf,
}

static CRC_ALGORITHM: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

#[derive(Clone)]
struct MyEnv<'a: 'static> {
    //Compiler warning here. If the lifetime is simply 'static, there's an error
    digest: Arc<Mutex<Option<crc::Digest<'a, u32>>>>,
}

fn main() -> anyhow::Result<()> {
    // Get command line arguments
    let args = Cli::from_args();
    if !(args.compiler == "cranelift"
        || args.compiler == "llvm"
        || args.compiler == "singlepass") {  // Singlepass does not support multi-value wasm yet, which we need for loop params
        panic!("Provided compiler option must be [\"singlepass\"|\"cranelift\"|\"llvm\"]");
    }

    // Enable multi-value wasm to use parameters with loops
    let mut features = Features::new();
    features.multi_value(true);


    // Instantiate the compiler and engine into a store
    let config_options = &args.compiler[..];
    let mut store = match config_options {
        "singlepass" => {
            let mut compiler = Singlepass::default();
            compiler.canonicalize_nans(true);

            let engine = EngineBuilder::new(compiler).set_features(Some(features));
            Store::new(engine)
        },

        "cranelift" => {
            let mut compiler = Cranelift::default();
            compiler
                .canonicalize_nans(args.should_optimize)
                .opt_level(
                    if args.should_optimize {
                        CraneliftOptLevel::SpeedAndSize
                    } else {
                        CraneliftOptLevel::None
                    });

            let engine = EngineBuilder::new(compiler).set_features(Some(features));
            Store::new(engine)
        },

        "llvm" => {
            let mut compiler = LLVM::default();
            compiler.canonicalize_nans(args.should_optimize);
            compiler.opt_level(
                    if args.should_optimize {
                        LLVMOptLevel::Aggressive
                    } else {
                        LLVMOptLevel::None
                    });

            let engine = EngineBuilder::new(compiler).set_features(Some(features));
            Store::new(engine)
        },

        _ => panic!("compiler did not match any known option"),
    };

    // Get the wasm program to test
    let wasm_bytes = fs::read(args.program)?;

    // Compile the Wasm module.
    let module = Module::new(&mut store, wasm_bytes)?;

    // Wasmer interoperability:
    //   Arc<...>   -   In order to allow the wasm code to interact with a data-structure on the rust side of things, we have to be able
    //                  to give it a pointer to the data, while still allowing for rust to be able to still do the same thing. Multiple
    //                  references = Arc.
    //   Mutex <...>  - The crc digest is mutable. Mutex allows mutability by locking the data even with multiple references.
    //   Option <...> - The crc digest is moved (not borrowed) when finalize() is called. Option allows us to 'take' the value

    // Make a crc digest
    let shared_digest: Arc<Mutex<Option<crc::Digest<u32>>>> = Arc::new(Mutex::new(Some(CRC_ALGORITHM.digest())));

    fn add_to_crc(env: FunctionEnvMut<MyEnv>, val: i32) {
        // The i32 value gets converted to bytes: don't worry about the type mismatch with the crc
        let mut digest_guard = env.data().digest.lock().unwrap();
        let digest_ref = digest_guard.as_mut().unwrap();
        // no clue why this works and not the one-liner... Lifetimes probably
        // let digest_ref = env.data().digest.lock().unwrap().as_mut().unwrap();
        digest_ref.update(&val.to_le_bytes())
    }
    // This clones will clone the Arc, not the underlying data
    let env = FunctionEnv::new(
        &mut store,
        MyEnv {
            digest: shared_digest.clone(),
        },
    );

    // Create an import object with the crc function from the crc library.
    let import_object = imports! {
        "env" => {
            // "addToCrc" => add_to_crc_func// as &From<wasmer::Function>,
            "addToCrc" => Function::new_typed_with_env(&mut store, &env, add_to_crc)
        },
    };

    // Instantiate the Wasm module.
    let instance = Instance::new(&mut store, &module, &import_object)?;

    // Get handles for the exports
    let main_func: TypedFunction<(), i32> = instance.exports.get_typed_function(&store, "_main")?;
    let crc_globals_func: TypedFunction<(), ()> = instance.exports.get_typed_function(&store, "_crc_globals")?;
    let memory = instance.exports.get_memory("_memory")?;

    // Call main and add the result to the crc
    let main_result: i32 = main_func.call(&mut store)?;

    shared_digest.lock().unwrap().as_mut().unwrap().update(&main_result.to_le_bytes());

    // Call the crc globals function to have wasm add all of it's globals to the crc
    crc_globals_func.call(&mut store)?;

    // Add the contents of memory to the crc
    let memory_view = memory.view(&store);
    let mem_size : u64 = memory_view.data_size() as u64; // bytes to i32s

    for address in (0..mem_size).step_by(4) {
        let mut mem_value_buf : [u8; 4] = [0,0,0,0];
        memory_view.read(address, &mut mem_value_buf)?;
        let mem_value = i32::from_le_bytes(mem_value_buf);

        shared_digest.lock().unwrap().as_mut().unwrap().update(&mem_value.to_le_bytes());
    }

    // Print the crc
    let result = shared_digest.lock().unwrap().take().unwrap().finalize();
    println!("{:x}", result);

    // Signal to the shell that everything went according to plan
    Ok(())
}
