#import "../../../components/index.typ": (
  classnames, colors, docs-category, docs-section, fonts, icon,
  paged-heading-offset, prose-styling, search-box, ty-pill, use-icon,
)

// Symbols that are not rendered as themselves because they would be invisible.
#let symbol-overrides = (
  " ": "␣",
  "\u{00a0}": "nbsp",
  "\u{202F}": "nnbsp",
  "\u{00ad}": "shy",
  "\u{2002}": "ensp",
  "\u{2003}": "emsp",
  "\u{2004}": "⅓emsp",
  "\u{2005}": "¼emsp",
  "\u{2006}": "⅙emsp",
  "\u{205f}": "mmsp",
  "\u{2007}": "numsp",
  "\u{2008}": "puncsp",
  "\u{2009}": "thinsp",
  "\u{200a}": "hairsp",
  "\u{2060}": "wjoin",
  "\u{200D}": "zwj",
  "\u{200C}": "zwnj",
  "\u{200B}": "zwsp",
  "\u{200E}": "lrm",
  "\u{200F}": "rlm",
)

// Additional keywords for some symbols. Used to improve symbol search.
#let symbol-keywords = (
  "ħ": ("hbar",),
  "⇒": ("implies",),
  "⟹": ("implies",),
  "⇔": ("iff",),
  "⅋": ("par",),
  "א": ("alef",),
  "ב": ("bet",),
  "ד": ("dalet",),
)

// Human-facing names of the math classes.
#let math-class-names = (
  "normal": "Normal",
  "alphabetic": "Alphabetic",
  "binary": "Binary",
  "closing": "Closing",
  "diacritic": "Diacritic",
  "fence": "Fence",
  "glyphpart": "Glyph Part",
  "large": "Large",
  "opening": "Opening",
  "punctuation": "Punctuation",
  "relation": "Relation",
  "space": "Space",
  "unary": "Unary",
  "vary": "Vary",
  "special": "Special",
)

// Fonts that are used to display the symbols.
#let symbol-fonts = (
  // TODO: Maybe prefer Libertinus Serif for the full default look?
  // Note: Keep in sync with docs.css.
  fonts.body,
  "New Computer Modern Math",
  "Twitter Color Emoji",
)

#let copy-button() = html.button(
  class: "copy",
  icon(16, "copy", "Copy"),
)

// The HTML template that is instantiated for the popup.
#let flyout-template() = {
  html.template(id: "flyout-template", html.div(class: "symbol-flyout", {
    html.div(class: "info", {
      html.button(class: "main", html.span(class: "sym"))
      html.div({
        html.h3(html.span(class: "unic-name"))
        html.p(class: "sym-deprecation", {
          use-icon(16, "warn", "Warning")
          html.span(class: "text")[This symbol is deprecated]
        })
        html.p(class: "sym-name", {
          [Name: ]
          html.code()
          copy-button()
        })
        html.p(class: "shorthand", {
          [Shorthand: ]
          html.code(class: "typ-escape")
          copy-button()
          html.span(class: "remark")
        })
        html.p(class: "codepoint", {
          [Escape: ]
          html.code(
            class: "typ-escape",
            "\\u{" + html.span(class: "value") + "}",
          )
          copy-button()
        })
        html.p(class: "accent", {
          [Accent: ]
          icon(16, "check", "Yes")
        })
        html.p(class: "math-class", {
          [Math Class: ]
          html.span(class: "value")
        })
        html.p(class: "latex-name", {
          [LaTeX: ]
          html.code()
        })
      })
    })
    html.div(class: "variants-box", {
      html.h4[Variants]
      html.ul(class: "symbol-grid")
    })
  }))

  html.template(id: "flyout-sym-row", {
    html.li(html.button(html.span(class: "sym")))
  })
}

// One list entry in a symbol list or cell in a symbol grid.
#let symbol-entry(
  name,
  value,
  deprecation,
  alternates,
  shorthand: false,
) = {
  let attrs = (
    // This can possibly produce IDs that are not valid CSS identifiers, but we
    // don't need that so this is fine.
    id: "symbol-" + name,
    data-codex-name: if not shorthand { name },
    data-unic-name: stdx.unicode-name(value),
    data-latex-name: stdx.latex-name(value),
    data-keywords: symbol-keywords.at(value, default: ()).join(" ", default: ""),
    data-value: value,
    data-accent: if stdx.is-accent(value) { "true" },
    data-alternates: alternates.join(" ", default: none),
    data-markup-shorthand: stdx.shorthands.markup.at(value, default: none),
    data-math-shorthand: stdx.shorthands.math.at(value, default: none),
    data-math-class: {
      let class = stdx.math-class(value)
      if class != none { math-class-names.at(class) }
    },
    data-override: symbol-overrides.at(value, default: none),
    data-deprecation: deprecation,
  )

  let body = symbol-overrides.at(value, default: value)

  context if target() == "paged" {
    let style = if value in symbol-overrides {
      (fill: colors.dark-gray.shade-10, weight: 500, style: "italic")
    }
    box(width: 5em, h(1fr) + text(font: symbol-fonts, ..style, body) + h(1em))
    let wrapper = if deprecation != none { strike } else { it => it }
    if shorthand {
      text(fill: colors.text.syntax.teal, wrapper(raw(name)))
    } else {
      wrapper(raw(name))
    }
  } else {
    html.elem(
      "li",
      attrs: attrs.pairs().filter(p => p.last() != none).to-dict(),
      html.button({
        html.span(class: "sym", body)
        html.code(
          ..if shorthand { (class: "typ-escape") },
          if not shorthand {
            show ".": it => it + html.wbr()
            name
          } else {
            name
          },
        )
      }),
    )
  }
}

// Computes the entries to display in a symbol list / grid.
#let symbol-list-entries(mod, prefix) = {
  let entries = ()
  for (name, s) in dictionary(mod) {
    if type(s) == module {
      let nested-prefix = if prefix == none {
        name
      } else {
        prefix + "." + name
      }
      entries += symbol-list-entries(s, nested-prefix)
      continue
    }

    let info = stdx.describe(s)
    let binding = stdx.binding(mod, name)

    for (variant, value, deprecation) in info.variants {
      if deprecation == none and binding.deprecation != none {
        deprecation = binding.deprecation.message
      }

      let complete(v) = {
        if prefix != none {
          prefix + "."
        }
        name
        if v != "" {
          "." + v
        }
      }

      let full-name = complete(variant)
      let alternates = info
        .variants
        .map(((variant, ..)) => complete(variant))
        .filter(v => v != full-name)

      entries.push(symbol-entry(
        full-name,
        value,
        deprecation,
        alternates,
      ))
    }
  }
  entries
}

// A list / grid of symbols.
//
// Any page containing this should also contain a single `flyout-template()` in
// website export.
#let symbol-list(entries, emoji: false) = {
  context if target() == "paged" {
    columns(2, list(..entries, marker: none))
  } else {
    html.ul(
      class: classnames("symbol-grid", emoji: emoji),
      entries.join(),
    )
  }
}

// A list / grid of symbols with their names.
#let symbol-name-list(mod, emoji: false) = {
  let entries = symbol-list-entries(mod, none)
  symbol-list(entries, emoji: emoji)
}

// A list / grid of symbols with their shorthands.
#let symbol-shorthand-list(shorthands) = {
  let entries = shorthands.pairs().map(((value, shorthand)) => {
    symbol-entry(
      shorthand,
      value,
      none,
      (),
      shorthand: true,
    )
  })
  symbol-list(entries)
}

#docs-category(
  title: "Symbols",
  description: "Predefined symbols in Typst.",
  category: "symbols",
)[
  The @sym and @emoji modules give names to symbols and emoji to make them easy to insert with a normal keyboard. Alternatively, you can also always directly enter Unicode symbols into your text and formulas. In addition to the symbols listed below, math mode defines `dif` and `Dif`. These are not normal symbol values because they also affect spacing and font style.

  You can define custom symbols with the constructor function of the @symbol type.

  = Shorthands <shorthands>
  Shorthands are concise sequences of characters that evoke specific glyphs. Shorthands and other ways to produce symbols can be used interchangeably. You can use different sets of shorthands in math and markup mode. Some shorthands, like `~` for a non-breaking space produce non-printing symbols, which are indicated with gray placeholder text.

  You can deactivate a shorthand's interpretation by escaping any of its characters. If you escape a single character in a shorthand, the remaining unescaped characters may form a different shorthand.

  == Within Markup Mode <within-markup-mode>
  #symbol-shorthand-list(stdx.shorthands.markup)

  == Within Math Mode <within-math-mode>
  #symbol-shorthand-list(stdx.shorthands.math)

  #context if target() == "html" {
    flyout-template()
  }
]

#let symbols-section(..args, mod: none, emoji: false, body) = docs-section(
  ..args,
  kind: "Symbols",
  {
    prose-styling(body)
    context if target() == "html" {
      html.div(class: "symbol-hint", {
        par[Click on a #ty-pill(symbol) to copy it to the clipboard.]
        search-box(id: "symbol-search", placeholder: "Search in symbols")
      })
    }
    symbol-name-list(mod, emoji: emoji)
    context if target() == "html" {
      flyout-template()
    }
  },
)

#show: paged-heading-offset.with(1)

#symbols-section(
  title: "General Symbols",
  route: "/reference/symbols/sym",
  def-target: <sym>,
  description: "Documentation for the `sym` module, which gives names to symbols.",
  mod: sym,
)[
  Named general symbols.

  For example, `[#sym.arrow]` produces the → symbol. Within @math[math], these symbols can be used without the `[#sym.]` prefix.

  The `d` in an integral's `dx` can be written as `[$dif x$]`. Outside math formulas, `dif` can be accessed as `math.dif`.
]

#symbols-section(
  title: "Emoji",
  route: "/reference/symbols/emoji",
  def-target: <emoji>,
  description: "Documentation for the `emoji` module, which gives names to emoji.",
  mod: emoji,
  emoji: true,
)[
  Named emojis.

  For example, `[#emoji.face]` produces the 😀 emoji. If you frequently use certain emojis, you can also import them from the `emoji` module (`[#import emoji: face]`) to use them without the `emoji.` prefix.
]
