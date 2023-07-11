use super::{cast, Args, Value};
use crate::diag::SourceResult;
use crate::diag::StrResult;
use crate::Bytes;
use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex, MutexGuard},
};
use typst::diag::At;
use wasmi::{Caller, Engine, Func as Function, Linker, Module, Value as WasiValue};

#[derive(Debug, Clone)]
pub struct Plugin(Arc<Repr>);

#[derive(Debug)]
struct Repr {
    bytes: Bytes,
    functions: Vec<(String, Function)>,
    store: Mutex<Store>,
}

cast! {
    type Plugin : "plugin",
}

impl PartialEq for Plugin {
    fn eq(&self, other: &Self) -> bool {
        self.0.bytes == other.0.bytes
    }
}

impl std::hash::Hash for Plugin {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.bytes.hash(state);
    }
}

type Store = wasmi::Store<PersistentData>;

#[derive(Debug, Clone)]
struct PersistentData {
    result_data: String,
    arg_buffer: String,
}

impl Plugin {
    pub fn new_from_bytes(bytes: Bytes) -> StrResult<Self> {
        let engine = Engine::default();
        let data = PersistentData {
            result_data: String::default(),
            arg_buffer: String::default(),
        };
        let mut store = Store::new(&engine, data);

        let module = Module::new(&engine, bytes.as_ref())
            .map_err(|err| format!("Couldn't load module: {err}"))?;

        let mut linker = Linker::new(&engine);
        let instance = linker
            .func_wrap(
                "typst_env",
                "wasm_minimal_protocol_send_result_to_host",
                move |mut caller: Caller<PersistentData>, ptr: u32, len: u32| {
                    let memory =
                        caller.get_export("memory").unwrap().into_memory().unwrap();
                    let mut buffer = vec![0u8; len as usize];
                    memory.read(&caller, ptr as _, &mut buffer).unwrap();
                    caller.data_mut().result_data = String::from_utf8(buffer).unwrap();
                },
            )
            .unwrap()
            .func_wrap(
                "typst_env",
                "wasm_minimal_protocol_write_args_to_buffer",
                move |mut caller: Caller<PersistentData>, ptr: u32| {
                    let memory =
                        caller.get_export("memory").unwrap().into_memory().unwrap();
                    let buffer = std::mem::take(&mut caller.data_mut().arg_buffer);
                    memory.write(&mut caller, ptr as _, buffer.as_bytes()).unwrap();
                    caller.data_mut().arg_buffer = buffer;
                },
            )
            .unwrap()
            .instantiate(&mut store, &module)
            .map_err(|e| format!("{e}"))?
            .start(&mut store)
            .map_err(|e| format!("{e}"))?;

        let functions = instance
            .exports(&store)
            .filter_map(|e| {
                let name = e.name().to_owned();
                e.into_func().map(|func| (name, func))
            })
            .collect::<Vec<_>>();
        Ok(Plugin(Arc::new(Repr { bytes, functions, store: Mutex::new(store) })))
    }

    fn store(&self) -> MutexGuard<'_, wasmi::Store<PersistentData>> {
        self.0.store.lock().unwrap()
    }

    fn write(&self, args: &[&str]) {
        let mut all_args = String::new();
        for arg in args {
            all_args += arg;
        }
        self.store().data_mut().arg_buffer = all_args;
    }

    pub fn call(&self, function: &str, args: &mut Args) -> SourceResult<Value> {
        let span = args.span;
        let ty = self
            .get_function(function)
            .ok_or("plugin doesn't have the method: {function}")
            .at(span)?
            .ty(self.store().deref());
        let arg_count = ty.params().len();
        let mut str_args = vec![];
        for k in 0..arg_count {
            let arg = args
                .eat::<Value>()?
                .ok_or(format!("plugin methods takes {arg_count} args, {k} provided"))
                .at(span)?
                .cast::<String>()
                .at(span)?;
            str_args.push(arg);
        }
        let s = self
            .call_inner(
                function,
                str_args.iter().map(|x| x.as_str()).collect::<Vec<_>>().as_slice(),
            )
            .at(span)?;
        Ok(Value::Str(s.into()))
    }

    fn call_inner(&self, function: &str, args: &[&str]) -> Result<String, String> {
        self.write(args);

        let function = self
            .get_function(function)
            .ok_or(format!("Plugin doesn't have the method: {function}"))?;

        let result_args =
            args.iter().map(|a| WasiValue::I32(a.len() as _)).collect::<Vec<_>>();

        let mut code = [WasiValue::I32(2)];
        let is_err = function
            .call(self.store().deref_mut(), &result_args, &mut code)
            .is_err();
        let code = if is_err {
            WasiValue::I32(2)
        } else {
            code.first().cloned().unwrap_or(WasiValue::I32(3)) // if the function returns nothing
        };

        let s = std::mem::take(&mut self.store().data_mut().result_data);

        match code {
            WasiValue::I32(0) => Ok(s),
            WasiValue::I32(1) => Err(format!(
                "plugin errored with: {:?} with code: {}",
                s,
                code.i32().unwrap()
            )),
            WasiValue::I32(2) => Err("plugin panicked".to_string()),
            _ => Err("plugin did not respect the protocol".to_string()),
        }
    }

    pub fn get_function(&self, function_name: &str) -> Option<Function> {
        let Some((_, function)) = self.0.functions.iter().find(|(s, _)| s == function_name) else {
            return None
        };
        Some(*function)
    }

    pub fn iter_functions(&self) -> impl Iterator<Item = &String> {
        self.0.functions.as_slice().iter().map(|(func_name, _)| func_name)
    }
}
