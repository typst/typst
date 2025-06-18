use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use ecow::{eco_format, EcoString};
use typst_syntax::Spanned;
use wasmi::Memory;

use crate::diag::{bail, At, SourceResult, StrResult};
use crate::engine::Engine;
use crate::foundations::{cast, func, scope, Binding, Bytes, Func, Module, Scope, Value};
use crate::loading::{DataSource, Load};

/// Loads a WebAssembly module.
///
/// The resulting [module] will contain one Typst [function] for each function
/// export of the loaded WebAssembly module.
///
/// Typst WebAssembly plugins need to follow a specific
/// [protocol]($plugin/#protocol). To run as a plugin, a program needs to be
/// compiled to a 32-bit shared WebAssembly library. Plugin functions may accept
/// multiple [byte buffers]($bytes) as arguments and return a single byte
/// buffer. They should typically be wrapped in idiomatic Typst functions that
/// perform the necessary conversions between native Typst types and bytes.
///
/// For security reasons, plugins run in isolation from your system. This means
/// that printing, reading files, or similar things are not supported.
///
/// # Example
/// ```example
/// #let myplugin = plugin("hello.wasm")
/// #let concat(a, b) = str(
///   myplugin.concatenate(
///     bytes(a),
///     bytes(b),
///   )
/// )
///
/// #concat("hello", "world")
/// ```
///
/// Since the plugin function returns a module, it can be used with import
/// syntax:
/// ```typ
/// #import plugin("hello.wasm"): concatenate
/// ```
///
/// # Purity
/// Plugin functions **must be pure:** A plugin function call most not have any
/// observable side effects on future plugin calls and given the same arguments,
/// it must always return the same value.
///
/// The reason for this is that Typst functions must be pure (which is quite
/// fundamental to the language design) and, since Typst function can call
/// plugin functions, this requirement is inherited. In particular, if a plugin
/// function is called twice with the same arguments, Typst might cache the
/// results and call your function only once. Moreover, Typst may run multiple
/// instances of your plugin in multiple threads, with no state shared between
/// them.
///
/// Typst does not enforce plugin function purity (for efficiency reasons), but
/// calling an impure function will lead to unpredictable and irreproducible
/// results and must be avoided.
///
/// That said, mutable operations _can be_ useful for plugins that require
/// costly runtime initialization. Due to the purity requirement, such
/// initialization cannot be performed through a normal function call. Instead,
/// Typst exposes a [plugin transition API]($plugin.transition), which executes
/// a function call and then creates a derived module with new functions which
/// will observe the side effects produced by the transition call. The original
/// plugin remains unaffected.
///
/// # Plugins and Packages
/// Any Typst code can make use of a plugin simply by including a WebAssembly
/// file and loading it. However, because the byte-based plugin interface is
/// quite low-level, plugins are typically exposed through a package containing
/// the plugin and idiomatic wrapper functions.
///
/// # WASI
/// Many compilers will use the [WASI ABI](https://wasi.dev/) by default or as
/// their only option (e.g. emscripten), which allows printing, reading files,
/// etc. This ABI will not directly work with Typst. You will either need to
/// compile to a different target or [stub all
/// functions](https://github.com/astrale-sharp/wasm-minimal-protocol/tree/master/crates/wasi-stub).
///
/// # Protocol
/// To be used as a plugin, a WebAssembly module must conform to the following
/// protocol:
///
/// ## Exports
/// A plugin module can export functions to make them callable from Typst. To
/// conform to the protocol, an exported function should:
///
/// - Take `n` 32-bit integer arguments `a_1`, `a_2`, ..., `a_n` (interpreted as
///   lengths, so `usize/size_t` may be preferable), and return one 32-bit
///   integer.
///
/// - The function should first allocate a buffer `buf` of length `a_1 + a_2 +
///   ... + a_n`, and then call
///   `wasm_minimal_protocol_write_args_to_buffer(buf.ptr)`.
///
/// - The `a_1` first bytes of the buffer now constitute the first argument, the
///   `a_2` next bytes the second argument, and so on.
///
/// - The function can now do its job with the arguments and produce an output
///   buffer. Before returning, it should call
///   `wasm_minimal_protocol_send_result_to_host` to send its result back to the
///   host.
///
/// - To signal success, the function should return `0`.
///
/// - To signal an error, the function should return `1`. The written buffer is
///   then interpreted as an UTF-8 encoded error message.
///
/// ## Imports
/// Plugin modules need to import two functions that are provided by the
/// runtime. (Types and functions are described using WAT syntax.)
///
/// - `(import "typst_env" "wasm_minimal_protocol_write_args_to_buffer" (func
///   (param i32)))`
///
///   Writes the arguments for the current function into a plugin-allocated
///   buffer. When a plugin function is called, it [receives the
///   lengths](#exports) of its input buffers as arguments. It should then
///   allocate a buffer whose capacity is at least the sum of these lengths. It
///   should then call this function with a `ptr` to the buffer to fill it with
///   the arguments, one after another.
///
/// - `(import "typst_env" "wasm_minimal_protocol_send_result_to_host" (func
///   (param i32 i32)))`
///
///   Sends the output of the current function to the host (Typst). The first
///   parameter shall be a pointer to a buffer (`ptr`), while the second is the
///   length of that buffer (`len`). The memory pointed at by `ptr` can be freed
///   immediately after this function returns. If the message should be
///   interpreted as an error message, it should be encoded as UTF-8.
///
/// # Resources
/// For more resources, check out the [wasm-minimal-protocol
/// repository](https://github.com/astrale-sharp/wasm-minimal-protocol). It
/// contains:
///
/// - A list of example plugin implementations and a test runner for these
///   examples
/// - Wrappers to help you write your plugin in Rust (Zig wrapper in
///   development)
/// - A stubber for WASI
#[func(scope)]
pub fn plugin(
    engine: &mut Engine,
    /// A [path]($syntax/#paths) to a WebAssembly file or raw WebAssembly bytes.
    source: Spanned<DataSource>,
) -> SourceResult<Module> {
    let loaded = source.load(engine.world)?;
    Plugin::module(loaded.data).at(source.span)
}

#[scope]
impl plugin {
    /// Calls a plugin function that has side effects and returns a new module
    /// with plugin functions that are guaranteed to have observed the results
    /// of the mutable call.
    ///
    /// Note that calling an impure function through a normal function call
    /// (without use of the transition API) is forbidden and leads to
    /// unpredictable behaviour. Read the [section on purity]($plugin/#purity)
    /// for more details.
    ///
    /// In the example below, we load the plugin `hello-mut.wasm` which exports
    /// two functions: The `get()` function retrieves a global array as a
    /// string. The `add(value)` function adds a value to the global array.
    ///
    /// We call `add` via the transition API. The call `mutated.get()` on the
    /// derived module will observe the addition. Meanwhile the original module
    /// remains untouched as demonstrated by the `base.get()` call.
    ///
    /// _Note:_ Due to limitations in the internal WebAssembly implementation,
    /// the transition API can only guarantee to reflect changes in the plugin's
    /// memory, not in WebAssembly globals. If your plugin relies on changes to
    /// globals being visible after transition, you might want to avoid use of
    /// the transition API for now. We hope to lift this limitation in the
    /// future.
    ///
    /// ```typ
    /// #let base = plugin("hello-mut.wasm")
    /// #assert.eq(base.get(), "[]")
    ///
    /// #let mutated = plugin.transition(base.add, "hello")
    /// #assert.eq(base.get(), "[]")
    /// #assert.eq(mutated.get(), "[hello]")
    /// ```
    #[func]
    pub fn transition(
        /// The plugin function to call.
        func: PluginFunc,
        /// The byte buffers to call the function with.
        #[variadic]
        arguments: Vec<Bytes>,
    ) -> StrResult<Module> {
        func.transition(arguments)
    }
}

/// A function loaded from a WebAssembly plugin.
#[derive(Debug, Clone, PartialEq, Hash)]
pub struct PluginFunc {
    /// The underlying plugin, shared by this and the other functions.
    plugin: Arc<Plugin>,
    /// The name of the plugin function.
    name: EcoString,
}

impl PluginFunc {
    /// The name of the plugin function.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Call the WebAssembly function with the given arguments.
    #[comemo::memoize]
    #[typst_macros::time(name = "call plugin")]
    pub fn call(&self, args: Vec<Bytes>) -> StrResult<Bytes> {
        self.plugin.call(&self.name, args)
    }

    /// Transition a plugin and turn the result into a module.
    #[comemo::memoize]
    #[typst_macros::time(name = "transition plugin")]
    pub fn transition(&self, args: Vec<Bytes>) -> StrResult<Module> {
        self.plugin.transition(&self.name, args).map(Plugin::into_module)
    }
}

cast! {
    PluginFunc,
    self => Value::Func(self.into()),
    v: Func => v.to_plugin().ok_or("expected plugin function")?.clone(),
}

/// A plugin with potentially multiple instances for multi-threaded
/// execution.
struct Plugin {
    /// Shared by all variants of the plugin.
    base: Arc<PluginBase>,
    /// A pool of plugin instances.
    ///
    /// When multiple plugin calls run concurrently due to multi-threading, we
    /// create new instances whenever we run out of ones.
    pool: Mutex<Vec<PluginInstance>>,
    /// A snapshot that new instances should be restored to.
    snapshot: Option<Snapshot>,
    /// A combined hash that incorporates all function names and arguments used
    /// in transitions of this plugin, such that this plugin has a deterministic
    /// hash and equality check that can differentiate it from "siblings" (same
    /// base, different transitions).
    fingerprint: u128,
}

impl Plugin {
    /// Create a plugin and turn it into a module.
    #[comemo::memoize]
    #[typst_macros::time(name = "load plugin")]
    fn module(bytes: Bytes) -> StrResult<Module> {
        Self::new(bytes).map(Self::into_module)
    }

    /// Create a new plugin from raw WebAssembly bytes.
    fn new(bytes: Bytes) -> StrResult<Self> {
        let engine = wasmi::Engine::default();
        let module = wasmi::Module::new(&engine, bytes.as_slice())
            .map_err(|err| format!("failed to load WebAssembly module ({err})"))?;

        // Ensure that the plugin exports its memory.
        if !matches!(module.get_export("memory"), Some(wasmi::ExternType::Memory(_))) {
            bail!("plugin does not export its memory");
        }

        let mut linker = wasmi::Linker::new(&engine);
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

        let base = Arc::new(PluginBase { bytes, linker, module });
        let instance = PluginInstance::new(&base, None)?;

        Ok(Self {
            base,
            snapshot: None,
            fingerprint: 0,
            pool: Mutex::new(vec![instance]),
        })
    }

    /// Execute a function with access to an instsance.
    fn call(&self, func: &str, args: Vec<Bytes>) -> StrResult<Bytes> {
        // Acquire an instance from the pool (potentially creating a new one).
        let mut instance = self.acquire()?;

        // Execute the call on an instance from the pool. If the call fails, we
        // return early and _don't_ return the instance to the pool as it might
        // be irrecoverably damaged.
        let output = instance.call(func, args)?;

        // Return the instance to the pool.
        self.pool.lock().unwrap().push(instance);

        Ok(output)
    }

    /// Call a mutable plugin function, producing a new mutable whose functions
    /// are guaranteed to be able to observe the mutation.
    fn transition(&self, func: &str, args: Vec<Bytes>) -> StrResult<Plugin> {
        // Derive a new transition hash from the old one and the function and arguments.
        let fingerprint = typst_utils::hash128(&(self.fingerprint, func, &args));

        // Execute the mutable call on an instance.
        let mut instance = self.acquire()?;

        // Call the function. If the call fails, we return early and _don't_
        // return the instance to the pool as it might be irrecoverably damaged.
        instance.call(func, args)?;

        // Snapshot the instance after the mutable call.
        let snapshot = instance.snapshot();

        // Create a new plugin and move (this is important!) the used instance
        // into it, so that the old plugin won't observe the mutation. Also
        // save the snapshot so that instances that are initialized for the
        // transitioned plugin's pool observe the mutation.
        Ok(Self {
            base: self.base.clone(),
            snapshot: Some(snapshot),
            fingerprint,
            pool: Mutex::new(vec![instance]),
        })
    }

    /// Acquire an instance from the pool (or create a new one).
    fn acquire(&self) -> StrResult<PluginInstance> {
        // Don't use match to ensure that the lock is released before we create
        // a new instance.
        if let Some(instance) = self.pool.lock().unwrap().pop() {
            return Ok(instance);
        }

        PluginInstance::new(&self.base, self.snapshot.as_ref())
    }

    /// Turn a plugin into a Typst module containing plugin functions.
    fn into_module(self) -> Module {
        let shared = Arc::new(self);

        // Build a scope from the collected functions.
        let mut scope = Scope::new();
        for export in shared.base.module.exports() {
            if matches!(export.ty(), wasmi::ExternType::Func(_)) {
                let name = EcoString::from(export.name());
                let func = PluginFunc { plugin: shared.clone(), name: name.clone() };
                scope.bind(name, Binding::detached(Func::from(func)));
            }
        }

        Module::anonymous(scope)
    }
}

impl Debug for Plugin {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("Plugin(..)")
    }
}

impl PartialEq for Plugin {
    fn eq(&self, other: &Self) -> bool {
        self.base.bytes == other.base.bytes && self.fingerprint == other.fingerprint
    }
}

impl Hash for Plugin {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.base.bytes.hash(state);
        self.fingerprint.hash(state);
    }
}

/// Shared by all pooled & transitioned variants of the plugin.
struct PluginBase {
    /// The raw WebAssembly bytes.
    bytes: Bytes,
    /// The compiled WebAssembly module.
    module: wasmi::Module,
    /// A linker used to create a `Store` for execution.
    linker: wasmi::Linker<CallData>,
}

/// An single plugin instance for single-threaded execution.
struct PluginInstance {
    /// The underlying wasmi instance.
    instance: wasmi::Instance,
    /// The execution store of this concrete plugin instance.
    store: wasmi::Store<CallData>,
}

/// A snapshot of a plugin instance.
struct Snapshot {
    /// The number of pages in the main memory.
    mem_pages: u32,
    /// The data in the main memory.
    mem_data: Vec<u8>,
}

impl PluginInstance {
    /// Create a new execution instance of a plugin, potentially restoring
    /// a snapshot.
    #[typst_macros::time(name = "create plugin instance")]
    fn new(base: &PluginBase, snapshot: Option<&Snapshot>) -> StrResult<PluginInstance> {
        let mut store = wasmi::Store::new(base.linker.engine(), CallData::default());
        let instance = base
            .linker
            .instantiate(&mut store, &base.module)
            .and_then(|pre_instance| pre_instance.start(&mut store))
            .map_err(|e| eco_format!("{e}"))?;

        let mut instance = PluginInstance { instance, store };
        if let Some(snapshot) = snapshot {
            instance.restore(snapshot);
        }
        Ok(instance)
    }

    /// Call a plugin function with byte arguments.
    fn call(&mut self, func: &str, args: Vec<Bytes>) -> StrResult<Bytes> {
        let handle = self
            .instance
            .get_export(&self.store, func)
            .unwrap()
            .into_func()
            .unwrap();
        let ty = handle.ty(&self.store);

        // Check function signature. Do this lazily only when a function is called
        // because there might be exported functions like `_initialize` that don't
        // match the schema.
        if ty.params().iter().any(|&v| v != wasmi::core::ValType::I32) {
            bail!(
                "plugin function `{func}` has a parameter that is not a 32-bit integer"
            );
        }
        if ty.results() != [wasmi::core::ValType::I32] {
            bail!("plugin function `{func}` does not return exactly one 32-bit integer");
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
            .map(|a| wasmi::Val::I32(a.len() as i32))
            .collect::<Vec<_>>();

        // Store the input data.
        self.store.data_mut().args = args;

        // Call the function.
        let mut code = wasmi::Val::I32(-1);
        handle
            .call(&mut self.store, &lengths, std::slice::from_mut(&mut code))
            .map_err(|err| eco_format!("plugin panicked: {err}"))?;

        if let Some(MemoryError { offset, length, write }) =
            self.store.data_mut().memory_error.take()
        {
            return Err(eco_format!(
                "plugin tried to {kind} out of bounds: \
                 pointer {offset:#x} is out of bounds for {kind} of length {length}",
                kind = if write { "write" } else { "read" }
            ));
        }

        // Extract the returned data.
        let output = std::mem::take(&mut self.store.data_mut().output);

        // Parse the functions return value.
        match code {
            wasmi::Val::I32(0) => {}
            wasmi::Val::I32(1) => match std::str::from_utf8(&output) {
                Ok(message) => bail!("plugin errored with: {message}"),
                Err(_) => {
                    bail!("plugin errored, but did not return a valid error message")
                }
            },
            _ => bail!("plugin did not respect the protocol"),
        };

        Ok(Bytes::new(output))
    }

    /// Creates a snapshot of this instance from which another one can be
    /// initialized.
    #[typst_macros::time(name = "save snapshot")]
    fn snapshot(&self) -> Snapshot {
        let memory = self.memory();
        let mem_pages = memory.size(&self.store);
        let mem_data = memory.data(&self.store).to_vec();
        Snapshot { mem_pages: mem_pages.try_into().unwrap(), mem_data }
    }

    /// Restores the instance to a snapshot.
    #[typst_macros::time(name = "restore snapshot")]
    fn restore(&mut self, snapshot: &Snapshot) {
        let memory = self.memory();
        let current_size = memory.size(&self.store);
        let snapshot_pages = u64::from(snapshot.mem_pages);
        if current_size < snapshot_pages {
            memory
                .grow(&mut self.store, snapshot_pages - current_size)
                .unwrap();
        }

        memory.data_mut(&mut self.store)[..snapshot.mem_data.len()]
            .copy_from_slice(&snapshot.mem_data);
    }

    /// Retrieves a handle to the plugin's main memory.
    fn memory(&self) -> Memory {
        self.instance
            .get_export(&self.store, "memory")
            .unwrap()
            .into_memory()
            .unwrap()
    }
}

/// The persistent store data used for communication between store and host.
#[derive(Default)]
struct CallData {
    /// Arguments for a current call.
    args: Vec<Bytes>,
    /// The results of the current call.
    output: Vec<u8>,
    /// A memory error that occured during execution of the current call.
    memory_error: Option<MemoryError>,
}

/// If there was an error reading/writing memory, keep the offset + length to
/// display an error message.
struct MemoryError {
    offset: u32,
    length: u32,
    write: bool,
}

/// Write the arguments to the plugin function into the plugin's memory.
fn wasm_minimal_protocol_write_args_to_buffer(
    mut caller: wasmi::Caller<CallData>,
    ptr: u32,
) {
    let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
    let arguments = std::mem::take(&mut caller.data_mut().args);
    let mut offset = ptr as usize;
    for arg in arguments {
        if memory.write(&mut caller, offset, arg.as_slice()).is_err() {
            caller.data_mut().memory_error = Some(MemoryError {
                offset: offset as u32,
                length: arg.len() as u32,
                write: true,
            });
            return;
        }
        offset += arg.len();
    }
}

/// Extracts the output of the plugin function from the plugin's memory.
fn wasm_minimal_protocol_send_result_to_host(
    mut caller: wasmi::Caller<CallData>,
    ptr: u32,
    len: u32,
) {
    let memory = caller.get_export("memory").unwrap().into_memory().unwrap();
    let mut buffer = std::mem::take(&mut caller.data_mut().output);
    buffer.resize(len as usize, 0);
    if memory.read(&caller, ptr as _, &mut buffer).is_err() {
        caller.data_mut().memory_error =
            Some(MemoryError { offset: ptr, length: len, write: false });
        return;
    }
    caller.data_mut().output = buffer;
}
