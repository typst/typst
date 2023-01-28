#set page(width: auto)

```typ
#set hello()
#set hello()
#set hello.world()
#set hello.my.world()

#show heading: func
#show module.func: func
#show module.func: it => {}
#foo(ident: ident)

#hello
#hello()
#hello.world
#hello.world()
#hello().world()
#hello.my.world
#hello.my.world()
#hello.my().world
#hello.my().world()

$ hello $
$ hello() $
$ hello.world $
$ hello.world() $
$ hello().world() $
$ hello.my.world $
$ hello.my.world() $
$ hello.my().world $
$ hello.my().world() $

$ emph(hello) $
$ emph(hello()) $
$ emph(hello.world) $
$ emph(hello.world()) $
$ emph(hello().world()) $
$ emph(hello.my.world) $
$ emph(hello.my.world()) $
$ emph(hello.my().world) $
$ emph(hello.my().world()) $

$ #hello $
$ #hello() $
$ #hello.world $
$ #hello.world() $
$ #hello().world() $
$ #hello.my.world $
$ #hello.my.world() $
$ #hello.my().world $
$ #hello.my().world() $

#{ hello }
#{ hello() }
#{ hello.world }
#{ hello.world() }
#{ hello().world() }
#{ hello.my.world }
#{ hello.my.world() }
#{ hello.my().world }
#{ hello.my().world() }
```
