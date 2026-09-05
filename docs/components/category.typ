#import "system.typ": colors
#import "base.typ": (
  classnames, folding-details, heading-offset, html-heading-n, icon, labelled,
  oneliner, paged-heading-offset, short-or-long, small, text-with-code,
  title-case, to-func, use-icon, with-tooltip,
)
#import "example.typ": example, example-like-block
#import "linking.typ": def-dest, def-label, register-def
#import "live.typ": item-source-link, live-docs
#import "pill.typ": ty-pill
#import "reflect.typ": cast-strings, flat-types, std-path-of
#import "search.typ": register-index-item
#import "section.typ": docs-section
#import "styling.typ": prose-styling
#import "table.typ": docs-table

// Build a scope from a module and binding info.
#let scope-from(val, info) = {
  let mod = if type(val) == function {
    stdx.describe(val).scope
  } else {
    val
  }
  (val: val, dict: dictionary(mod), info: info)
}

// Build the scope information for a module.
//
// Requires the parent module and the name of the module.
#let scope(parent, key) = scope-from(
  dictionary(parent).at(key),
  stdx.binding(parent, key),
)

// Get nested binding information, feature gates of the parent scope info will
// be forwarded to the nested item info.
#let nested-binding(scope, key) = {
  let info = stdx.binding(scope.val, key)
  if info.feature == none {
    info.feature = scope.info.feature
  }
  info
}

// Displays the overview box for a function signature.
#let param-signature(func, path, params, returns) = context {
  let func-label = def-label(func)

  if target() == "paged" {
    example-like-block({
      set par(leading: 0.8em)
      let path = path.map(raw)
      path.last() = text(fill: colors.text.syntax.blue, path.last())
      path.join(`.`)
      `(`
      for param in params {
        let param-label = label(str(func-label) + "." + param.name)
        if params.len() > 1 [\ `  `]
        if param.variadic [`..`]
        if param.named {
          ref(param-label, supplement: raw(param.name))
          `: `
        }
        flat-types(param.input).map(ty-pill).join(h(0.3em))
        if params.len() > 1 [`,`]
      }
      if params.len() > 1 [\ ]
      `)`

      let returns = flat-types(returns)
      if returns.len() > 0 {
        raw(" " + sym.arrow.r + " ")
        returns.map(ty-pill).join(h(0.3em))
      }
    })
  } else {
    let single = params.len() <= 1
    let container = if single { html.span } else { html.div }
    html.div(
      class: classnames("code", "code-definition", single-arg: single),
      {
        let path = path
        path.last() = html.span(class: "typ-func", path.last())
        path.join(html.span(class: "typ-punct", [.]))
        [(]
        container(class: "arguments", {
          for param in params {
            let param-label = label(str(func-label) + "." + param.name)
            html.span(class: "overview-param", context {
              if param.variadic [..]
              if param.named { ref(param-label, supplement: param.name) + [: ] }
              flat-types(param.input).map(ty-pill).join()
              if params.len() > 1 [,]
            })
          }
        })
        [)]

        let returns = flat-types(returns)
        if returns.len() > 0 {
          " " + sym.arrow.r + " "
          returns.map(ty-pill).join()
        }
      },
    )
  }
}

// Displays parameter modifiers.
#let modifier-list(..modifiers) = {
  let modifiers = modifiers.pos()
  context if target() == "paged" {
    let gap = h(1em)
    text(style: "italic", modifiers.map(small).join(gap))
  } else {
    modifiers.map(modifier => html.small(modifier, class: "mod")).join()
  }
}

// Displays the contents of a parameter heading, including the pills, parameter
// attributes (named / variadic / ...), and default value.
//
// When this is changed, `docs/content/reference/index.typ` needs to be updated
#let param-headline(param, input-types) = {
  let pills = input-types.map(ty-pill)
  let modifiers = ()
  if param.required {
    modifiers.push[Required]
  }
  if param.positional {
    if param.named {
      // Note: This never seems to trigger in practice.
      modifiers.push(with-tooltip[Shorthand][
        This named parameter can also be specified as a positional argument.
      ])
    } else {
      modifiers.push(with-tooltip[Positional][
        Positional parameters are specified in order, without names.
      ])
    }
  }
  if param.variadic {
    modifiers.push(with-tooltip[Variadic][
      Variadic parameters can be specified multiple times.
    ])
  }
  if param.settable {
    modifiers.push(with-tooltip[Settable][
      Settable parameters can be customized for all following uses of the
      function with a `set` rule.
    ])
  }

  raw(param.name)

  let default = if not param.required and "default" in param {
    [Default: ]
    raw(lang: "typc", repr(param.default))
  }

  context if target() == "paged" {
    // Undo the heading weight.
    set text(weight: 400)
    let gap = h(1em)
    gap
    pills.join[ #small[or] ]
    gap
    modifier-list(..modifiers)
    if default != none {
      // The following four lines ensure that there the default has a minimum
      // distances to the modifiers, while being flush-right even if it is alone
      // in its line.
      gap
      box(width: 0pt)
      h(1fr)
      sym.wj
      box(small(default))
    }
  } else {
    html.div(class: "additional-info", {
      html.div(pills.join[ #small[or] ])
      modifier-list(..modifiers)
    })
    if default != none {
      html.small(class: "default", default)
    }
  }
}

// Displays the documentation for a single parameter.
#let param-details(func, param, base-label, muted: false) = {
  let param-label = label(str(def-label(func)) + "." + param.name)
  let input-types = flat-types(param.input)

  {
    show heading: it => {
      // The expression `figure.caption` is ambiguous. It's both an element and
      // a contextual parameter access. This is really a language-level problem
      // but reflects in linking ambiguity in the docs. To avoid a linking
      // error, we simply prefer the element always.
      let skip = func == figure and param.name == "caption"
      if not skip {
        register-def(param-label, it.location())
      }
      if not muted {
        register-index-item(
          kind: "Parameter of " + repr(func),
          title: title-case(param.name),
          dest: it.location(),
        )
      }
      it
    }
    let title = short-or-long(param.name, param-headline(param, input-types))
    labelled(
      heading(depth: 2, title),
      label(str(base-label) + "-" + param.name),
    )
  }

  {
    show raw.where(lang: "example"): example.with(folding: true)
    set par(spacing: 1em)
    heading-offset(2, prose-styling(
      live-docs(param.docs, param.def-site),
      base-target: param-label,
    ))
  }

  // For things that take constant strings, we show a table with additional
  // details.
  if str in input-types {
    let strings = cast-strings(param.input)
    if strings.len() > 0 {
      let t = docs-table(
        table.header[Variant][Details],
        ..strings
          .map(((variant, details)) => (
            context if target() == "paged" {
              raw(lang: "typc", "\"" + variant + "\"")
            } else {
              html.code(class: "typ-str", "\"" + variant + "\"")
            },
            live-docs(details, none),
          ))
          .flatten(),
      )
      context if strings.len() > 10 and target() == "html" {
        folding-details(title: [View options], t)
      } else {
        t
      }
    }
  }
}

// Displays a signature overview followed by docs for the individual parameters.
#let params-section(
  func,
  params,
  returns,
  base-label,
  path: auto,
  indent: false,
  muted: false,
) = {
  if path == auto {
    path = std-path-of(func).split(".")
  }

  param-signature(func, path, params, returns)

  // A bit of indent in paged export.
  show: rest => context if indent and target() == "paged" {
    pad(left: 3em, rest)
  } else {
    rest
  }

  for param in params {
    param-details(func, param, base-label, muted: muted)
  }
}

// Displays a link to the sources for an item.
//
// Requires context.
#let sources-link(info) = {
  if target() == "html" and info.def-site != none {
    let url = item-source-link(info.def-site)
    html.a(
      href: url,
      class: "sources-link",
      use-icon(16, "code", "Go to source"),
    )
  }
}

// Displays binding information, if any, such as:
// - A deprecation
// - A feature gate
#let definition-info(info) = {
  let gap = if target() == "paged" { h(0.5em, weak: true) }

  let item(size, name, alt, body) = {
    context if target() == "paged" {
      small(icon(size, name, alt) + [ ] + body)
    } else {
      html.small(class: "definition-info", {
        html.div(use-icon(size, name, alt))
        html.span(body)
      })
    }
  }

  if info.feature != none {
    gap
    item(16, "toggle", "Feature toggle", {
      [Requires the ]
      raw(info.feature)
      [ feature]
    })
  }

  if info.deprecation != none {
    gap
    item(16, "warn", "Warning", {
      text-with-code(info.deprecation.message)
      if info.deprecation.until != none {
        [; it will be removed in Typst #info.deprecation.until]
      }
    })
  }
}

// Displays additional details about a function.
//
// When the labels are changed here, `docs/content/reference/index.typ` needs to
// change, too.
#let func-subtitle(info, binding-info) = context {
  let gap = if target() == "paged" { h(0.5em, weak: true) }
  set text(0.75em)
  if info.element {
    gap
    small(with-tooltip[Element][
      Element functions can be customized with `set` and `show` rules.
    ])
  }
  if info.contextual != none and info.contextual {
    gap
    small(with-tooltip[Contextual][
      Contextual functions can only be used when the context is known.
    ])
  }
  if info.since != none {
    gap
    small[Since: #info.since]
  } else {
    panic("missing `since` for function " + info.def-site.key + " (" + stdx.str-from-path(info.def-site.path) + ")")
  }
  if binding-info != none {
    definition-info(binding-info)
  }
  sources-link(info)
}

// Displays additional details about a type.
#let ty-subtitle(ty-info, binding-info) = context {
  let gap = if target() == "paged" { h(0.5em, weak: true) }
  set text(0.75em)
  if ty-info.since != none {
    gap
    small[Since: #ty-info.since]
  } else {
    panic("missing `since` for type " + ty-info.def-site.key + " (" + stdx.str-from-path(ty-info.def-site.path) + ")")
  }
  if binding-info != none {
    definition-info(binding-info)
  }
  sources-link(ty-info)
}

// Renders documentation for a function as part of a large documentation
// section.
#let func-member(
  func,
  base-label: none,
  binding-info: none,
  definitions-section: none,
) = {
  assert.ne(binding-info, none, message: "binding-info is required")

  let info = stdx.describe(to-func(func))
  let base-label = label(str(base-label) + "-" + info.name)
  let muted = "typed-html" in info.keywords

  {
    show heading: it => {
      register-def(func, it.location())
      if not muted {
        register-index-item(
          kind: "Function",
          title: info.title,
          dest: it.location(),
          keywords: info.keywords,
        )
      }
      if target() == "paged" {
        it
      } else {
        html-heading-n(
          it.level + 1,
          class: classnames(
            "scoped-definition",
            deprecated: binding-info.deprecation != none,
          ),
          it.body,
        )
      }
    }
    let title = short-or-long(
      info.title,
      raw(info.name) + func-subtitle(info, binding-info),
    )
    labelled(heading(depth: 2, title), base-label)
  }

  {
    show raw.where(lang: "example"): example.with(folding: true, open: true)
    prose-styling(live-docs(info.docs, info.def-site), base-target: func)
  }

  let params = info.params
  let path = if params.len() >= 1 and params.first().name == "self" {
    params = params.slice(1)
    ("self", info.name)
  } else {
    auto
  }

  if "typed-html" in info.keywords {
    params = params.filter(p => not stdx.is-global-html-attr(p.name))
  }

  heading-offset(1, params-section(
    func,
    params,
    info.returns,
    base-label,
    path: path,
    indent: true,
    muted: muted,
  ))

  heading-offset(2, definitions-section(
    info.name,
    scope-from(info.scope, binding-info),
    base-label: label(str(base-label) + "-definitions"),
  ))
}

// Documents the constructor of a type.
#let constructor-section(func, path) = {
  let info = stdx.describe(func)
  let base-label = <constructor>

  let title = short-or-long(
    [Constructor],
    with-tooltip[Constructor][
      If a type has a constructor, you can call it like a function to create
      a new value of the type.
    ] + func-subtitle(info, none),
  )

  {
    show heading: it => {
      register-def(func, it.location())
      it
    }
    labelled(heading(title), base-label)
  }

  {
    show raw.where(lang: "example"): example.with(folding: true, open: true)
    prose-styling(live-docs(info.docs, info.def-site), base-target: func)
  }

  params-section(
    func,
    info.params,
    info.returns,
    path: path,
    base-label,
    indent: true,
  )
}

// Renders documentation for a type as part of a large documentation section.
#let ty-member(
  ty,
  base-label: none,
  binding-info: none,
  definitions-section: none,
) = {
  assert.ne(binding-info, none, message: "binding-info is required")

  let info = stdx.describe(ty)
  let base-label = label(str(base-label) + "-" + info.short-name)

  {
    show heading: it => {
      register-def(ty, it.location())
      register-index-item(
        kind: "Type",
        title: info.title,
        dest: it.location(),
        keywords: info.keywords,
      )
      if target() == "paged" {
        it
      } else {
        html-heading-n(
          it.level + 1,
          class: classnames(
            "scoped-definition",
            deprecated: binding-info.deprecation != none,
          ),
          it.body
        )
      }
    }
    let title = short-or-long(
      info.title,
      text(size: 13pt, ty-pill(ty, linked: false)) + ty-subtitle(info, binding-info),
    )
    labelled(heading(depth: 2, title), base-label)
  }

  {
    show raw.where(lang: "example"): example.with(folding: true, open: true)
    prose-styling(live-docs(info.docs, info.def-site), base-target: ty)
  }

  if info.constructor != none {
    heading-offset(2, constructor-section(
      info.constructor,
      std-path-of(ty).split("."),
    ))
  }

  heading-offset(2, definitions-section(
    info.short-name,
    scope-from(info.scope, binding-info),
    base-label: label(str(base-label) + "-definitions"),
  ))
}

// Renders a section that documents definitions on a type or function.
#let definitions-section(parent, scope, base-label: <definitions>) = {
  if scope.dict.len() == 0 {
    return
  }

  let nested = base-label != <definitions>
  let title = short-or-long(
    [Definitions] + if nested [ on #parent],
    with-tooltip[Definitions #if nested [on #raw(parent)]][
      Functions and types can have associated definitions. These are
      accessed by specifying the function or type, followed by a period,
      and then the definition's name.
    ],
  )

  labelled(heading(title), base-label)

  for (name, value) in scope.dict {
    if type(value) == function {
      func-member(
        value,
        base-label: base-label,
        binding-info: nested-binding(scope, name),
        definitions-section: definitions-section,
      )
    } else if type(value) == type {
      ty-member(
        value,
        base-label: base-label,
        binding-info: nested-binding(scope, name),
        definitions-section: definitions-section,
      )
    }
  }
}

// Typst does not support mutual recursion. We're emulating it by passing one
// function to the other and here we're redefining the first function so that
// it automatically gets the second one.
#let func-member = func-member.with(definitions-section: definitions-section)
#let ty-member = ty-member.with(definitions-section: definitions-section)

// Heading styling shared by function and type docs.
#let func-or-ty-section(..args) = {
  show heading.where(level: 3): set text(16pt)
  show heading.where(level: 3): set block(below: 16pt)
  docs-section(..args)
}

// Renders a section for a function.
#let func-section(
  base-route,
  name,
  func,
  info,
  binding-info,
) = {
  show: func-or-ty-section.with(
    kind: "Function",
    route: base-route + "/" + name,
    title: info.title,
    title-fmt: raw(info.name),
    subtitle: func-subtitle(info, binding-info),
    has-summary: true,
    keywords: info.keywords,
    def-target: func,
    description: "Documentation for the `" + name + "` function.",
  )

  prose-styling(
    live-docs(info.docs, info.def-site),
    base-target: func,
  )

  {
    let base-label = <parameters>
    labelled(heading[Parameters], base-label)
    params-section(
      func,
      info.params,
      info.returns,
      base-label,
    )
  }

  definitions-section(
    info.name,
    scope-from(info.scope, binding-info),
  )
}

// Renders a section for a type.
#let ty-section(base-route, name, ty, ty-info, binding-info) = {
  show: func-or-ty-section.with(
    route: base-route + "/" + name,
    title: ty-info.title,
    title-fmt: ty-pill(ty, linked: false),
    subtitle: ty-subtitle(ty-info, binding-info),
    has-summary: true,
    kind: "Type",
    keywords: ty-info.keywords,
    def-target: ty,
    description: "Documentation for the `" + name + "` type.",
  )

  prose-styling(
    live-docs(ty-info.docs, ty-info.def-site),
    base-target: ty,
  )

  if ty-info.constructor != none {
    constructor-section(
      ty-info.constructor,
      std-path-of(ty).split("."),
    )
  }

  definitions-section(
    ty-info.short-name,
    scope-from(ty-info.scope, binding-info),
  )
}

// The definition target for a group.
#let group-target(base-target, info) = {
  if "def-target" in info {
    info.def-target
  } else {
    label(str(def-label(base-target)) + ":" + info.name)
  }
}

// Renders a section for grouped definitions.
//
// An example of this are the various groups like "Attach" in the math docs.
#let group-section(base-route, base-target, info) = {
  let def-target = group-target(base-target, info)
  show: docs-section.with(
    kind: "Group",
    route: base-route + "/" + info.name,
    title: info.title,
    description: info.description,
    def-target: def-target,
  )

  prose-styling(info.docs, base-target: def-target)

  if info.items.len() > 0 {
    let base-label = <functions>
    labelled(heading[Functions], base-label)
    for (key, value) in info.items {
      func-member(
        value,
        base-label: base-label,
        binding-info: nested-binding(info.scope, key),
      )
    }
  }

  if "epilogue" in info {
    prose-styling(info.epilogue, base-target: def-target)
  }

  // This can't go into the Typed HTML group's epilogue because it should not
  // have prose styling...
  if info.title == "Typed HTML" {
    for param in stdx.describe(html.div).params {
      if stdx.is-global-html-attr(param.name) {
        param-details(html.div, param, <global-attributes>, muted: true)
      }
    }
  }
}

// Renders the "Definitions" outline in a category section.
#let category-outline(definitions, def-target) = context {
  // For paged
  let dests = ()
  let oneliners = ()
  // For HTML
  let lis = ()

  for (kind, name, value, info) in definitions {
    let def-target = if kind == "group" {
      group-target(def-target, info)
    } else {
      value
    }

    let body = if kind == "group" and not "def-target" in info {
      name
    } else {
      raw(name)
    }

    let oneliner = oneliner(info.docs)

    if target() == "paged" {
      dests.push(def-dest(def-target))
      oneliners.push(oneliner)
    } else {
      lis.push(html.li({
        link(def-dest(def-target), body)
        html.span(oneliner)
      }))
    }
  }

  if target() == "paged" {
    // We want to show a small description of the definition in question.
    // Other than that, this just recreates the default outline look.
    show outline.entry: it => {
      let loc = it.element.location()
      // This incurs quadratic runtime, but the lists are all relatively
      // small. Unfortunately, Typst exposes no way to have a location ->
      // value hash map.
      let i = dests.position(d => d == loc)
      let oneliner = oneliners.at(i)
      link(loc, it.indented(it.prefix(), {
        it.body()
        h(0.5em, weak: true)
        {
          // The trailing sentence period is a problem here because the
          // filler dots follow right after, so we trim it.
          show regex("\\.$"): ""
          small(oneliner)
        }
        box(it.fill, width: 1fr)
        [ ]
        sym.wj
        it.page()
      }))
    }

    outline(
      title: [Definitions],
      indent: 0pt,
      target: selector.or(..dests),
    )
  } else {
    [= Definitions <definitions>]
    html.ul(class: "subgridded catgrid", lis.join())
  }
}

// Builds a sorted list of definitions from the scope, filtering out grouped
// definitions and sub-categories, and including provided scope additions.
#let build-category-definitions(
  category: none,
  scope: none,
  scope-additions: (:),
  groups: (),
  sub-categories: (:),
) = {
  assert.ne(category, none, message: "category is required")
  assert.ne(scope, none, message: "scope is required")

  let definitions = {
    // Direct definitions from the scope.
    let skip = {
      groups.map(g => g.items.values()).flatten()
      sub-categories.values()
    }
    scope
      .dict
      .pairs()
      .filter(((k, v)) => (
        stdx.binding(scope.val, k).category == category
          and type(v) in (function, type)
          and v not in skip
          and not (
            (scope.val == math and k == "text") // dupe
              or (scope.val == pdf and k == "embed") // deprecated
              or (scope.val == std and k == "pattern") // deprecated
          )
      ))
      .map(((k, v)) => ("direct", k, v, stdx.describe(v)))

    // Manual ddditions.
    scope-additions.pairs().map(((k, v)) => ("addition", k, v, stdx.describe(v)))

    // Grouped definitions.
    groups.map(g => ("group", g.name, none, g))

    // Sub-categories.
    sub-categories.pairs().map(((k, v)) => ("category", k, v, stdx.describe(v)))
  }

  definitions.sorted(key: ((.., info)) => info.title)
}

// Show a list of category definitions built by `build-category-definitions()`.
#let category-definitions(
  route,
  scope,
  def-target,
  definitions,
  func-category: none,
) = {
  // The individual sections for all definitions.
  show: paged-heading-offset.with(1)

  for (kind, name, value, info) in definitions {
    if kind in ("direct", "addition", "category") {
      let binding-info = if kind != "addition" { nested-binding(scope, name) }

      if kind == "category" {
        func-category(route, name, value, info, binding-info)
      } else if type(value) == function {
        func-section(route, name, value, info, binding-info)
      } else if type(value) == type {
        ty-section(route, name, value, info, binding-info)
      }
    } else if kind == "group" {
      let group = info
      if group.at("scope", default: none) == none {
        group.scope = scope
      }
      group-section(route, def-target, group)
    }
  }
}

// Renders a section for a combined type and category (used for export formats).
#let func-category(
  base-route,
  // The name of the function.
  name,
  // The value of the function.
  func,
  // The dict returned by `stdx.describe()`.
  info,
  // The function binding information returned by `stdx.binding()`.
  binding-info,
) = context {
  let def-target = func
  let route = base-route + "/" + name
  let scope = scope-from(func, binding-info)

  let settings = query(selector(<category-settings>).within(here()))
    .map(m => m.value)
    .first(default: none)

  let definitions = build-category-definitions(
    category: name,
    scope: scope,
    ..settings,
  )

  func-or-ty-section(
    kind: "Function",
    route: route,
    title: info.title,
    title-fmt: raw(info.name),
    subtitle: func-subtitle(info, binding-info),
    has-summary: true,
    keywords: info.keywords,
    def-target: def-target,
    description: "Documentation for the `" + name + "` function.",
    {
      prose-styling(
        live-docs(info.docs, info.def-site),
        base-target: func,
      )

      {
        let base-label = <parameters>
        labelled(heading[Parameters], base-label)
        params-section(
          func,
          info.params,
          info.returns,
          base-label,
        )
      }

      if definitions.len() > 0 {
        category-outline(definitions, def-target)
      }
    },
  )

  category-definitions(
    route,
    scope,
    def-target,
    definitions,
    func-category: func-category,
  )
}

// Renders the docs for one category, including an overview section and sections
// for all definitions in the category.
#let docs-category(
  // The name of the category, e.g. `"foundations"`.
  category: none,
  // The title case name of the category.
  title: none,
  // A plain-text description for the category.
  description: none,
  // The scope from which the definitions to be documented are taken. It is
  // filtered by category.
  scope: none,
  // Additional definitions to document.
  scope-additions: (:),
  // Definitions that should be documented together as a group.
  groups: (),
  // Definitions that should be documented as a sub-category.
  sub-categories: (:),
  // General documentation prose for the category.
  body,
) = {
  assert.ne(category, none, message: "category is required")

  let route = "/reference/" + category
  let def-target = if scope == none {
    label("reference:" + category)
  } else {
    scope.val
  }

  let scope = if scope != none {
    scope
  } else {
    scope-from(std, (feature: none))
  }

  let definitions = build-category-definitions(
    category: category,
    scope: scope,
    scope-additions: scope-additions,
    groups: groups,
    sub-categories: sub-categories,
  )

  // The category overview section with an outline of definitions.
  docs-section(
    title: title,
    has-summary: true,
    route: route,
    def-target: def-target,
    kind: "Category",
    description: description,
    {
      prose-styling(body, base-target: def-target)
      if definitions.len() > 0 {
        category-outline(definitions, def-target)
      }
    },
  )

  category-definitions(
    route,
    scope,
    def-target,
    definitions,
    func-category: func-category,
  )
}
