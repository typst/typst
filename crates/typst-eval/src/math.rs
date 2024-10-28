use ecow::eco_format;
use typst_library::diag::{At, SourceResult};
use typst_library::foundations::{Content, NativeElement, Symbol, Value};
use typst_library::math::{
    AlignPointElem, AttachElem, FracElem, LrElem, PrimesElem, RootElem,
};
use typst_library::text::TextElem;
use typst_syntax::ast::{self, AstNode};

use crate::{Eval, Vm};

impl Eval for ast::Math<'_> {
    type Output = Content;
    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        Ok(Content::sequence(
            self.exprs()
                .map(|expr| expr.eval_display(vm))
                .collect::<SourceResult<Vec<_>>>()?,
        ))
    }
}

impl Eval for ast::MathIdent<'_> {
    type Output = Value;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        vm.scopes.get_in_math(&self).cloned().at(self.span())
    }
}

impl Eval for ast::MathShorthand<'_> {
    type Output = Value;

    fn eval(self, _: &mut Vm) -> SourceResult<Self::Output> {
        Ok(Value::Symbol(Symbol::single(self.get().into())))
    }
}

impl Eval for ast::MathAlignPoint<'_> {
    type Output = Content;

    fn eval(self, _: &mut Vm) -> SourceResult<Self::Output> {
        Ok(AlignPointElem::shared().clone())
    }
}

impl Eval for ast::MathDelimited<'_> {
    type Output = Content;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let open = self.open().eval_display(vm)?;
        let body = self.body().eval(vm)?;
        let close = self.close().eval_display(vm)?;
        Ok(LrElem::new(open + body + close).pack())
    }
}

impl Eval for ast::MathAttach<'_> {
    type Output = Content;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let base = self.base().eval_display(vm)?;
        let mut elem = AttachElem::new(base);

        if let Some(expr) = self.top() {
            elem.push_t(Some(expr.eval_display(vm)?));
        }

        // Always attach primes in scripts style (not limits style),
        // i.e. at the top-right corner.
        if let Some(primes) = self.primes() {
            elem.push_tr(Some(primes.eval(vm)?));
        }

        if let Some(expr) = self.bottom() {
            elem.push_b(Some(expr.eval_display(vm)?));
        }

        Ok(elem.pack())
    }
}

impl Eval for ast::MathPrimes<'_> {
    type Output = Content;

    fn eval(self, _: &mut Vm) -> SourceResult<Self::Output> {
        Ok(PrimesElem::new(self.count()).pack())
    }
}

impl Eval for ast::MathFrac<'_> {
    type Output = Content;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let num = self.num().eval_display(vm)?;
        let denom = self.denom().eval_display(vm)?;
        Ok(FracElem::new(num, denom).pack())
    }
}

impl Eval for ast::MathRoot<'_> {
    type Output = Content;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let index = self.index().map(|i| TextElem::packed(eco_format!("{i}")));
        let radicand = self.radicand().eval_display(vm)?;
        Ok(RootElem::new(radicand).with_index(index).pack())
    }
}

trait ExprExt {
    fn eval_display(&self, vm: &mut Vm) -> SourceResult<Content>;
}

impl ExprExt for ast::Expr<'_> {
    fn eval_display(&self, vm: &mut Vm) -> SourceResult<Content> {
        Ok(self.eval(vm)?.display().spanned(self.span()))
    }
}
