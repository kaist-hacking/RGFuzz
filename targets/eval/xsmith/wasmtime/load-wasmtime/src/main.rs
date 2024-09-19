use std::fs;
use structopt::StructOpt;
use crc::{Crc, CRC_32_ISO_HDLC};
use std::sync::{Arc, Mutex};
use wasmtime::{Caller, Engine, Module, Store, Instance, Func, Config, OptLevel};


/// Run a wasm program using Wasmer
#[derive(StructOpt)]
#[structopt(name = "load-wasmtime", about = "Uses Wasmtime to load and execute a WebAssembly program.")]
struct Cli {
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

    // Get the wasm program to test
    let wasm_bytes = fs::read(args.program)?;

    // Compile the Wasm module.
    let engine =
        if args.should_optimize {
            Engine::new(Config::new().cranelift_opt_level(OptLevel::SpeedAndSize))?
        } else {
            Engine::new(Config::new().cranelift_opt_level(OptLevel::None))?
        };

    let module = Module::new(&engine, wasm_bytes)?;


    // Wasmer interoperability:
    //   Arc<...>   -   In order to allow the wasm code to interact with a data-structure on the rust side of things, we have to be able
    //                  to give it a pointer to the data, while still allowing for rust to be able to still do the same thing. Multiple
    //                  references = Arc.
    //   Mutex <...>  - The crc digest is mutable. Mutex allows mutability by locking the data even with multiple references.
    //   Option <...> - The crc digest is moved (not borrowed) when finalize() is called. Option allows us to 'take' the value

    // For wasmtime, it's most likely that not all of this is required, since the store is passed as mutable in a lot of the calls.
    // However, I adapted the code as closely as possible from the wasmer runtime, and it seems to work.

    // Make a crc digest
    let shared_digest: Arc<Mutex<Option<crc::Digest<u32>>>> = Arc::new(Mutex::new(Some(CRC_ALGORITHM.digest())));

    struct Env<'a> {
        digest: Arc<Mutex<Option<crc::Digest<'a, u32>>>>,
    }

    fn add_to_crc(env: &Env, val: i32) {
        env.digest.lock().unwrap().as_mut().unwrap().update(&val.to_le_bytes());
    }

    fn get_crc(env: &Env) -> u32 {
        let owned_digest = env.digest.lock().unwrap().take().unwrap();
        owned_digest.finalize()
    }

    let mut store = Store::new(
        &engine,
        Env { digest: shared_digest.clone() }
    );


    let add_to_crc_func = Func::wrap(&mut store, |caller: Caller<'_, Env>, val: i32| {
        add_to_crc(caller.data(), val);
    });


    let import_object = [add_to_crc_func.into()];

    // Instantiate the Wasm module.
    let instance = Instance::new(&mut store, &module, &import_object)?;

    // Get handles for the exports
    let main_func = instance.get_typed_func::<(), i32>(&mut store, "_main")?;
    let crc_globals_func = instance.get_typed_func::<(), ()>(&mut store, "_crc_globals")?;
    let memory = instance.get_memory(&mut store, "_memory").unwrap();

    // Call main and add the result to the crc
    let main_result: i32 = main_func.call(&mut store, ())?;
    add_to_crc(&Env{ digest: shared_digest.clone() },main_result);

    // Call the crc globals function to have wasm add all of it's globals to the crc
    crc_globals_func.call(&mut store, ())?;

    // Add the contents of memory to the crc
    // Get the pointer and size in bytes
    let mem_ptr: *mut i32 = memory.data_ptr(&store) as *mut i32;
    let mem_size = (memory.data_size(&store) / 4) as isize; //bytes to i32s

    for address in 0..mem_size {
        let mem_value: i32;
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
