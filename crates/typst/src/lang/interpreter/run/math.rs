use typst_syntax::Span;

use crate::diag::{At, SourceResult};
use crate::engine::Engine;
use crate::foundations::{Content, NativeElement, SequenceElem};
use crate::lang::interpreter::Vm;
use crate::lang::opcodes::{Attach, Delimited, Equation, Frac, Root};
use crate::math::{AttachElem, EquationElem, FracElem, LrElem, RootElem};

use super::SimpleRun;

impl SimpleRun for Delimited {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the left delimiter, body, and right delimiter.
        let left: Content = vm.read(self.left).clone().display();
        let body: Content = vm.read(self.body).clone().display();
        let right: Content = vm.read(self.right).clone().display();

        // Make the value into a delimited.
        let value = LrElem::new(
            SequenceElem::new(vec![left.into(), body.into(), right.into()])
                .pack()
                .spanned(span),
        );

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Attach {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the base, top, and bottom.
        let base = vm.read(self.base);
        let top = vm.read(self.top);
        let bottom = vm.read(self.bottom);

        // Make the value into an attach.
        let mut value = AttachElem::new(base.clone().display());

        if let Some(top) = top {
            value.push_t(Some(top.clone().display()));
        }

        if let Some(bottom) = bottom {
            value.push_b(Some(bottom.clone().display()));
        }

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Frac {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the numerator and denominator.
        let numerator = vm.read(self.numerator);
        let denominator = vm.read(self.denominator);

        // Make the value into a fraction.
        let value =
            FracElem::new(numerator.clone().display(), denominator.clone().display());

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Root {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the degree and radicand.
        let degree = vm.read(self.degree);
        let radicand = vm.read(self.radicand);

        // Make the value into a root.
        let mut value = RootElem::new(radicand.clone().display());

        if let Some(degree) = degree {
            value.push_index(Some(degree.clone().display()));
        }

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Equation {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value);

        // Make the value into an equation.
        let value =
            EquationElem::new(value.clone().cast().at(span)?).with_block(self.block);

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}
