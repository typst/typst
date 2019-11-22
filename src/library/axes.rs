use crate::func::prelude::*;

/// 📐 `align`: Aligns content in different ways.
#[derive(Debug, PartialEq)]
pub struct Align {
    body: Option<SyntaxTree>,
    positional_1: Option<AlignSpecifier>,
    positional_2: Option<AlignSpecifier>,
    primary: Option<AlignSpecifier>,
    secondary: Option<AlignSpecifier>,
    horizontal: Option<AlignSpecifier>,
    vertical: Option<AlignSpecifier>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum AlignSpecifier {
    Origin,
    Center,
    End,
    Left,
    Right,
    Top,
    Bottom,
}

function! {
    data: Align,

    parse(args, body, ctx) {
        let body = parse!(optional: body, ctx);

        let mut align = Align {
            body,
            positional_1: None,
            positional_2: None,
            primary: None,
            secondary: None,
            horizontal: None,
            vertical: None,
        };

        if let Some(arg) = args.get_pos_opt::<ArgIdent>()? {
            align.positional_1 = Some(parse_align_specifier(arg)?);
        }

        if let Some(arg) = args.get_pos_opt::<ArgIdent>()? {
            align.positional_2 = Some(parse_align_specifier(arg)?);
        }

        let mut parse_arg = |axis, target: &mut Option<AlignSpecifier>| {
            Ok(if let Some(arg) = args.get_key_opt::<ArgIdent>(axis)? {
                if target.is_none() {
                    *target = Some(parse_align_specifier(arg)?);
                } else {
                    err!("duplicate alignment specification for {} axis", axis);
                }
            })
        };

        parse_arg("primary", &mut align.primary)?;
        parse_arg("secondary", &mut align.secondary)?;
        parse_arg("horizontal", &mut align.horizontal)?;
        parse_arg("vertical", &mut align.vertical)?;

        args.done()?;

        Ok(align)
    }

    layout(this, ctx) {
        let mut axes = ctx.axes;
        let primary_horizontal = axes.primary.axis.is_horizontal();

        let mut primary = false;
        let mut secondary = false;

        let mut set_axis = |is_primary: bool, spec: Option<AlignSpecifier>| -> LayoutResult<()> {
            if let Some(spec) = spec {
                let (axis, was_set, name) = match is_primary {
                    true => (&mut axes.primary, &mut primary, "primary"),
                    false => (&mut axes.secondary, &mut secondary, "secondary"),
                };

                if *was_set {
                    panic!("duplicate alignment for {} axis", name);
                }

                *was_set = true;

                let horizontal = axis.axis.is_horizontal();
                let alignment = generic_alignment(spec, horizontal)?;

                if axis.alignment == Alignment::End && alignment == Alignment::Origin {
                    axis.expand = true;
                }

                axis.alignment = alignment;
            }

            Ok(())
        };

        if let Some(spec) = this.positional_1 {
            let positional = generic_alignment(spec, primary_horizontal).is_ok();
            set_axis(positional, this.positional_1)?;
        }

        if let Some(spec) = this.positional_2 {
            let positional = generic_alignment(spec, primary_horizontal).is_ok();
            set_axis(positional, this.positional_2)?;
        }

        set_axis(true, this.primary)?;
        set_axis(false, this.secondary)?;
        set_axis(primary_horizontal, this.horizontal)?;
        set_axis(!primary_horizontal, this.vertical)?;

        Ok(match &this.body {
            Some(body) => commands![AddMultiple(
                layout_tree(body, LayoutContext {
                    axes,
                    .. ctx.clone()
                })?
            )],
            None => commands![Command::SetAxes(axes)]
        })
    }
}

fn parse_align_specifier(arg: Spanned<&str>) -> ParseResult<AlignSpecifier> {
    Ok(match arg.val {
        "origin" => AlignSpecifier::Origin,
        "center" => AlignSpecifier::Center,
        "end" => AlignSpecifier::End,
        "left" => AlignSpecifier::Left,
        "right" => AlignSpecifier::Right,
        "top" => AlignSpecifier::Top,
        "bottom" => AlignSpecifier::Bottom,
        s => err!("invalid alignment specifier: {}", s),
    })
}

fn generic_alignment(spec: AlignSpecifier, horizontal: bool) -> LayoutResult<Alignment> {
    use AlignSpecifier::*;
    Ok(match (spec, horizontal) {
        (Origin, _) | (Left, true) | (Top, false) => Alignment::Origin,
        (Center, _) => Alignment::Center,
        (End, _) | (Right, true) | (Bottom, false) => Alignment::End,
        _ => Err(LayoutError::UnalignedAxis("invalid alignment"))?,
    })
}
