use std::fs;
use structopt::StructOpt;
use wasmer::{imports, Instance, Module, NativeFunc, Store, Function, Features};
use wasmer_compiler_singlepass::Singlepass;
use wasmer_compiler_cranelift::{Cranelift, CraneliftOptLevel};
use wasmer_compiler_llvm::{LLVM, LLVMOptLevel};
use wasmer_engine_universal::Universal;
use wasmer_engine_dylib::Dylib;
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

    /// Engine. Options are "universal" or "dylib".
    #[structopt(long, required(true))]
    engine: String,

    /// Select if the compiler should apply optimizations.
    #[structopt(long = "optimize")]
    should_optimize: bool,

    /// The path to the wasm program (.wasm).
    #[structopt(required(true))]
    program: std::path::PathBuf,
}

static CRC_ALGORITHM: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);

fn main() -> anyhow::Result<()> {
    // Get command line arguments
    let args = Cli::from_args();
    if !(args.compiler == "cranelift"
         || args.compiler == "llvm") {
         // || args.compiler == "singlepass") {  // Singlepass does not support multi-value wasm yet, which we need for loop params
        panic!("Provided compiler option must be [\"cranelift\"|\"llvm\"]");
    }
    if !(args.engine == "universal"
         || args.engine == "dylib") {
        panic!("Provided engine option must be [\"universal\"|\"dylib\"]");
    }

    // Enable multi-value wasm to use parameters with loops
    let mut compiler_features = Features::new();
    compiler_features.multi_value(true);

  
    // Instantiate the compiler and engine into a store
    let config_options = (&args.compiler[..], &args.engine[..]);
    let store = match config_options {
        ("singlepass", engine) => {
            let mut compiler = Singlepass::new();
            compiler
                .canonicalize_nans(args.should_optimize);

            match engine {
                "universal" => Store::new(&Universal::new(compiler).features(compiler_features).engine()),
                "dylib"     => Store::new(&Dylib::new(compiler).features(compiler_features).engine()),
                _ => panic!("engine did not match any known option"),
            }
        },
        
        ("cranelift", engine) => {
            let mut compiler = Cranelift::new();
            compiler
                .canonicalize_nans(args.should_optimize)
                .opt_level(
                    if args.should_optimize {
                        CraneliftOptLevel::SpeedAndSize
                    } else {
                        CraneliftOptLevel::None
                    });
            
            match engine {
                "universal" => Store::new(&Universal::new(compiler).features(compiler_features).engine()),
                "dylib"     => Store::new(&Dylib::new(compiler).features(compiler_features).engine()),
                _ => panic!("engine did not match any known option"),
            }
        },

        ("llvm", engine) => {
            let mut compiler = LLVM::new();
            compiler
                .opt_level(
                    if args.should_optimize {
                        LLVMOptLevel::Aggressive
                    } else {
                        LLVMOptLevel::None
                    })
                .canonicalize_nans(args.should_optimize);
                    
            match engine {
                "universal" => Store::new(&Universal::new(compiler).features(compiler_features).engine()),
                "dylib"     => Store::new(&Dylib::new(compiler).features(compiler_features).engine()),
                _ => panic!("engine did not match any known option"),
            }
        },

        (_, _) => panic!("compiler did not match any known option"),
    };

    // Get the wasm program to test
    let wasm_bytes = fs::read(args.program)?;
  
    // Compile the Wasm module.
    let module = Module::new(&store, wasm_bytes)?;

    // Wasmer interoperability:
    //   Arc<...>   -   In order to allow the wasm code to interact with a data-structure on the rust side of things, we have to be able
    //                  to give it a pointer to the data, while still allowing for rust to be able to still do the same thing. Multiple
    //                  references = Arc.
    //   Mutex <...>  - The crc digest is mutable. Mutex allows mutability by locking the data even with multiple references.
    //   Option <...> - The crc digest is moved (not borrowed) when finalize() is called. Option allows us to 'take' the value

    // Make a crc digest
    let shared_digest: Arc<Mutex<Option<crc::Digest<u32>>>> = Arc::new(Mutex::new(Some(CRC_ALGORITHM.digest())));

    #[derive(wasmer::WasmerEnv, Clone)]
    struct Env<'a> {
        digest: Arc<Mutex<Option<crc::Digest<'a, u32>>>>,
    }

    fn add_to_crc(env: &Env, val: i32) {
        // The i32 value gets converted to bytes: don't worry about the type mismatch with the crc
        env.digest.lock().unwrap().as_mut().unwrap().update(&val.to_le_bytes());
    }

    fn get_crc(env: &Env) -> u32 {
        let owned_digest = env.digest.lock().unwrap().take().unwrap();
        owned_digest.finalize()
    }

    // Create a function to pass to wasm
    let add_to_crc_func = Function::new_native_with_env(
        &store,
        Env { digest: shared_digest.clone() }, // These clones clone the Arc, not the underlying data
        add_to_crc
    );

    // let add_to_crc_wasm_func = Function::new_native(
    //     &store,
    //     add_to_crc_func
    // );

    
    // Create an import object with the crc function from the crc library.
    let import_object = imports! {
        "env" => {
            "addToCrc" => add_to_crc_func// as &From<wasmer::Function>,
        },
    };

    // Instantiate the Wasm module.
    let instance = Instance::new(&module, &import_object)?;

    // Get handles for the exports
    let main_func: NativeFunc<(), i32> = instance.exports.get_native_function("_main")?;
    let crc_globals_func: NativeFunc<(), ()> = instance.exports.get_native_function("_crc_globals")?;
    let memory = instance.exports.get_memory("_memory")?;

    // Call main and add the result to the crc
    let main_result: i32 = main_func.call()?;
    add_to_crc(&Env{ digest: shared_digest.clone() },main_result);

    // Call the crc globals function to have wasm add all of it's globals to the crc
    crc_globals_func.call()?;

    // Add the contents of memory to the crc
    // Get the pointer and size in bytes
    let mem_ptr: *mut i32 = memory.data_ptr() as *mut i32;
    let mem_size = (memory.data_size() / 4) as isize; // bytes to i32s
    
    for address in 0..mem_size {
        let mem_value : i32;
        unsafe { // raw memory: can't be typed by the compiler
            mem_value = *mem_ptr.offset(address) as i32;
        }
        add_to_crc(&Env { digest: shared_digest.clone() }, mem_value);
    }

    // Print the crc
    println!("{:x}", get_crc(&Env { digest: shared_digest.clone() }));

    // Signal to the shell that everything went according to plan
    Ok(())
}
