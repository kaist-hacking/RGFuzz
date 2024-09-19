use rand::RngCore;
use wasmtime::*;
use std::env;
use std::process;
use itertools::Itertools;
use anyhow::bail;
use rand::seq::SliceRandom;
use rand::{SeedableRng, rngs::StdRng};
use xxhash_rust::xxh3::xxh3_64_with_seed;

fn main() -> Result<()> {
    let args: Vec<_> = env::args().collect();
    if args.len() != 4 {
        println!("[*] {} <filename> <optlevel> <seed>", args[0]);
        process::exit(1);
    }

    let seed = args[3].parse::<u64>().unwrap();
    let mut rng = StdRng::seed_from_u64(seed);

    let mut config = Config::default();
    config.strategy(Strategy::Cranelift);
    config.wasm_threads(true);
    config.cranelift_nan_canonicalization(true);
    config.cranelift_debug_verifier(false);
    match args[2].as_str() {
        "0"=>config.cranelift_opt_level(OptLevel::None),
        "1"=>config.cranelift_opt_level(OptLevel::Speed),
        "2"=>config.cranelift_opt_level(OptLevel::SpeedAndSize),
        _=>{
            println!("[*] Invalid opt level {}", args[2]);
            process::exit(1);
        } 
    };
    
    #[cfg(target_arch="x86_64")]
    unsafe {
        // list of configs that are safe to turn off (may not be exhaustive)
        let config_turn_off_list = [
            "has_sse3", "has_ssse3",
            "has_sse41", "has_sse42",
            "has_popcnt", "has_avx",
            "has_avx2", "has_fma",
            "has_bmi1", "has_bmi2",
            "has_lzcnt",
            "has_avx512bitalg", "has_avx512dq",
            "has_avx512f", "has_avx512vl",
            "has_avx512vbmi",
        ];
        for c in config_turn_off_list {
            if rng.next_u32() % 10 == 0 { // 10% chance
                config.cranelift_flag_set(c, "false");
            }
        }
        rng = StdRng::seed_from_u64(seed);
    }
    #[cfg(target_arch="aarch64")]
    unsafe {
        // list of configs that are safe to turn off (may not be exhaustive)
        let config_turn_off_list = [
            "use_bti", "has_lse",
            "has_pauth",
            "sign_return_address",
            "sign_return_address_all",
            "sign_return_address_with_bkey",
        ];
        for c in config_turn_off_list {
            if rng.next_u32() % 10 == 0 { // 10% chance
                config.cranelift_flag_set(c, "false");
            }
        }
        rng = StdRng::seed_from_u64(seed);
    }
    #[cfg(target_arch="s390x")]
    unsafe {
        // list of configs that are safe to turn off (may not be exhaustive)
        let config_turn_off_list = [
            "has_mie2", "has_vxrs_ext2",
        ];
        for c in config_turn_off_list {
            if rng.next_u32() % 10 == 0 { // 10% chance
                config.cranelift_flag_set(c, "false");
            }
        }
        rng = StdRng::seed_from_u64(seed);
    }
    #[cfg(target_arch="riscv64")]
    unsafe {
        // list of configs that are safe to turn off (may not be exhaustive)
        let config_turn_off_list = [
            "has_zca", "has_zcd", 
            "has_zcb", "has_zbkb", 
            "has_zba", "has_zbb",
            "has_zbc", "has_zbs", 
            "has_zvl32b", "has_zvl64b", 
            "has_zvl128b", "has_zvl256b", 
            "has_zvl512b", "has_zvl1024b", 
            "has_zvl2048b", "has_zvl4096b", 
            "has_zvl8192b", "has_zvl16384b",
            "has_zvl32768b", "has_zvl65536b",

            // g option should be enabled
            // "has_m", "has_a", "has_d", "has_f", 
            // "has_zicsr", "has_zifencei", 

            // v option should be enabled for SIMD
            // "has_v", 
        ];
        for c in config_turn_off_list {
            if rng.next_u32() % 10 == 0 { // 10% chance
                config.cranelift_flag_set(c, "false");
            }
        }
        rng = StdRng::seed_from_u64(seed);
    }
    // unsafe {
    //     config.cranelift_flag_enable("has_v"); // simd
    // }

    let engine = Engine::new(&config)?;
    let module = Module::from_file(&engine, args[1].clone())?;
    let mut store = Store::new(&engine, ());

    let instance = Instance::new(&mut store, &module, &[])?;
    let main = instance.exports(&mut store)
        .find(|x| x.clone().into_func().is_some())
        .map(|x| x.into_func().unwrap())
        .expect("There is no exported function");
    let main_ty = main.ty(&store);
    let memory = instance.exports(&mut store)
        .find(|x| x.clone().into_memory().is_some())
        .map(|x| x.into_memory().unwrap());
    // instance.get_export(&mut store, "mem").unwrap().into_memory().unwrap();

    // no args
    if main_ty.params().len() == 0 {
        let params: [Val; 0] = Default::default();
        let mut results = vec![Val::I32(0); main_ty.results().len()];
        match main.call(&mut store, &params, &mut results) {
            Ok(_) => println!("{:?}", results),
            Err(_e) => println!("Error"),
        }
        if memory.is_some() {
            println!("MEMORYHASH: {}", xxh3_64_with_seed(memory.unwrap().data(&store), seed));
        }
        return Ok(())
    }

    let codegen_interesting_i32 = [
        0, 1, 2, 31, 32, 42, 63, 64, 0xff, 0xfff, 0x1000, 0x1001, 0xffff,
        -1, -2, -31, -32, -42, -63, -64, -0xff, -0xfff, -0x1000, -0x1001, -0xffff,
        -i32::MAX, i32::MIN, i32::MAX
    ];
    let codegen_interesting_i64 = [
        0i64, 1i64, 2i64, 31i64, 32i64, 42i64, 63i64, 64i64, 0xffi64, 0xfffi64, 0x1000i64, 0x1001i64, 0xffffi64,
        -1i64, -2i64, -31i64, -32i64, -42i64, -63i64, -0xffi64, -0xfffi64, -0x1000i64, -0x1001i64, -0xffffi64,
        i32::MIN as i64, i32::MAX as i64, 0x80000000i64, -i32::MAX as i64,
        0xffffffffi64, -0xffffffffi64, i64::MIN, i64::MAX, -i64::MAX
    ];
    let codegen_interesting_f32 = [
        0.0f32, -0.0f32, 1.0f32, -1.0f32, 4096.0f32, -4096.0f32,
        (1i64 << 31) as f32, (1i64 << 31 - 1) as f32, (1i64 << 63) as f32, (1i64 << 63 - 1) as f32, 
        (1i64 << 32) as f32, (1i64 << 32 - 1) as f32, (1i128 << 64) as f32, (1i128 << 64 - 1) as f32, 
        -(1i64 << 31) as f32, -(1i64 << 31 - 1) as f32, -(1i128 << 63) as f32, -(1i128 << 63 - 1) as f32, 
        -(1i64 << 32) as f32, -(1i64 << 32 - 1) as f32, -(1i128 << 64) as f32, -(1i128 << 64 - 1) as f32, 
        f32::INFINITY, -f32::INFINITY, f32::NAN
    ];
    let codegen_interesting_f64 = [
        0.0f64, -0.0f64, 1.0f64, -1.0f64, 4096.0f64, -4096.0f64,
        (1i64 << 31) as f64, (1i64 << 31 - 1) as f64, (1i64 << 63) as f64, (1i64 << 63 - 1) as f64, 
        (1i64 << 32) as f64, (1i64 << 32 - 1) as f64, (1i128 << 64) as f64, (1i128 << 64 - 1) as f64, 
        -(1i64 << 31) as f64, -(1i64 << 31 - 1) as f64, -(1i128 << 63) as f64, -(1i128 << 63 - 1) as f64, 
        -(1i64 << 32) as f64, -(1i64 << 32 - 1) as f64, -(1i128 << 64) as f64, -(1i128 << 64 - 1) as f64, 
        f64::INFINITY, -f64::INFINITY, f64::NAN
    ];
    let codegen_interesting_v128 = [
        0u128,
        0xfffefdfccccdcecf807f7e7d00010203u128,
        0x00010203cccdcecf807f7e7dfffefdfcu128,
        0x40404040404040404040404040404040u128,
        0x80808080808080808080808080808080u128,
        0xccccccccccccccccccccccccccccccccu128,
        0xffffffffffffffffffffffffffffffffu128
    ];

    let mut run_interesting_i32 = codegen_interesting_i32.map(|x| Val::I32(x)).to_vec();
    let mut run_interesting_i64 = codegen_interesting_i64.map(|x| Val::I64(x)).to_vec();
    let mut run_interesting_f32 = codegen_interesting_f32.map(|x| Val::F32(x.to_bits())).to_vec();
    let mut run_interesting_f64 = codegen_interesting_f64.map(|x| Val::F64(x.to_bits())).to_vec();
    let mut run_interesting_v128 = codegen_interesting_v128.map(|x| Val::V128(x.into())).to_vec();

    // push random value (just one)
    for _ in 0..1 {
        run_interesting_i32.push(Val::I32(rng.next_u32() as i32));
        run_interesting_i64.push(Val::I64(rng.next_u64() as i64));
        run_interesting_f32.push(Val::F32(rng.next_u32()));
        run_interesting_f64.push(Val::F64(rng.next_u64()));
        run_interesting_v128.push(Val::V128(((rng.next_u64() as u128) << 64 | rng.next_u64() as u128).into()));
    }

    let mut interesting_arg_vec = Vec::new();
    for ty in main_ty.params() {
        interesting_arg_vec.push(match ty {
            ValType::I32 => run_interesting_i32.clone(),
            ValType::I64 => run_interesting_i64.clone(),
            ValType::F32 => run_interesting_f32.clone(),
            ValType::F64 => run_interesting_f64.clone(),
            ValType::V128 => run_interesting_v128.clone(),
            t => bail!("argument type {:?} unsupported", t),
        });
    }

    if interesting_arg_vec.len() > 2 { // if too many to execute
        for _ in 0..1000 {
            let mut results = vec![Val::I32(0); main_ty.results().len()];
            let values = interesting_arg_vec.iter().map(|arg_vec| arg_vec.choose(&mut rng).unwrap().clone()).collect_vec();
            // println!("{:?}", values);
            match main.call(&mut store, values.as_slice(), &mut results) {
                Ok(_) => {
                    println!("{:?}: {:?}", values, results)
                },
                // Err(e) => println!("{:?}: {:?}", values, e.source()),
                Err(_e) => println!("{:?}: Error", values),
            }
        }
    }
    else {
        let args_product = interesting_arg_vec.into_iter().multi_cartesian_product();
        for args in args_product {
            // println!("{:?}", values);
            let mut results = vec![Val::I32(0); main_ty.results().len()];
            let values = args.as_slice();
            match main.call(&mut store, values, &mut results) {
                Ok(_) => {
                    println!("{:?}: {:?}", values, results)
                },
                // Err(e) => println!("{:?}: {:?}", values, e.source()),
                Err(_e) => println!("{:?}: Error", values),
            }
        }
    }
    if memory.is_some() {
        println!("MEMORYHASH: {}", xxh3_64_with_seed(memory.unwrap().data(&store), seed));
    }

    Ok(())
}