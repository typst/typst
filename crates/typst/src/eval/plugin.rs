use crate::diag::SourceResult;
use crate::Bytes;
use ecow::EcoString;
use std::sync::Mutex;
use typst::diag::At;
use wasmi::{AsContext, Caller, Engine, Func as Function, Linker, Module, Value};

#[derive(Debug)]
pub struct Plugin(Mutex<PluginInstance>, Bytes);

impl crate::eval::Type for Plugin {
    const TYPE_NAME: &'static str = "Plugin";
}

impl PartialEq for Plugin {
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}

impl std::hash::Hash for Plugin {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.1.hash(state);
    }
}

impl Plugin {
    pub fn new(instance: PluginInstance, bytes: Bytes) -> Self {
        Self(Mutex::new(instance), bytes)
    }
    pub fn call(
        &self,
        function: &str,
        args: &mut typst::eval::Args,
    ) -> SourceResult<typst::eval::Value> {
        let span = args.span;
        let plugin = &mut self.0.lock().unwrap();
        let ty = plugin
            .get_function(function)
            .ok_or("Plugin doesn't have the method: {function}")
            .at(span)?
            .ty(plugin.get_store());
        let arg_count = ty.params().len();
        let mut str_args = vec![];
        for k in 0..arg_count {
            let arg = args
                .eat::<typst::eval::Value>()?
                .ok_or(format!("plugin methods takes {arg_count} args, {k} provided"))
                .at(span)?
                .cast::<String>()
                .at(span)?;
            str_args.push(arg);
        }
        let s = plugin
            .call(
                function,
                str_args.iter().map(|x| x.as_str()).collect::<Vec<_>>().as_slice(),
            )
            .at(span)?;
        Ok(typst::eval::Value::Str(s.into()))
    }

    pub fn has_function(&self, method: &str) -> bool {
        self.0.lock().unwrap().has_function(method)
    }

    pub fn iter_func(&self) -> Vec<EcoString> {
        self.0
            .lock()
            .unwrap()
            .iter_functions()
            .map(|x| x.to_string().into())
            .collect::<Vec<_>>()
    }
}

type Store = wasmi::Store<PersistentData>;

#[derive(Debug, Clone)]
struct PersistentData {
    result_data: String,
    arg_buffer: String,
}

#[derive(Debug)]
pub struct PluginInstance {
    store: Store,
    functions: Vec<(String, Function)>,
}

impl PluginInstance {
    pub fn new_from_bytes(bytes: impl AsRef<[u8]>) -> Result<Self, String> {
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
        Ok(Self { store, functions })
    }

    pub fn write(&mut self, args: &[&str]) {
        let mut all_args = String::new();
        for arg in args {
            all_args += arg;
        }
        self.store.data_mut().arg_buffer = all_args;
    }

    pub fn call(&mut self, function: &str, args: &[&str]) -> Result<String, String> {
        self.write(args);

        let (_, function) = self
            .functions
            .iter()
            .find(|(s, _)| s == function)
            .ok_or(format!("Plugin doesn't have the method: {function}"))?;

        let result_args =
            args.iter().map(|a| Value::I32(a.len() as _)).collect::<Vec<_>>();

        let mut code = [Value::I32(2)];
        let is_err = function.call(&mut self.store, &result_args, &mut code).is_err();
        let code = if is_err {
            Value::I32(2)
        } else {
            code.first().cloned().unwrap_or(Value::I32(3)) // if the function returns nothing
        };

        let s = std::mem::take(&mut self.store.data_mut().result_data);

        match code {
            Value::I32(0) => Ok(s),
            Value::I32(1) => Err(format!(
                "plugin errored with: {:?} with code: {}",
                s,
                code.i32().unwrap()
            )),
            Value::I32(2) => Err("plugin panicked".to_string()),
            _ => Err("plugin did not respect the protocol".to_string()),
        }
    }

    pub fn has_function(&self, method: &str) -> bool {
        self.functions.iter().any(|(s, _)| s == method)
    }

    pub fn get_function(&self, function_name: &str) -> Option<Function> {
        let Some((_, function)) = self.functions.iter().find(|(s, _)| s == function_name) else {
            return None
        };
        Some(*function)
    }

    pub fn iter_functions(&self) -> impl Iterator<Item = &String> {
        self.functions.as_slice().iter().map(|(x, _)| x)
    }

    pub fn get_store(&self) -> &impl AsContext {
        &self.store
    }
}
