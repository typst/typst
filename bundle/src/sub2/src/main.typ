#import "/global/settings.typ": settings
#show: settings

Sub-document 2

#{
  let path = raw(entrypoint())
  `(entrypoint() == "` + path + `")`
}
