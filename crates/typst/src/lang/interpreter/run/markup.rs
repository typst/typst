use typst_syntax::Span;

use crate::diag::{bail, At, SourceResult};
use crate::engine::Engine;
use crate::foundations::{NativeElement, Smart, Value};
use crate::lang::interpreter::Vm;
use crate::lang::opcodes::{Emph, EnumItem, Heading, ListItem, Ref, Strong, TermItem};
use crate::model::{
    EmphElem, EnumItem as EnumItemElem, HeadingElem, ListItem as ListItemElem, RefElem,
    StrongElem, Supplement, TermItem as TermItemElem,
};

use super::SimpleRun;

impl SimpleRun for Ref {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the supplement.
        let supplement = vm.read(self.supplement);

        // Read the label.
        let value = vm.read(self.label);
        let Value::Label(label) = value else {
            bail!(span, "expected label, found {}", value.ty().long_name());
        };

        // Create the reference.
        let reference = RefElem::new(*label)
            .with_supplement(Smart::Custom(Some(Supplement::Content(
                supplement.clone().display(),
            ))))
            .pack()
            .spanned(span);

        // Write the reference to the output.
        vm.write_one(self.out, reference).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Strong {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value);

        // Make the value strong.
        let value = StrongElem::new(value.clone().cast().at(span)?);

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Emph {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value);

        // Make the value emphasized.
        let value = EmphElem::new(value.clone().cast().at(span)?);

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl SimpleRun for Heading {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value and level.
        let value = vm.read(self.value);
        let level = self.level;

        // Make the value into a heading.
        let mut value = HeadingElem::new(value.clone().cast().at(span)?);

        // Set the level of the heading.
        value.push_level(Smart::Custom(level.into()));

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl SimpleRun for ListItem {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value.
        let value = vm.read(self.value);

        // Make the value into a list item.
        let value = ListItemElem::new(value.clone().cast().at(span)?);

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl SimpleRun for EnumItem {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value and number.
        let value = vm.read(self.value);
        let number = self.number.map(|number| number.get() as usize - 1);

        // Make the value into an enum item.
        let value = EnumItemElem::new(value.clone().cast().at(span)?).with_number(number);

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}

impl SimpleRun for TermItem {
    fn run(&self, span: Span, vm: &mut Vm, _: &mut Engine) -> SourceResult<()> {
        // Obtain the value and description.
        let value = vm.read(self.term);
        let description = vm.read(self.description);

        // Make the value into a term.
        let value = TermItemElem::new(
            value.clone().cast().at(span)?,
            description.clone().cast().at(span)?,
        );

        // Write the value to the output.
        vm.write_one(self.out, value.pack().spanned(span)).at(span)?;

        Ok(())
    }
}
