#import "/global/settings.typ": settings
#show: settings

Sub-document 1

#{
  let path = raw(entrypoint())
  `(entrypoint() == "` + path + `")`
}
