use typst_syntax::ast::{self, AstNode};

use crate::{
    diag::{bail, error, SourceResult},
    foundations::Value,
    World,
};

use super::{Eval, Vm};

impl Eval for ast::Write18<'_> {
    type Output = Value;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        if let Err(err) = vm.engine.world.run_shell_command(&self.command()) {
            bail!(self.span(), "\\write18{{…}} failed {err}");
        }

        Ok(Value::None)
    }
}

impl Eval for ast::InputPipe<'_> {
    type Output = Value;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let out = vm
            .engine
            .world
            .run_shell_command(&self.command())
            .map_err(|err| vec![error!(self.span(), "\\input|\"…\" failed {err}")])?;

        Ok(Value::Str(out.into()))
    }
}
