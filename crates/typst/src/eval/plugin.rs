use super::{cast, Args, Bytes, Value};
use crate::diag::{SourceResult, StrResult};
use ecow::EcoString;
use std::sync::{Arc, Mutex, MutexGuard};
use typst::diag::At;
use wasmi::{
    AsContext, AsContextMut, Caller, Engine, Func as Function, Linker, Module,
    Value as WasiValue,
};

/// Plugin loaded from WebAssembly code, cheap to clone and hash.
///
/// It can run external code compiled using [this protocol](../../../../docs/dev/plugins.md).
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
    result_data: Vec<u8>,
    arguments: Vec<Bytes>,
}

impl Plugin {
    /// creates a new [Plugin] instance.
    pub fn new_from_bytes(bytes: impl AsRef<[u8]>) -> StrResult<Self> {
        let engine = Engine::default();
        let data = PersistentData { result_data: Vec::new(), arguments: Vec::new() };
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
                    let mut buffer = std::mem::take(&mut caller.data_mut().result_data);
                    buffer.resize(len as usize, 0);
                    memory.read(&caller, ptr as _, &mut buffer).unwrap();
                    caller.data_mut().result_data = buffer;
                },
            )
            .unwrap()
            .func_wrap(
                "typst_env",
                "wasm_minimal_protocol_write_args_to_buffer",
                move |mut caller: Caller<PersistentData>, ptr: u32| {
                    let memory =
                        caller.get_export("memory").unwrap().into_memory().unwrap();
                    let arguments = std::mem::take(&mut caller.data_mut().arguments);
                    let mut offset = ptr as usize;
                    for arg in arguments {
                        memory.write(&mut caller, offset, arg.as_slice()).unwrap();
                        offset += arg.as_slice().len();
                    }
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
        Ok(Plugin(Arc::new(Repr {
            bytes: bytes.as_ref().into(),
            functions,
            store: Mutex::new(store),
        })))
    }

    fn store(&self) -> MutexGuard<'_, Store> {
        self.0.store.lock().unwrap()
    }

    /// Call a function defined in the plugin under `function_name`.
    ///
    /// # Errors
    /// - if the plugin doesn't contain the function
    /// - if the number of argument isn't correct
    pub fn call(&self, function_name: &str, args: &mut Args) -> SourceResult<Value> {
        let span = args.span;
        let ty = self
            .get_function(function_name)
            .ok_or(format!("Plugin doesn't have the method: {function_name}"))
            .at(span)?
            .ty(self.store().as_context());

        let arg_count = ty.params().len();
        let mut byte_args = vec![];
        for k in 0..arg_count {
            let arg = args
                .eat::<Value>()?
                .ok_or(format!("plugin methods takes {arg_count} args, {k} provided"))
                .at(span)?
                .cast::<Bytes>()
                .at(span)?;
            byte_args.push(arg);
        }
        let s = self.call_inner(function_name, byte_args).at(span)?;
        Ok(Value::Bytes(s.into()))
    }

    fn call_inner(&self, function_name: &str, args: Vec<Bytes>) -> StrResult<Vec<u8>> {
        self.store().data_mut().arguments = args;
        let function = self.get_function(function_name).unwrap(); // checked in call

        let result_args = self
            .store()
            .data_mut()
            .arguments
            .iter()
            .map(|a| WasiValue::I32(a.len() as _))
            .collect::<Vec<_>>();

        let mut code = [WasiValue::I32(2)];
        let is_err = function
            .call(&mut self.store().as_context_mut(), &result_args, &mut code)
            .is_err();
        let code = if is_err {
            WasiValue::I32(2)
        } else {
            code.first().cloned().unwrap_or(WasiValue::I32(3)) // if the function returns nothing
        };

        let s = std::mem::take(&mut self.store().data_mut().result_data);

        match code {
            WasiValue::I32(0) => Ok(s),
            WasiValue::I32(1) => Err(match String::from_utf8(s) {
                Ok(err) => format!("plugin errored with: '{}'", err,).into(),
                Err(_) => {
                    EcoString::from("plugin errored and did not return valid UTF-8")
                }
            }),
            WasiValue::I32(2) => Err("plugin panicked".into()),
            _ => Err("plugin did not respect the protocol".into()),
        }
    }

    /// gets the function register under `function_name` if it exists.
    pub fn get_function(&self, function_name: &str) -> Option<Function> {
        let Some((_, function)) = self.0.functions.iter().find(|(s, _)| s == function_name) else {
            return None
        };
        Some(*function)
    }

    /// return an iterator of all the function names contained by the plugin.
    pub fn iter_functions(&self) -> impl Iterator<Item = &String> {
        self.0.functions.as_slice().iter().map(|(func_name, _)| func_name)
    }
}
