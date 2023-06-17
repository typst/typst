pub mod geom;
pub(crate) use geom::*;

pub mod ir;
pub(crate) use ir::*;

pub(crate) mod lowering;
pub(crate) use lowering::LowerBuilder;

pub(crate) mod vm;
pub(crate) use vm::{GroupContext, RenderVm, TransformContext};

pub(crate) mod codegen;
pub(crate) use codegen::{SvgText, SvgTextBuilder, SvgTextNode};
