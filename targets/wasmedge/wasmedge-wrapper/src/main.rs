use rand::RngCore;
use wasmedge_sdk::config::CommonConfigOptions;
use wasmedge_sdk::config::CompilerConfigOptions;
use wasmedge_sdk::config::ConfigBuilder;
use wasmedge_sdk::types::Val;
use wasmedge_sdk::Compiler;
use wasmedge_sdk::CompilerOptimizationLevel;
use wasmedge_sdk::CompilerOutputFormat;
use wasmedge_sdk::LogManager;
use wasmedge_sdk::Module;
use wasmedge_sdk::ValType;
use wasmedge_sdk::VmBuilder;
use std::env;
use std::process;
use itertools::Itertools;
use anyhow::{bail, Error};
use rand::seq::SliceRandom;
use rand::{SeedableRng, rngs::StdRng};
use xxhash_rust::xxh3::xxh3_64_with_seed;

fn main() -> Result<(), Error> {
    LogManager::log_off();

    let args: Vec<_> = env::args().collect();
    if args.len() != 5 {
        println!("[*] {} <filename> <out_dir> <optlevel> <seed>", args[0]);
        process::exit(1);
    }

    let seed = args[4].parse::<u64>().unwrap();
    let mut rng = StdRng::seed_from_u64(seed);
    
    let vm = match args[3].as_str() {
        "0" => { // no aot
            let config = ConfigBuilder::new(CommonConfigOptions::default().threads(true))
                .build()?;
            let module = Module::from_file(Some(&config), args[1].clone())?;
            let vm = VmBuilder::new().with_config(config).build()?;
            vm.register_module(None, module)?
        },
        "1" => { // O0
            let config = ConfigBuilder::new(CommonConfigOptions::default().threads(true))
                .with_compiler_config(
                    CompilerConfigOptions::default()
                        .optimization_level(CompilerOptimizationLevel::O0)
                        .out_format(CompilerOutputFormat::Native))
                .build()?;
            let compiler = Compiler::new(Some(&config))?;
            let aot_file_path = compiler.compile_from_file(args[1].clone(), "aot-o0", args[2].clone())?;
            let module = Module::from_file(Some(&config), aot_file_path)?;
            let vm = VmBuilder::new().with_config(config).build()?;
            vm.register_module(None, module)?
        },
        "2" => { // O1
            let config = ConfigBuilder::new(CommonConfigOptions::default().threads(true))
                .with_compiler_config(
                    CompilerConfigOptions::default()
                        .optimization_level(CompilerOptimizationLevel::O1)
                        .out_format(CompilerOutputFormat::Native))
                .build()?;
            let compiler = Compiler::new(Some(&config))?;
            let aot_file_path = compiler.compile_from_file(args[1].clone(), "aot-o1", args[2].clone())?;
            let module = Module::from_file(Some(&config), aot_file_path)?;
            let vm = VmBuilder::new().with_config(config).build()?;
            vm.register_module(None, module)?
        },
        "3" => { // O2
            let config = ConfigBuilder::new(CommonConfigOptions::default().threads(true))
                .with_compiler_config(
                    CompilerConfigOptions::default()
                        .optimization_level(CompilerOptimizationLevel::O2)
                        .out_format(CompilerOutputFormat::Native))
                .build()?;
            let compiler = Compiler::new(Some(&config))?;
            let aot_file_path = compiler.compile_from_file(args[1].clone(), "aot-o2", args[2].clone())?;
            let module = Module::from_file(Some(&config), aot_file_path)?;
            let vm = VmBuilder::new().with_config(config).build()?;
            vm.register_module(None, module)?
        },
        "4" => { // O3
            let config = ConfigBuilder::new(CommonConfigOptions::default().threads(true))
                .with_compiler_config(
                    CompilerConfigOptions::default()
                        .optimization_level(CompilerOptimizationLevel::O3)
                        .out_format(CompilerOutputFormat::Native))
                .build()?;
            let compiler = Compiler::new(Some(&config))?;
            let aot_file_path = compiler.compile_from_file(args[1].clone(), "aot-o3", args[2].clone())?;
            let module = Module::from_file(Some(&config), aot_file_path)?;
            let vm = VmBuilder::new().with_config(config).build()?;
            vm.register_module(None, module)?
        },
        "5" => { // Os
            let config = ConfigBuilder::new(CommonConfigOptions::default().threads(true))
                .with_compiler_config(
                    CompilerConfigOptions::default()
                        .optimization_level(CompilerOptimizationLevel::Os)
                        .out_format(CompilerOutputFormat::Native))
                .build()?;
            let compiler = Compiler::new(Some(&config))?;
            let aot_file_path = compiler.compile_from_file(args[1].clone(), "aot-os", args[2].clone())?;
            let module = Module::from_file(Some(&config), aot_file_path)?;
            let vm = VmBuilder::new().with_config(config).build()?;
            vm.register_module(None, module)?
        },
        "6" => { // Oz
            let config = ConfigBuilder::new(CommonConfigOptions::default().threads(true))
                .with_compiler_config(
                    CompilerConfigOptions::default()
                        .optimization_level(CompilerOptimizationLevel::Oz)
                        .out_format(CompilerOutputFormat::Native))
                .build()?;
            let compiler = Compiler::new(Some(&config))?;
            let aot_file_path = compiler.compile_from_file(args[1].clone(), "aot-oz", args[2].clone())?;
            let module = Module::from_file(Some(&config), aot_file_path)?;
            let vm = VmBuilder::new().with_config(config).build()?;
            vm.register_module(None, module)?
        },
        _ => {
            println!("[*] Invalid opt level {}", args[3]);
            process::exit(1);
        }
    };

    let instance = vm.active_module()?;
    let main = match instance.func_names() {
        Some(x) => instance.func(x[0].clone())?,
        None => bail!("There is no exported function"),
    };
    let main_ty = main.ty();
    
    let memory = match instance.memory_names() {
        Some(x) => Some(instance.memory(x[0].clone())?),
        None => None,
    };

    // no args
    if main_ty.args().is_none() || main_ty.args().unwrap().len() == 0 {
        let params: [Val; 0] = Default::default();
        match main.run(vm.executor(), params.into_iter().map(|x| x.into())) {
            Ok(results) => {
                    let mut result_str = String::new();
                    for result in results {
                        match result.ty() {
                            ValType::I32 => { result_str += result.to_i32().to_string().as_str(); },
                            ValType::I64 => { result_str += result.to_i64().to_string().as_str(); },
                            ValType::F32 => { result_str += result.to_f32().to_string().as_str(); },
                            ValType::F64 => { result_str += result.to_f64().to_string().as_str(); },
                            ValType::V128 => { result_str += result.to_v128().to_string().as_str(); },
                            ValType::FuncRef => { result_str += format!("{:?}", result.func_ref()).as_str(); },
                            ValType::ExternRef => { result_str += "externref"; },
                        }
                        result_str += ",";
                    }
                    println!("{}", result_str)
                
            },
            Err(_e) => println!("Error"),
        }
        if memory.is_some() {
            let memory_inner = memory.unwrap();
            println!("MEMORYHASH: {}", xxh3_64_with_seed(memory_inner.read(0, memory_inner.page()*65536)?.as_slice(), seed));
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
    let mut run_interesting_f32 = codegen_interesting_f32.map(|x| Val::F32(x)).to_vec();
    let mut run_interesting_f64 = codegen_interesting_f64.map(|x| Val::F64(x)).to_vec();
    let mut run_interesting_v128 = codegen_interesting_v128.map(|x| Val::V128(x as i128)).to_vec();

    // push random value (just one)
    for _ in 0..1 {
        run_interesting_i32.push(Val::I32(rng.next_u32() as i32));
        run_interesting_i64.push(Val::I64(rng.next_u64() as i64));
        run_interesting_f32.push(Val::F32(f32::from_bits(rng.next_u32())));
        run_interesting_f64.push(Val::F64(f64::from_bits(rng.next_u64())));
        run_interesting_v128.push(Val::V128(((rng.next_u64() as u128) << 64 | rng.next_u64() as u128) as i128));
    }

    let mut interesting_arg_vec = Vec::new();
    for ty in main_ty.args().unwrap() {
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
            let values = interesting_arg_vec.iter().map(|arg_vec| arg_vec.choose(&mut rng).unwrap().clone()).collect_vec();
            // println!("{:?}", values);
            match main.run(vm.executor(), values.clone().into_iter().map(|x| x.into())) {
                Ok(results) => {
                    let mut result_str = String::new();
                    for result in results {
                        match result.ty() {
                            ValType::I32 => { result_str += result.to_i32().to_string().as_str(); },
                            ValType::I64 => { result_str += result.to_i64().to_string().as_str(); },
                            ValType::F32 => { result_str += result.to_f32().to_string().as_str(); },
                            ValType::F64 => { result_str += result.to_f64().to_string().as_str(); },
                            ValType::V128 => { result_str += result.to_v128().to_string().as_str(); },
                            ValType::FuncRef => { result_str += format!("{:?}", result.func_ref()).as_str(); },
                            ValType::ExternRef => { result_str += "externref"; },
                        }
                        result_str += ",";
                    }
                    println!("{:?}: {}", values, result_str)
                },
                // Err(e) => println!("{:?}: {:?}", values, e.source()),
                Err(_e) => println!("{:?}: Error", values),
            }
            if memory.is_some() {
                let memory_inner = memory.as_ref().unwrap();
                println!("MEMORYHASH: {}", xxh3_64_with_seed(memory_inner.read(0, memory_inner.page()*65536)?.as_slice(), seed));
            }
        }
    }
    else {
        let args_product = interesting_arg_vec.into_iter().multi_cartesian_product();
        for args in args_product {
            // println!("{:?}", values);
            let values = args.clone();
            match main.run(vm.executor(), values.clone().into_iter().map(|x| x.into())) {
                Ok(results) => {
                    let mut result_str = String::new();
                    for result in results {
                        match result.ty() {
                            ValType::I32 => { result_str += result.to_i32().to_string().as_str(); },
                            ValType::I64 => { result_str += result.to_i64().to_string().as_str(); },
                            ValType::F32 => { result_str += result.to_f32().to_string().as_str(); },
                            ValType::F64 => { result_str += result.to_f64().to_string().as_str(); },
                            ValType::V128 => { result_str += result.to_v128().to_string().as_str(); },
                            ValType::FuncRef => { result_str += format!("{:?}", result.func_ref()).as_str(); },
                            ValType::ExternRef => { result_str += "externref"; },
                        }
                        result_str += ",";
                    }
                    println!("{:?}: {}", args, result_str)
                },
                // Err(e) => println!("{:?}: {:?}", values, e.source()),
                Err(_e) => println!("{:?}: Error", values),
            }
            if memory.is_some() {
                let memory_inner = memory.as_ref().unwrap();
                println!("MEMORYHASH: {}", xxh3_64_with_seed(memory_inner.read(0, memory_inner.page()*65536)?.as_slice(), seed));
            }
        }
    }
    
    if memory.is_some() {
        let memory_inner = memory.unwrap();
        println!("MEMORYHASH: {}", xxh3_64_with_seed(memory_inner.read(0, memory_inner.page()*65536)?.as_slice(), seed));
    }

    Ok(())
}