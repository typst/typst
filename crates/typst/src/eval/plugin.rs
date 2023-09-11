use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};

use ecow::{eco_format, EcoString};
use std::sync::{Arc, Mutex};
use wasmi::{AsContext, AsContextMut, Caller, Engine, Linker, Module};

use super::{cast, Bytes};
use crate::diag::{bail, StrResult};

/// A plugin loaded from WebAssembly code.
///
/// It can run external code conforming to its protocol.
///
/// This type is cheap to clone and hash.
#[derive(Clone)]
pub struct Plugin(Arc<Repr>);

/// The internal representation of a plugin.
struct Repr {
    /// The raw WebAssembly bytes.
    bytes: Bytes,
    /// The function defined by the WebAssembly module.
    functions: Vec<(EcoString, wasmi::Func)>,
    /// Owns all data associated with the WebAssembly module.
    store: Mutex<Store>,
}

/// Owns all data associated with the WebAssembly module.
type Store = wasmi::Store<StoreData>;

/// The persistent store data used for communication between store and host.
#[derive(Default)]
struct StoreData {
    args: Vec<Bytes>,
    output: Vec<u8>,
}

impl Plugin {
    /// Create a new plugin from raw WebAssembly bytes.
    #[comemo::memoize]
    pub fn new(bytes: Bytes) -> StrResult<Self> {
        let engine = Engine::default();
        let module = Module::new(&engine, bytes.as_slice())
            .map_err(|err| format!("failed to load WebAssembly module ({err})"))?;

        let mut linker = Linker::new(&engine);
        linker
            .func_wrap(
                "typst_env",
                "wasm_minimal_protocol_send_result_to_host",
                wasm_minimal_protocol_send_result_to_host,
            )
            .unwrap();
        linker
            .func_wrap(
                "typst_env",
                "wasm_minimal_protocol_write_args_to_buffer",
                wasm_minimal_protocol_write_args_to_buffer,
            )
            .unwrap();

        let mut store = Store::new(&engine, StoreData::default());
        let instance = linker
            .instantiate(&mut store, &module)
            .and_then(|pre_instance| pre_instance.start(&mut store))
            .map_err(|e| eco_format!("{e}"))?;

        // Ensure that the plugin exports its memory.
        if !matches!(
            instance.get_export(&store, "memory"),
            Some(wasmi::Extern::Memory(_))
        ) {
            bail!("plugin does not export its memory");
        }

        // Collect exported functions.
        let functions = instance
            .exports(&store)
            .filter_map(|export| {
                let name = export.name().into();
                export.into_func().map(|func| (name, func))
            })
            .collect();

        Ok(Plugin(Arc::new(Repr { bytes, functions, store: Mutex::new(store) })))
    }

    /// Call the plugin function with the given `name`.
    pub fn call(&self, name: &str, args: Vec<Bytes>) -> StrResult<Bytes> {
        // Find the function with the given name.
        let func = self
            .0
            .functions
            .iter()
            .find(|(v, _)| v == name)
            .map(|&(_, func)| func)
            .ok_or_else(|| {
                eco_format!("plugin does not contain a function called {name}")
            })?;

        let mut store = self.0.store.lock().unwrap();
        let ty = func.ty(store.as_context());

        // Check function signature.
        if ty.params().iter().any(|&v| v != wasmi::core::ValueType::I32) {
            bail!(
                "plugin function `{name}` has a parameter that is not a 32-bit integer"
            );
        }
        if ty.results() != [wasmi::core::ValueType::I32] {
            bail!("plugin function `{name}` does not return exactly one 32-bit integer");
        }

        // Check inputs.
        let expected = ty.params().len();
        let given = args.len();
        if expected != given {
            bail!(
                "plugin function takes {expected} argument{}, but {given} {} given",
                if expected == 1 { "" } else { "s" },
                if given == 1 { "was" } else { "were" },
            );
        }

        // Collect the lengths of the argument buffers.
        let lengths = args
            .iter()
            .map(|a| wasmi::Value::I32(a.len() as i32))
            .collect::<Vec<_>>();

        // Store the input data.
        store.data_mut().args = args;

        // Call the function.
        let mut code = wasmi::Value::I32(-1);
        func.call(store.as_context_mut(), &lengths, std::slice::from_mut(&mut code))
            .map_err(|err| eco_format!("plugin panicked: {err}"))?;

        // Extract the returned data.
        let output = std::mem::take(&mut store.data_mut().output);

        // Parse the functions return value.
        match code {
            wasmi::Value::I32(0) => {}
            wasmi::Value::I32(1) => match std::str::from_utf8(&output) {
                Ok(message) => bail!("plugin errored with: {message}"),
                Err(_) => {
                    bail!("plugin errored, but did not return a valid error message")
                }
            },
            _ => bail!("plugin did not respect the protocol"),
        };

        Ok(output.into())
    }

    /// An iterator over all the function names defined by the plugin.
    pub fn iter(&self) -> impl Iterator<Item = &EcoString> {
        self.0.functions.as_slice().iter().map(|(func_name, _)| func_name)
    }
}

impl Debug for Plugin {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("plugin(..)")
    }
}

impl PartialEq for Plugin {
    fn eq(&self, other: &Self) -> bool {
        self.0.bytes == other.0.bytes
    }
}

impl Hash for Plugin {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.bytes.hash(state);
    }
}

cast! {
    type Plugin: "plugin",
}

/// Write the arguments to the plugin function into the plugin's memory.
fn wasm_minimal_protocol_write_args_to_buffer(mut caller: Caller<StoreData>, ptr: u32) {
    let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
    let arguments = std::mem::take(&mut caller.data_mut().args);
    let mut offset = ptr as usize;
    for arg in arguments {
        memory.write(&mut caller, offset, arg.as_slice()).unwrap();
        offset += arg.len();
    }
}

/// Extracts the output of the plugin function from the plugin's memory.
fn wasm_minimal_protocol_send_result_to_host(
    mut caller: Caller<StoreData>,
    ptr: u32,
    len: u32,
) {
    let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
    let mut buffer = std::mem::take(&mut caller.data_mut().output);
    buffer.resize(len as usize, 0);
    memory.read(&caller, ptr as _, &mut buffer).unwrap();
    caller.data_mut().output = buffer;
}
