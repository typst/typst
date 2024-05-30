use comemo::Tracked;

use crate::engine::Engine;
use crate::foundations::{Content, Context, IntoValue, NativeElement, Recipe, SequenceElem, Styles, Unlabellable, Value};
use crate::diag::{bail, SourceResult, StrResult};
use crate::lang::ops;

/// A value joiner.
///
/// This behaves like a state machine that can be used to join values together.
#[derive(Debug, Clone)]
pub enum Joiner {
    /// Builds a value that is not *necessarily* displayed.
    Value(Value),
    /// Builds a value that is displayed.
    Display(SequenceElem),
    /// Builds a value that is styled using a given style chain.
    Styled { parent: Option<Box<Joiner>>, styles: Styles, content: SequenceElem },
    /// Builds a value that is styled using a given recipe.
    Recipe { parent: Option<Box<Joiner>>, recipe: Box<Recipe>, content: SequenceElem },
}

impl Joiner {
    #[typst_macros::time(name = "join")]
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

                    last.set_label(label);

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

                    last.set_label(label);

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

                    last.set_label(label);

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
                styles.apply_iter(to_add);
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
            recipe: Box::new(recipe),
        }
    }

    pub fn collect(self, engine: &mut Engine, context: Tracked<Context>) -> SourceResult<Value> {
        fn collect_inner(
            joiner: Joiner,
            engine: &mut Engine,
            context: Tracked<Context>,
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
                        collect_inner(*parent, engine, context, Some(rest))?
                    } else {
                        rest.into_value()
                    }
                }
                Joiner::Recipe { parent, recipe, mut content } => {
                    if let Some(rest) = rest {
                        content.push(rest);
                    }

                    let rest = content.pack().styled_with_recipe(engine, context, *recipe)?;
                    if let Some(parent) = parent {
                        collect_inner(*parent, engine, context, Some(rest))?
                    } else {
                        rest.into_value()
                    }
                }
            })
        }

        collect_inner(self, engine, context, None)
    }
}
