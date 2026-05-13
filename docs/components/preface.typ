// The preface of the docs. Only used in the paged version.

#import "system.typ": fonts, logotype

#show link: underline

#page({
  logotype
  v(2fr)
  title[Typst Documentation]
  text(1.5em, luma(100))[Version: #sys.version]
  v(1fr)
})

#page[
  #v(4fr)

  Copyright #sym.copyright
  2019 -- #datetime.today().display("[year]")
  #context document.author.join(", ", last: ", and ").

  The contents of this document and the Typst compiler are licensed under the terms of the Apache License, Version 2.0.

  This documentation is open source and open for contribution on https://github.com/typst/typst. It has been typeset using Typst #sys.version, #fonts.body, and #fonts.mono.

  Published by Typst GmbH, Heidestraße 34, 10557 Berlin, Germany.

  https://typst.app/

  #v(1fr)
]
