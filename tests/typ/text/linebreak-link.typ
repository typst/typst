// Test linebreaking of links.

---
#link("https://example.com/(ab") \
#link("https://example.com/(ab)") \
#link("https://example.com/(paren)") \
#link("https://example.com/paren)") \
#link("https://hi.com/%%%%%%%%abcdef") \

---
#set page(width: 240pt)
#set par(justify: true)

Here's a link https://url.com/data/extern12840%data_urlenc and then there are more
links #link("www.url.com/data/extern12840%data_urlenc") in my text of links
http://mydataurl/hash/12098541029831025981024980124124214/incremental/progress%linkdata_information_setup_my_link_just_never_stops_going/on?query=false

---
// Ensure that there's no unconditional break at the end of a link.
#set page(width: 180pt, height: auto, margin: auto)
#set text(11pt)

For info see #link("https://myhost.tld").
