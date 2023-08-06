use super::{cast, Args, Bytes, Value};
use crate::diag::{SourceResult, StrResult};
use ecow::EcoString;
use std::sync::{Arc, Mutex, MutexGuard};
use typst::diag::At;
use wasmi::{
    AsContext, AsContextMut, Caller, Engine, Func as Function, Linker, Memory, Module,
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
    memory: Memory,
    free_function: Function,
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
    result_ptr: u32,
    result_len: u32,
    arg_buffer: Vec<u8>,
}

/// Reference to a slice of memory returned after
/// [calling a wasm function](PluginInstance::call).
///
/// # Drop
/// On [`Drop`], this will free the slice of memory inside the plugin.
///
/// As such, this structure mutably borrows the [`PluginInstance`], which prevents
/// another function from being called.
struct ReturnedData<'a> {
    memory: Memory,
    ptr: u32,
    len: u32,
    free_function: &'a Function,
    context_mut: MutexGuard<'a, Store>,
}

impl<'a> ReturnedData<'a> {
    /// Get a reference to the returned slice of data.
    ///
    /// # Panic
    /// This may panic if the function returned an invalid `(ptr, len)` pair.
    pub fn get(&self) -> &[u8] {
        &self.memory.data(&*self.context_mut)
            [self.ptr as usize..(self.ptr + self.len) as usize]
    }
}

impl Drop for ReturnedData<'_> {
    fn drop(&mut self) {
        if self.ptr != 0 {
            self.free_function
                .call(
                    &mut *self.context_mut,
                    &[WasiValue::I32(self.ptr as _), WasiValue::I32(self.len as _)],
                    &mut [],
                )
                .unwrap();
        }
    }
}

impl Plugin {
    /// creates a new plugin.
    pub fn new_from_bytes(bytes: impl AsRef<[u8]>) -> StrResult<Self> {
        let engine = Engine::default();
        let data = PersistentData {
            arg_buffer: Vec::new(),
            result_ptr: 0,
            result_len: 0,
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
                    caller.data_mut().result_ptr = ptr;
                    caller.data_mut().result_len = len;
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
                    memory.write(&mut caller, ptr as _, &buffer).unwrap();
                    caller.data_mut().arg_buffer = buffer;
                },
            )
            .unwrap()
            .instantiate(&mut store, &module)
            .map_err(|e| format!("{e}"))?
            .start(&mut store)
            .map_err(|e| format!("{e}"))?;

        let mut free_function = None;
        let functions = instance
            .exports(&store)
            .filter_map(|e| {
                let name = e.name().to_owned();

                e.into_func().map(|func| {
                    if name == "wasm_minimal_protocol_free_byte_buffer" {
                        free_function = Some(func);
                    }
                    (name, func)
                })
            })
            .collect::<Vec<_>>();
        let free_function = free_function.ok_or(EcoString::from("Module didn't export a free function."))?;
        let memory =
            instance.get_export(&store, "memory").unwrap().into_memory().unwrap();
        Ok(Plugin(Arc::new(Repr {
            store: Mutex::new(store),
            memory,
            free_function,
            functions,
            bytes: bytes.as_ref().into(),
        })))
    }

    fn store(&self) -> MutexGuard<'_, Store> {
        self.0.store.lock().unwrap()
    }

    /// Call a function defined in the plugin under `function_name`.
    ///
    /// This will eat the number of argument it needs of type Bytes.
    ///
    /// # Errors
    /// - if the plugin doesn't contain the function
    /// - if the number of argument isn't correct
    pub fn call(&self, function_name: &str, args: &mut Args) -> SourceResult<Value> {
        let span = args.span;
        let ty = self
            .get_function(function_name)
            .ok_or("plugin doesn't have the method: {function}")
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
        let byte_args = byte_args.iter().map(|b| b.as_slice()).collect::<Vec<_>>();
        let s = self.call_inner(function_name, byte_args).at(span)?;
        Ok(Value::Bytes(s.get().into()))
    }

    fn call_inner<'a>(
        &self,
        function: &str,
        args: impl IntoIterator<Item = &'a [u8]>,
    ) -> StrResult<ReturnedData> {
        self.store().data_mut().result_ptr = 0;
        self.store().data_mut().result_len = 0;

        let mut result_args = Vec::new();
        let mut guard = self.store();
        let arg_buffer = &mut guard.data_mut().arg_buffer;
        arg_buffer.clear();
        for arg in args {
            result_args.push(WasiValue::I32(arg.len() as _));
            arg_buffer.extend_from_slice(arg);
        }
        drop(guard);

        let function = self
            .get_function(function)
            .ok_or(format!("plugin doesn't have the method: {function}"))?;

        let mut code = WasiValue::I32(2);
        let ty = function.ty(self.store().as_context());
        if ty.params().len() != result_args.len() {
            return Err("incorrect number of arguments".into());
        }

        let call_result = function.call(
            &mut self.store().as_context_mut(),
            &result_args,
            std::array::from_mut(&mut code),
        );
        let (ptr, len) = (self.store().data().result_ptr, self.store().data().result_len);
        let result = ReturnedData {
            memory: self.0.memory,
            ptr,
            len,
            free_function: &self.0.free_function,
            context_mut: self.store(),
        };

        match call_result {
            Ok(()) => {}
            Err(wasmi::Error::Trap(_)) => return Err("plugin panicked".into()),
            Err(_) => return Err("plugin did not respect the protocol".into()),
        };

        match code {
            WasiValue::I32(0) => Ok(result),
            WasiValue::I32(1) => Err(match std::str::from_utf8(result.get()) {
                Ok(err) => format!("plugin errored with: '{}'", err,).into(),
                Err(_) => {
                    EcoString::from("plugin errored and did not return valid UTF-8")
                }
            }),
            _ => Err("plugin did not respect the protocol".into()),
        }
    }

    /// get the function register under `function_name` if it exists.
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
