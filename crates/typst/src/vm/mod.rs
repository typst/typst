mod access;
mod closure;
mod module;
mod opcodes;
mod pattern;
mod values;

use std::num::NonZeroU64;
use std::sync::Arc;

use bytemuck::cast;
use comemo::{Track, Tracked, TrackedMut};
use ecow::EcoVec;
use typst_syntax::{Source, Span};

use crate::compiler::compile_module;
use crate::diag::{bail, At, SourceResult, StrResult};
use crate::engine::{Engine, Route};
use crate::eval::{ops, Tracer};
use crate::foundations::{
    Content, IntoValue, Label, Module, NativeElement, Recipe, SequenceElem, Styles,
    Unlabellable, Value,
};
use crate::introspection::{Introspector, Locator};
use crate::{Library, World};

pub use self::access::*;
pub use self::closure::*;
pub use self::module::*;
pub use self::opcodes::*;
pub use self::pattern::*;
pub use self::values::*;

/// Evaluate a source file and return the resulting module.
#[comemo::memoize]
#[typst_macros::time(name = "eval", span = source.root().span())]
pub fn eval(
    world: Tracked<dyn World + '_>,
    route: Tracked<Route>,
    mut tracer: TrackedMut<Tracer>,
    source: &Source,
) -> SourceResult<Module> {
    // Prepare the engine.
    let locator = Locator::new();
    let introspector = Introspector::default();

    // Compile the module
    let compiled = compile_module(
        source,
        world,
        introspector.track(),
        route,
        locator.track(),
        TrackedMut::reborrow_mut(&mut tracer),
    )?;

    // Evaluate the module
    run_module(&compiled, world, introspector.track(), route, locator.track(), tracer)
}

bitflags::bitflags! {
    /// The current state of the VM.
    struct State: u16 {
        /// The VM is currently running.
        const LOOPING = 0b0000_0001;
        /// The VM is currently joining.
        const JOINING = 0b0000_0010;
        /// The VM is currently displaying.
        const DISPLAY = 0b0000_0100;
        /// The VM is currently breaking.
        const BREAKING = 0b0001_0000;
        /// The VM is currently continuing.
        const CONTINUING = 0b0010_0000;
        /// The VM is currently returning.
        const RETURNING = 0b0100_0000;
        /// The VM is done.
        const DONE = 0b1000_0000;
    }
}

impl State {
    const fn is_breaking(&self) -> bool {
        self.contains(Self::BREAKING)
    }

    const fn is_continuing(&self) -> bool {
        self.contains(Self::CONTINUING)
    }

    const fn is_returning(&self) -> bool {
        self.contains(Self::RETURNING)
    }

    const fn is_done(&self) -> bool {
        self.contains(Self::DONE)
    }

    const fn is_running(&self) -> bool {
        !self.is_done()
    }

    const fn is_joining(&self) -> bool {
        self.contains(Self::JOINING)
    }

    const fn is_display(&self) -> bool {
        self.is_joining() && self.contains(Self::DISPLAY)
    }
}

pub struct VM<'a> {
    /// The mutable state of the VM.
    state: VMState<'a>,
    /// The span of the whole VM.
    span: Span,
    /// The list of instructions.
    instructions: &'a [u8],
}

#[derive(Debug)]
pub enum ControlFlow {
    Done(Value),
    Break(Value),
    Continue(Value),
    Return(Value),
}

impl VM<'_> {
    /// Runs the VM to completion.
    pub fn run(&mut self, engine: &mut Engine) -> SourceResult<ControlFlow> {
        while self.state.state.is_running() {
            let Some(opcode) = self.run_one(engine)? else {
                break;
            };

            if opcode == Opcode::Flow {
                if self.state.iterator.is_some() {
                    if self.state.state.is_continuing() {
                        self.state.instruction_pointer = 0;
                        self.state.state.remove(State::CONTINUING);
                        continue;
                    } else if self.state.state.is_breaking()
                        || self.state.state.is_returning()
                    {
                        // In theory, the compiler should make sure that this is valid.
                        break;
                    }
                } else if self.state.state.is_breaking()
                    || self.state.state.is_continuing()
                    || self.state.state.is_returning()
                {
                    // In theory, the compiler should make sure that this is valid.
                    break;
                }
            }
        }

        let output = if let Some(reg) = self.state.output {
            reg.read(&self.state).cloned().map(Some).at(self.span)?
        } else if let Some(joined) = self.state.joined.clone() {
            Some(joined.collect(engine)?)
        } else {
            None
        };

        if self.state.state.is_continuing() && self.state.iterator.is_none() {
            Ok(ControlFlow::Continue(output.unwrap_or(Value::None)))
        } else if self.state.state.is_breaking() && self.state.iterator.is_none() {
            Ok(ControlFlow::Break(output.unwrap_or(Value::None)))
        } else if self.state.state.is_returning() {
            Ok(ControlFlow::Return(output.unwrap_or(Value::None)))
        } else {
            Ok(ControlFlow::Done(output.unwrap_or(Value::None)))
        }
    }

    /// Gets the next op code from the instruction list.
    fn next_opcode(&self) -> Option<(Opcode, Span)> {
        if self.state.instruction_pointer == self.instructions.len() {
            return None;
        }

        debug_assert!(self.state.instruction_pointer + 1 <= self.instructions.len());

        let opcode = self.instructions[self.state.instruction_pointer];
        let opcode = Opcode::from_u8(opcode).expect("malformed opcode");
        if opcode == Opcode::Flow {
            return Some((opcode, Span::detached()));
        }

        let span = self.read_span(self.state.instruction_pointer + 1);

        Some((opcode, span))
    }

    fn read_span(&self, offset: usize) -> Span {
        let bytes = &self.instructions[offset..offset + std::mem::size_of::<Span>()];
        let Some(number) = cast::<[u8; 8], Option<NonZeroU64>>([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
            bytes[7],
        ]) else {
            return Span::detached();
        };

        // SAFETY: we know that the instructions are valid because they were
        //         generated by the compiler.
        unsafe { Span::new_unchecked(number) }
    }

    fn run_one(&mut self, engine: &mut Engine) -> SourceResult<Option<Opcode>> {
        let Some((opcode, span)) = self.next_opcode() else {
            self.state.state.insert(State::DONE);
            return Ok(None);
        };

        opcode.run(&self.instructions, span, &mut self.state, engine)?;

        Ok(Some(opcode))
    }
}

pub struct VMState<'a> {
    /// The current state of the VM.
    state: State,
    /// The output register, if any.
    output: Option<Readable>,
    /// The global library.
    global: &'a Library,
    /// The current instruction pointer.
    instruction_pointer: usize,
    /// The registers.
    registers: [Value; 256],
    /// The joined values.
    joined: Option<Joiner>,
    /// The constants.
    constants: &'a [Value],
    /// The strings.
    /// These are stored as [`Value`] but they are always [`Value::Str`].
    strings: &'a [Value],
    /// The labels.
    labels: &'a [Label],
    /// The closures.
    closures: &'a [CompiledClosure],
    /// The access patterns.
    accesses: &'a [Access],
    /// The destructure patterns.
    patterns: &'a [Pattern],
    /// The default values of each scope.
    defaults: &'a [EcoVec<DefaultValue>],
    /// The parent VM.
    parent: Option<&'a mut VMState<'a>>,
    /// The iterator, if any.
    iterator: Option<Box<dyn Iterator<Item = Value>>>,
}

impl<'a> VMState<'a> {
    /// Read a value from the VM.
    pub fn read<T: VmRead>(&self, readable: T) -> StrResult<T::Output<'_>> {
        readable.read(self)
    }

    /// Write a value to the VM, returning a mutable reference to the value.
    pub fn write(&mut self, writable: impl VmWrite) -> StrResult<&mut Value> {
        writable.write(self)
    }

    /// Write a value to the VM.
    pub fn write_one(
        &mut self,
        writable: impl VmWrite,
        value: impl IntoValue,
    ) -> StrResult<()> {
        writable.write_one(self, value)
    }

    /// Join a value to the current joining state.
    pub fn join(&mut self, value: impl IntoValue) -> StrResult<()> {
        if !self.state.is_joining() {
            bail!("cannot join in non-joining state");
        }

        // Convert the value to a display value if we are in display mode.
        let value = value.into_value();

        // We don't join `None`.
        if value.is_none() {
            return Ok(());
        }

        if let Some(joiner) = self.joined.take() {
            self.joined = Some(joiner.join(value)?);
        } else if self.state.is_display() {
            self.joined =
                Some(Joiner::Display(SequenceElem::new(vec![value.display().into()])));
        } else {
            self.joined = Some(Joiner::Value(value));
        }

        Ok(())
    }

    pub fn styled(&mut self, styles: Styles) -> StrResult<()> {
        if !self.state.is_joining() {
            bail!("cannot style in non-joining state");
        }

        if let Some(joiner) = self.joined.take() {
            self.joined = Some(joiner.styled(styles));
        } else {
            self.joined = Some(Joiner::Styled {
                parent: None,
                content: SequenceElem::new(vec![]),
                styles,
            });
        }

        Ok(())
    }

    pub fn recipe(&mut self, recipe: Recipe) -> StrResult<()> {
        if !self.state.is_joining() {
            bail!("cannot style in non-joining state");
        }

        if let Some(joiner) = self.joined.take() {
            self.joined = Some(joiner.recipe(recipe));
        } else {
            self.joined = Some(Joiner::Recipe {
                parent: None,
                content: SequenceElem::new(vec![]),
                recipe,
            });
        }

        Ok(())
    }

    /// Instantiate a closure.
    #[typst_macros::time(name = "instantiate closure", span = closure.inner.span)]
    pub fn instantiate(&self, closure: &CompiledClosure) -> SourceResult<Closure> {
        // Load the default values for the parameters.
        let mut params = EcoVec::with_capacity(closure.params.len());
        for param in &closure.params {
            match param {
                CompiledParam::Pos(target, pos) => params
                    .push((OptionalWritable::some(*target), Param::Pos(pos.clone()))),
                CompiledParam::Named { span, target, name, default } => {
                    params.push((
                        OptionalWritable::some(*target),
                        Param::Named {
                            name: name.clone(),
                            default: self.read(*default).at(*span)?.cloned(),
                        },
                    ));
                }
                CompiledParam::Sink(span, target, name) => {
                    params.push((*target, Param::Sink(*span, name.clone())));
                }
            }
        }

        // Load the captured values.
        let mut captures = EcoVec::with_capacity(closure.captures.len());
        for capture in &closure.captures {
            captures.push((
                capture.location,
                self.read(capture.value).at(capture.span)?.clone(),
            ));
        }

        Ok(Closure::new(
            Arc::clone(&closure.inner),
            params,
            captures,
            closure.self_storage,
        ))
    }

    /// Enter a new scope.
    pub fn enter_scope<'b>(
        &'b mut self,
        engine: &mut Engine,
        default_values: &[DefaultValue],
        instructions: &'b [u8],
        iterator: Option<Box<dyn Iterator<Item = Value>>>,
        output: Option<Readable>,
        joins: bool,
        content: bool,
        span: Span,
    ) -> SourceResult<ControlFlow>
    where
        'a: 'b,
    {
        // These are required to prove that the registers can be created
        // at compile time safely.
        const SIZE: usize = 256;
        const NONE: Value = Value::None;

        let global: &'b Library = cast_lifetime::<'a, 'b, _>(self.global);
        let constants: &'b [Value] = cast_lifetime::<'a, 'b, _>(self.constants);
        let strings: &'b [Value] = cast_lifetime::<'a, 'b, _>(self.strings);
        let labels: &'b [Label] = cast_lifetime::<'a, 'b, _>(self.labels);
        let closures: &'b [CompiledClosure] = cast_lifetime::<'a, 'b, _>(self.closures);
        let accesses: &'b [Access] = cast_lifetime::<'a, 'b, _>(self.accesses);
        let patterns: &'b [Pattern] = cast_lifetime::<'a, 'b, _>(self.patterns);
        let defaults: &'b [EcoVec<DefaultValue>] =
            cast_lifetime::<'a, 'b, _>(self.defaults);
        let this: &'b mut VMState<'b> = unsafe {
            // SAFETY: we are casting the lifetime of the reference to the
            //         parent VM state to the lifetime of the new VM state.
            //         This is safe because the parent VM state is guaranteed
            //         to live at least as long as the new VM state.
            std::mem::transmute(self)
        };

        let mut state = VMState::<'b> {
            state: State::empty()
                | if iterator.is_some() { State::LOOPING } else { State::empty() }
                | if joins { State::JOINING } else { State::empty() }
                | if content { State::DISPLAY } else { State::empty() },
            output,
            global,
            instruction_pointer: 0,
            registers: [NONE; SIZE],
            joined: None,
            constants,
            strings,
            labels,
            closures,
            accesses,
            patterns,
            defaults,
            parent: Some(this),
            iterator,
        };

        for default in default_values {
            state.registers[default.target.as_raw() as usize] = default.value.clone();
        }

        let mut vm = VM { state, span, instructions };

        vm.run(engine)
    }
}

#[derive(Clone)]
enum Joiner {
    Value(Value),
    Display(SequenceElem),
    Styled { parent: Option<Box<Joiner>>, styles: Styles, content: SequenceElem },
    Recipe { parent: Option<Box<Joiner>>, recipe: Recipe, content: SequenceElem },
}

impl Joiner {
    pub fn join(self, other: Value) -> StrResult<Joiner> {
        if other.is_none() {
            return Ok(self);
        }

        if let Value::Label(label) = other {
            match self {
                Self::Value(value) => Ok(Joiner::Value(ops::join(value, other)?)),
                Self::Display(mut content) => {
                    let Some(last) = content
                        .children_mut()
                        .rev()
                        .find(|elem| !elem.can::<dyn Unlabellable>())
                    else {
                        bail!("nothing to label");
                    };

                    last.update(|elem| {
                        elem.set_label(label)
                    });

                    Ok(Joiner::Display(content))
                }
                Self::Styled { parent, mut content, styles } => {
                    let Some(last) = content
                        .children_mut()
                        .rev()
                        .find(|elem| !elem.can::<dyn Unlabellable>())
                    else {
                        bail!("nothing to label");
                    };

                    last.update(|elem| elem.set_label(label));

                    Ok(Joiner::Styled { parent, content, styles })
                }
                Self::Recipe { parent, recipe, mut content } => {
                    let Some(last) = content
                        .children_mut()
                        .rev()
                        .find(|elem| !elem.can::<dyn Unlabellable>())
                    else {
                        bail!("nothing to label");
                    };

                    last.update(|elem| elem.set_label(label));

                    Ok(Joiner::Recipe { parent, content, recipe })
                }
            }
        } else {
            match self {
                Self::Value(value) => Ok(Joiner::Value(ops::join(value, other)?)),
                Self::Display(mut content) => {
                    content.push(other.display());
                    Ok(Joiner::Display(content))
                }
                Self::Styled { parent, mut content, styles } => {
                    content.push(other.display());
                    Ok(Joiner::Styled { parent, content, styles })
                }
                Self::Recipe { parent, recipe, mut content } => {
                    content.push(other.display());
                    Ok(Joiner::Recipe { parent, content, recipe })
                }
            }
        }
    }

    pub fn styled(self, to_add: Styles) -> Joiner {
        if let Self::Styled { parent, content, mut styles } = self {
            if content.is_empty() {
                styles.push_all(to_add);
                return Self::Styled { parent, content, styles };
            } else {
                Self::Styled {
                    parent: Some(Box::new(Self::Styled { parent, content, styles })),
                    content: SequenceElem::new(vec![]),
                    styles: to_add,
                }
            }
        } else {
            Self::Styled {
                parent: Some(Box::new(self)),
                content: SequenceElem::new(vec![]),
                styles: to_add,
            }
        }
    }

    pub fn recipe(self, recipe: Recipe) -> Joiner {
        Self::Recipe {
            parent: Some(Box::new(self)),
            content: SequenceElem::new(vec![]),
            recipe,
        }
    }

    pub fn collect(self, engine: &mut Engine) -> SourceResult<Value> {
        fn collect_inner(
            joiner: Joiner,
            engine: &mut Engine,
            rest: Option<Content>,
        ) -> SourceResult<Value> {
            Ok(match joiner {
                Joiner::Value(value) => {
                    if let Some(rest) = rest {
                        Content::sequence([value.display(), rest]).into_value()
                    } else {
                        value
                    }
                }
                Joiner::Display(mut content) => {
                    if let Some(rest) = rest {
                        content.push(rest);
                    }

                    if content.len() == 1 {
                        content.pop().unwrap().into_value()
                    } else {
                        content.into_value()
                    }
                }
                Joiner::Styled { parent, mut content, styles } => {
                    if let Some(rest) = rest {
                        content.push(rest);
                    }

                    let rest = content.pack().styled_with_map(styles);
                    if let Some(parent) = parent {
                        collect_inner(*parent, engine, Some(rest))?
                    } else {
                        rest.into_value()
                    }
                }
                Joiner::Recipe { parent, recipe, mut content } => {
                    if let Some(rest) = rest {
                        content.push(rest);
                    }

                    let rest = content.pack().styled_with_recipe(engine, recipe)?;
                    if let Some(parent) = parent {
                        collect_inner(*parent, engine, Some(rest))?
                    } else {
                        rest.into_value()
                    }
                }
            })
        }

        collect_inner(self, engine, None)
    }
}

/// Workaround for some lifetime issues.
fn cast_lifetime<'a: 'b, 'b, T: ?Sized + 'a>(value: &'a T) -> &'b T {
    value
}
