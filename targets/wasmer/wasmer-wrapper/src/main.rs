use rand::RngCore;
use wasmer::sys::{EngineBuilder, Features};
use wasmer::{CompilerConfig, CpuFeature, Cranelift, CraneliftOptLevel, Extern, Imports, Instance, Memory, Module, Singlepass, Store, Triple, Type, Value};
use wasmer_compiler_llvm::{LLVM, LLVMOptLevel};
use std::env;
use std::process;
use itertools::Itertools;
use anyhow::{bail, Error};
use rand::seq::SliceRandom;
use rand::{SeedableRng, rngs::StdRng};
use xxhash_rust::xxh3::xxh3_64_with_seed;

fn main() -> Result<(), Error> {
    let args: Vec<_> = env::args().collect();
    if args.len() != 4 {
        println!("[*] {} <filename> <optlevel> <seed>", args[0]);
        process::exit(1);
    }

    let seed = args[3].parse::<u64>().unwrap();
    let mut rng = StdRng::seed_from_u64(seed);
    
    let mut features = Features::new();
    features.multi_value(true);
    features.simd(true);
    features.threads(true);

    let mut engine = match args[2].as_str() {
        "0" => {
            let mut compiler = Cranelift::default();
            compiler.canonicalize_nans(true);
            compiler.opt_level(CraneliftOptLevel::None);

            let engine = EngineBuilder::new(compiler).set_features(Some(features));
            engine
        },
        "1" => {
            let mut compiler = Cranelift::default();
            compiler.canonicalize_nans(true);
            compiler.opt_level(CraneliftOptLevel::Speed);

            let engine = EngineBuilder::new(compiler).set_features(Some(features));
            engine
        },
        "2" => {
            let mut compiler = Cranelift::default();
            compiler.canonicalize_nans(true);
            compiler.opt_level(CraneliftOptLevel::SpeedAndSize);
            
            let engine = EngineBuilder::new(compiler).set_features(Some(features));
            engine
        },
        "3" => {
            let mut compiler = LLVM::default();
            compiler.canonicalize_nans(true);
            compiler.opt_level(LLVMOptLevel::None);
            
            let engine = EngineBuilder::new(compiler).set_features(Some(features));
            engine
        },
        "4" => {
            let mut compiler = LLVM::default();
            compiler.canonicalize_nans(true);
            compiler.opt_level(LLVMOptLevel::Less);
            
            let engine = EngineBuilder::new(compiler).set_features(Some(features));
            engine
        },
        "5" => {
            let mut compiler = LLVM::default();
            compiler.canonicalize_nans(true);
            compiler.opt_level(LLVMOptLevel::Default);
            
            let engine = EngineBuilder::new(compiler).set_features(Some(features));
            engine
        },
        "6" => {
            let mut compiler = LLVM::default();
            compiler.canonicalize_nans(true);
            compiler.opt_level(LLVMOptLevel::Aggressive);
            
            let engine = EngineBuilder::new(compiler).set_features(Some(features));
            engine
        },
        "7" => {
            let mut compiler = Singlepass::default();
            compiler.canonicalize_nans(true);

            let engine = EngineBuilder::new(compiler).set_features(Some(features));
            engine
        },
        _=>{
            println!("[*] Invalid opt level {}", args[2]);
            process::exit(1);
        } 
    };
    
    #[cfg(target_arch="x86_64")]
    {
        let triple = Triple::host();
        let mut cpu_features = CpuFeature::for_host();
        let config_turn_off_list = [
            // CpuFeature::SSE2, // this is required by wasmer cranelift
            CpuFeature::SSE3,
            CpuFeature::SSSE3,
            CpuFeature::SSE41,
            CpuFeature::SSE42,
            CpuFeature::POPCNT,
            CpuFeature::AVX,
            CpuFeature::BMI1,
            CpuFeature::BMI2,
            CpuFeature::AVX2,
            CpuFeature::AVX512DQ,
            CpuFeature::AVX512VL,
            CpuFeature::AVX512F,
            CpuFeature::LZCNT,
        ];
        for c in config_turn_off_list {
            if rng.next_u32() % 10 == 0 { // 10% chance
                cpu_features.remove(c);
            }
        }
        engine = engine.set_target(Some(wasmer::Target::new(triple.clone(), cpu_features)));

        rng = StdRng::seed_from_u64(seed);
    }

    let mut store = Store::new(engine);
    let module = Module::from_file(&store, args[1].clone())?;

    let instance = Instance::new(&mut store, &module, &Imports::new())?;
    let main_extern = instance.exports.iter().find(|x| {
            match x.1 {
                Extern::Function(_) => true,
                _ => false,
            }
        }).expect("There is no exported function").1;
    let main = match main_extern {
        Extern::Function(func) => func,
        _ => unreachable!(),
    };
    let main_ty = main.ty(&store);
    let memory_extern = instance.exports.iter().find(|x| {
            match x.1 {
                Extern::Memory(_) => true,
                _ => false,
            }
        });
    let memory = match memory_extern {
        Some((_, Extern::Memory(memory))) => Some(memory),
        None => None,
        _ => unreachable!(),
    };

    // no args
    if main_ty.params().len() == 0 {
        let params: [Value; 0] = Default::default();
        match main.call(&mut store, &params) {
            Ok(x) => println!("{:?}", x),
            Err(_e) => println!("Error"),
        }
        if memory.is_some() {
            println!("MEMORYHASH: {}", xxh3_64_with_seed(memory.unwrap().view(&mut store).copy_to_vec().unwrap().as_slice(), seed));
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

    let mut run_interesting_i32 = codegen_interesting_i32.map(|x| Value::I32(x)).to_vec();
    let mut run_interesting_i64 = codegen_interesting_i64.map(|x| Value::I64(x)).to_vec();
    let mut run_interesting_f32 = codegen_interesting_f32.map(|x| Value::F32(x)).to_vec();
    let mut run_interesting_f64 = codegen_interesting_f64.map(|x| Value::F64(x)).to_vec();
    let mut run_interesting_v128 = codegen_interesting_v128.map(|x| Value::V128(x.into())).to_vec();

    // push random value (just one)
    for _ in 0..1 {
        run_interesting_i32.push(Value::I32(rng.next_u32() as i32));
        run_interesting_i64.push(Value::I64(rng.next_u64() as i64));
        run_interesting_f32.push(Value::F32(f32::from_bits(rng.next_u32())));
        run_interesting_f64.push(Value::F64(f64::from_bits(rng.next_u64())));
        run_interesting_v128.push(Value::V128(((rng.next_u64() as u128) << 64 | rng.next_u64() as u128).into()));
    }

    let mut interesting_arg_vec = Vec::new();
    for ty in main_ty.params() {
        interesting_arg_vec.push(match ty {
            Type::I32 => run_interesting_i32.clone(),
            Type::I64 => run_interesting_i64.clone(),
            Type::F32 => run_interesting_f32.clone(),
            Type::F64 => run_interesting_f64.clone(),
            Type::V128 => run_interesting_v128.clone(),
            t => bail!("argument type {:?} unsupported", t),
        });
    }

    if interesting_arg_vec.len() > 2 { // if too many to execute
        for _ in 0..1000 {
            let values = interesting_arg_vec.iter().map(|arg_vec| arg_vec.choose(&mut rng).unwrap().clone()).collect_vec();
            // println!("{:?}", values);
            match main.call(&mut store, values.as_slice()) {
                Ok(x) => {
                    println!("{:?}: {:?}", values, x)
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
            let values = args.as_slice();
            match main.call(&mut store, values) {
                Ok(x) => {
                    println!("{:?}: {:?}", values, x)
                },
                // Err(e) => println!("{:?}: {:?}", values, e.source()),
                Err(_e) => println!("{:?}: Error", values),
            }
        }
    }
    
    if memory.is_some() {
        println!("MEMORYHASH: {}", xxh3_64_with_seed(memory.unwrap().view(&mut store).copy_to_vec().unwrap().as_slice(), seed));
    }

    Ok(())
}