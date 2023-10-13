#set page(width: auto)

```typ
#set hello()
#set hello()
#set hello.world()
#set hello.my.world()
#let foo(x) = x * 2
#show heading: func
#show module.func: func
#show module.func: it => {}
#foo(ident: ident)
#hello
#hello()
#box[]
#hello.world
#hello.world()
#hello().world()
#hello.my.world
#hello.my.world()
#hello.my().world
#hello.my().world()
#{ hello }
#{ hello() }
#{ hello.world() }
$ hello $
$ hello() $
$ box[] $
$ hello.world $
$ hello.world() $
$ hello.my.world() $
$ f_zeta(x), f_zeta(x)/1 $
$ emph(hello.my.world()) $
$ emph(hello.my().world) $
$ emph(hello.my().world()) $
$ #hello $
$ #hello() $
$ #hello.world $
$ #hello.world() $
$ #box[] $
#if foo []
```
