#import "@local/template:1.10.0": *

#let template(body) = panic("hey")

#show block: template

#block()

//#show: templates.book.with(title: none, str-title: auto)

//#error("panic!")
//
//#definition(1)[
//  Test
//]
