use typst_library::diag::{warning, At, SourceResult};
use typst_library::foundations::{
    Content, Label, NativeElement, Repr, Smart, Symbol, Unlabellable, Value,
};
use typst_library::math::EquationElem;
use typst_library::model::{
    EmphElem, EnumItem, HeadingElem, LinkElem, ListItem, ParbreakElem, RefElem,
    StrongElem, Supplement, TermItem, Url,
};
use typst_library::text::{
    LinebreakElem, RawContent, RawElem, SmartQuoteElem, SpaceElem, TextElem,
};
use typst_syntax::ast::{self, AstNode};
use typst_utils::PicoStr;

use crate::{Eval, Vm};

impl Eval for ast::Markup<'_> {
    type Output = Content;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        eval_markup(vm, &mut self.exprs())
    }
}

/// Evaluate a stream of markup.
fn eval_markup<'a>(
    vm: &mut Vm,
    exprs: &mut impl Iterator<Item = ast::Expr<'a>>,
) -> SourceResult<Content> {
    let flow = vm.flow.take();
    let mut seq = Vec::with_capacity(exprs.size_hint().1.unwrap_or_default());

    while let Some(expr) = exprs.next() {
        match expr {
            ast::Expr::SetRule(set) => {
                let styles = set.eval(vm)?;
                if vm.flow.is_some() {
                    break;
                }

                seq.push(eval_markup(vm, exprs)?.styled_with_map(styles))
            }
            ast::Expr::ShowRule(show) => {
                let recipe = show.eval(vm)?;
                if vm.flow.is_some() {
                    break;
                }

                let tail = eval_markup(vm, exprs)?;
                seq.push(tail.styled_with_recipe(&mut vm.engine, vm.context, recipe)?)
            }
            expr => match expr.eval(vm)? {
                Value::Label(label) => {
                    if let Some(elem) =
                        seq.iter_mut().rev().find(|node| !node.can::<dyn Unlabellable>())
                    {
                        if elem.label().is_some() {
                            vm.engine.sink.warn(warning!(
                                elem.span(), "content labelled multiple times";
                                hint: "only the last label is used, the rest are ignored",
                            ));
                        }

                        *elem = std::mem::take(elem).labelled(label);
                    } else {
                        vm.engine.sink.warn(warning!(
                            expr.span(),
                            "label `{}` is not attached to anything",
                            label.repr()
                        ));
                    }
                }
                value => seq.push(value.display().spanned(expr.span())),
            },
        }

        if vm.flow.is_some() {
            break;
        }
    }

    if flow.is_some() {
        vm.flow = flow;
    }

    Ok(Content::sequence(seq))
}

impl Eval for ast::Text<'_> {
    type Output = Content;

    fn eval(self, _: &mut Vm) -> SourceResult<Self::Output> {
        Ok(TextElem::packed(self.get().clone()))
    }
}

impl Eval for ast::Space<'_> {
    type Output = Content;

    fn eval(self, _: &mut Vm) -> SourceResult<Self::Output> {
        Ok(SpaceElem::shared().clone())
    }
}

impl Eval for ast::Linebreak<'_> {
    type Output = Content;

    fn eval(self, _: &mut Vm) -> SourceResult<Self::Output> {
        Ok(LinebreakElem::shared().clone())
    }
}

impl Eval for ast::Parbreak<'_> {
    type Output = Content;

    fn eval(self, _: &mut Vm) -> SourceResult<Self::Output> {
        Ok(ParbreakElem::shared().clone())
    }
}

impl Eval for ast::Escape<'_> {
    type Output = Value;

    fn eval(self, _: &mut Vm) -> SourceResult<Self::Output> {
        Ok(Value::Symbol(Symbol::runtime_char(self.get())))
    }
}

impl Eval for ast::Shorthand<'_> {
    type Output = Value;

    fn eval(self, _: &mut Vm) -> SourceResult<Self::Output> {
        Ok(Value::Symbol(Symbol::runtime_char(self.get())))
    }
}

impl Eval for ast::SmartQuote<'_> {
    type Output = Content;

    fn eval(self, _: &mut Vm) -> SourceResult<Self::Output> {
        Ok(SmartQuoteElem::new().with_double(self.double()).pack())
    }
}

impl Eval for ast::Strong<'_> {
    type Output = Content;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let body = self.body();
        if body.exprs().next().is_none() {
            vm.engine
                .sink
                .warn(warning!(
                    self.span(), "no text within stars";
                    hint: "using multiple consecutive stars (e.g. **) has no additional effect",
                ));
        }

        Ok(StrongElem::new(body.eval(vm)?).pack())
    }
}

impl Eval for ast::Emph<'_> {
    type Output = Content;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let body = self.body();
        if body.exprs().next().is_none() {
            vm.engine
                .sink
                .warn(warning!(
                    self.span(), "no text within underscores";
                    hint: "using multiple consecutive underscores (e.g. __) has no additional effect"
                ));
        }

        Ok(EmphElem::new(body.eval(vm)?).pack())
    }
}

impl Eval for ast::Raw<'_> {
    type Output = Content;

    fn eval(self, _: &mut Vm) -> SourceResult<Self::Output> {
        let lines = self.lines().map(|line| (line.get().clone(), line.span())).collect();
        let mut elem = RawElem::new(RawContent::Lines(lines)).with_block(self.block());
        if let Some(lang) = self.lang() {
            elem.push_lang(Some(lang.get().clone()));
        }
        Ok(elem.pack())
    }
}

impl Eval for ast::Link<'_> {
    type Output = Content;

    fn eval(self, _: &mut Vm) -> SourceResult<Self::Output> {
        let url = Url::new(self.get().clone()).at(self.span())?;
        Ok(LinkElem::from_url(url).pack())
    }
}

impl Eval for ast::Label<'_> {
    type Output = Value;

    fn eval(self, _: &mut Vm) -> SourceResult<Self::Output> {
        Ok(Value::Label(
            Label::new(PicoStr::intern(self.get())).expect("unexpected empty label"),
        ))
    }
}

impl Eval for ast::Ref<'_> {
    type Output = Content;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let target = Label::new(PicoStr::intern(self.target()))
            .expect("unexpected empty reference");
        let mut elem = RefElem::new(target);
        if let Some(supplement) = self.supplement() {
            elem.push_supplement(Smart::Custom(Some(Supplement::Content(
                supplement.eval(vm)?,
            ))));
        }
        Ok(elem.pack())
    }
}

impl Eval for ast::Heading<'_> {
    type Output = Content;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let depth = self.depth();
        let body = self.body().eval(vm)?;
        Ok(HeadingElem::new(body).with_depth(depth).pack())
    }
}

impl Eval for ast::ListItem<'_> {
    type Output = Content;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        Ok(ListItem::new(self.body().eval(vm)?).pack())
    }
}

impl Eval for ast::EnumItem<'_> {
    type Output = Content;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let body = self.body().eval(vm)?;
        let mut elem = EnumItem::new(body);
        if let Some(number) = self.number() {
            elem.push_number(Some(number));
        }
        Ok(elem.pack())
    }
}

impl Eval for ast::TermItem<'_> {
    type Output = Content;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let term = self.term().eval(vm)?;
        let description = self.description().eval(vm)?;
        Ok(TermItem::new(term, description).pack())
    }
}

impl Eval for ast::Equation<'_> {
    type Output = Content;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let body = self.body().eval(vm)?;
        let block = self.block();
        Ok(EquationElem::new(body).with_block(block).pack())
    }
}
