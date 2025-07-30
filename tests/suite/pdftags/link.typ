--- link-tags-heading-without-numbering pdftags ---
= Heading <heading>

#link(<heading>)[link to heading]

--- link-tags-heading-with-numbering pdftags ---
#set heading(numbering: "1.")
= Heading <heading>

#link(<heading>)[link to heading]

--- link-tags-non-refable-location pdftags ---
A random location <somewhere>

#link(<somewhere>)[link to somewhere]

--- link-tags-contact-prefix pdftags ---
#link("mailto:hello@typst.app")

#link("tel:123")

--- link-tags-position pdftags ---
#context link(here().position())[somewhere]
