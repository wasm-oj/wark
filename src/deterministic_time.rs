use std::sync::{Arc, Mutex};
use wasmer::{
    Function, FunctionEnv, FunctionEnvMut, Imports, Memory, Memory32, Memory64, Store, WasmPtr,
};
use wasmer_wasix::WasiError;

pub(crate) struct HostFuncEnv {
    memory: Arc<Mutex<Option<Memory>>>,
}

pub(crate) fn clock_time_get_32(
    env: FunctionEnvMut<HostFuncEnv>,
    _clock_id: u32,
    _precision: u64,
    _result: WasmPtr<u64, Memory32>,
) -> Result<i32, WasiError> {
    let memory = env.data().memory.lock().unwrap().clone().unwrap();
    let view = memory.view(&env);
    let _ = _result.write(&view, 0);
    Ok(0)
}

pub(crate) fn clock_time_get_64(
    env: FunctionEnvMut<HostFuncEnv>,
    _clock_id: u32,
    _precision: u64,
    _result: WasmPtr<u64, Memory64>,
) -> Result<i32, WasiError> {
    let memory = env.data().memory.lock().unwrap().clone().unwrap();
    let view = memory.view(&env);
    let _ = _result.write(&view, 0);
    Ok(0)
}

pub(crate) fn use_deterministic_time(
    mut store: &mut Store,
    app_memory: &Arc<Mutex<Option<Memory>>>,
    imports: &mut Imports,
) {
    let env = FunctionEnv::new(
        &mut store,
        HostFuncEnv {
            memory: app_memory.clone(),
        },
    );

    imports.define(
        "wasi",
        "clock_time_get",
        Function::new_typed_with_env(&mut store, &env, clock_time_get_32),
    );
    imports.define(
        "wasi_unstable",
        "clock_time_get",
        Function::new_typed_with_env(&mut store, &env, clock_time_get_32),
    );
    imports.define(
        "wasi_snapshot_preview1",
        "clock_time_get",
        Function::new_typed_with_env(&mut store, &env, clock_time_get_32),
    );
    imports.define(
        "wasix_32v1",
        "clock_time_get",
        Function::new_typed_with_env(&mut store, &env, clock_time_get_32),
    );
    imports.define(
        "wasix_64v1",
        "clock_time_get",
        Function::new_typed_with_env(&mut store, &env, clock_time_get_64),
    );
}
