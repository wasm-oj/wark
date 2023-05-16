use crate::cost::{get_remaining_points, Cost, CostPoints};
use std::borrow::Cow;
use std::io::{Read, Write};
use std::sync::Arc;
use wasmer::{BaseTunables, CompilerConfig, Engine, Memory, Pages, Target};
use wasmer::{Cranelift, Instance};
use wasmer::{Module, Store};
use wasmer_types::TrapCode;
use wasmer_wasix::wasmer_wasix_types::wasi::ExitCode;
use wasmer_wasix::{wasmer_wasix_types, Pipe, WasiEnv, WasiError};

use crate::memory::LimitingTunables;

#[derive(Debug)]
pub struct RunResult {
    /// The cost of the program.
    pub cost: u64,
    /// The memory usage of the program, in MB.
    pub memory: u32,
    /// The stdout of the program.
    pub stdout: Vec<u8>,
    /// The stderr of the program.
    pub stderr: Vec<u8>,
    /// The operations counts of the program. (instruction counts, not runtime costs)
    pub operations: std::collections::HashMap<String, u64>,
}

#[derive(Debug)]
pub enum RunError {
    SpendingLimitExceeded(u64),
    MemoryLimitExceeded(u32),
    RuntimeError(String),
    CompileError(String),
    IOError(String),
}

pub(crate) fn run(
    wasm: Cow<'_, [u8]>,
    budget: u64,
    mem: u32,
    input: String,
) -> Result<RunResult, RunError> {
    let metering = Arc::new(Cost::new(budget));
    let mut compiler = Cranelift::default();
    compiler.push_middleware(metering.clone());

    let base = BaseTunables::for_target(&Target::default());
    let tunables = LimitingTunables::new(base, Pages(mem * 16));

    let mut engine: Engine = compiler.into();
    engine.set_tunables(tunables);

    let mut store = Store::new(engine);
    let module = Module::new(&store, wasm).map_err(|e| RunError::CompileError(e.to_string()))?;

    // Prepare the standard IO pipes
    let (mut stdin_sender, stdin_reader) = Pipe::channel();
    let (stdout_sender, mut stdout_reader) = Pipe::channel();
    let (stderr_sender, mut stderr_reader) = Pipe::channel();

    // Prepare the WASI sandbox environment
    let mut sandbox = WasiEnv::builder("app")
        .stdin(Box::new(stdin_reader))
        .stdout(Box::new(stdout_sender))
        .stderr(Box::new(stderr_sender))
        .finalize(&mut store)
        .map_err(|e| RunError::CompileError(e.to_string()))?;

    let imports = sandbox
        .import_object(&mut store, &module)
        .map_err(|e| RunError::CompileError(e.to_string()))?;
    let instance = Instance::new(&mut store, &module, &imports)
        .map_err(|e| RunError::CompileError(e.to_string()))?;

    sandbox
        .initialize(&mut store, instance.clone())
        .map_err(|e| RunError::CompileError(e.to_string()))?;

    // Write to the stdin
    writeln!(stdin_sender, "{}", input).map_err(|e| RunError::IOError(e.to_string()))?;

    // Run the program
    let start = instance
        .exports
        .get_function("_start")
        .map_err(|e| RunError::CompileError(e.to_string()))?;
    match start.call(&mut store, &[]) {
        Ok(_) => {}
        Err(e) => {
            if let Some(trap) = e.clone().to_trap() {
                match trap {
                    TrapCode::StackOverflow => {
                        return Err(RunError::RuntimeError("Stack overflow".to_string()))
                    }
                    TrapCode::HeapAccessOutOfBounds => {
                        return Err(RunError::RuntimeError(
                            "Heap access out of bounds".to_string(),
                        ))
                    }
                    TrapCode::HeapMisaligned => {
                        return Err(RunError::RuntimeError("Heap misaligned".to_string()))
                    }
                    TrapCode::TableAccessOutOfBounds => {
                        return Err(RunError::RuntimeError(
                            "Table access out of bounds".to_string(),
                        ))
                    }
                    TrapCode::IndirectCallToNull => {
                        return Err(RunError::RuntimeError("Indirect call to null".to_string()))
                    }
                    TrapCode::BadSignature => {
                        return Err(RunError::RuntimeError("Bad signature".to_string()))
                    }
                    TrapCode::IntegerOverflow => {
                        return Err(RunError::RuntimeError("Integer overflow".to_string()))
                    }
                    TrapCode::IntegerDivisionByZero => {
                        return Err(RunError::RuntimeError(
                            "Integer division by zero".to_string(),
                        ))
                    }
                    TrapCode::BadConversionToInteger => {
                        return Err(RunError::RuntimeError(
                            "Bad conversion to integer".to_string(),
                        ))
                    }
                    TrapCode::UnreachableCodeReached => {
                        let remaining_budget = get_remaining_points(&mut store, &instance);
                        match remaining_budget {
                            CostPoints::Remaining(_) => {
                                return Err(RunError::RuntimeError(
                                    "Unreachable code reached.".to_string(),
                                ))
                            }
                            CostPoints::Exhausted => {
                                return Err(RunError::SpendingLimitExceeded(budget))
                            }
                        };
                    }
                    TrapCode::UnalignedAtomic => {
                        return Err(RunError::RuntimeError("Unaligned atomic".to_string()))
                    }
                }
            }

            let wasi_error = e.downcast::<WasiError>();
            match wasi_error {
                Ok(wasi_error) => match wasi_error {
                    WasiError::Exit(exit) => match exit {
                        ExitCode::Errno(errno) => match errno {
                            wasmer_wasix_types::wasi::Errno::Toobig => {
                                return Err(RunError::MemoryLimitExceeded(mem));
                            }
                            _ => {
                                return Err(RunError::RuntimeError(format!(
                                    "Exited with errno {}",
                                    errno
                                )));
                            }
                        },
                        ExitCode::Other(code) => {
                            return Err(RunError::RuntimeError(format!(
                                "Exited with code [{}]",
                                code
                            )));
                        }
                    },
                    WasiError::UnknownWasiVersion => {
                        return Err(RunError::RuntimeError("Unknown WASI version".to_string()));
                    }
                },
                Err(e) => {
                    return Err(RunError::RuntimeError(e.to_string()));
                }
            }
        }
    }
    sandbox.cleanup(&mut store, None);

    // Check the instruction count
    let remaining_budget = get_remaining_points(&mut store, &instance);
    let cost = match remaining_budget {
        CostPoints::Remaining(remaining) => budget - remaining,
        CostPoints::Exhausted => unreachable!(),
    };

    // Check the memory usage
    let mut memories: Vec<Memory> = instance
        .exports
        .iter()
        .memories()
        .map(|pair| pair.1.clone())
        .collect();
    let memory = memories.pop().unwrap().ty(&store);
    let max_mem = (memory.minimum.0 + 15) / 16;
    if max_mem > mem {
        unreachable!();
    }

    // Read the stdout and stderr
    let stdout = {
        let mut buf = Vec::new();
        stdout_reader
            .read_to_end(&mut buf)
            .map_err(|e| RunError::IOError(e.to_string()))?;
        buf
    };
    let stderr = {
        let mut buf = Vec::new();
        stderr_reader
            .read_to_end(&mut buf)
            .map_err(|e| RunError::IOError(e.to_string()))?;
        buf
    };

    let operations = metering.operation_counts.lock().unwrap().clone();

    Ok(RunResult {
        cost,
        memory: max_mem,
        stdout,
        stderr,
        operations,
    })
}
