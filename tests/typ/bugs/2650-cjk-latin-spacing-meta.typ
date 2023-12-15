// https://github.com/typst/typst/issues/2650
#let with-locate(body) = locate(loc => body)

测a试

测#with-locate[a]试
