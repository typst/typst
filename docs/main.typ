#import "components/index.typ": styling
#show: styling

#context if target() == "bundle" {
  include "assets/index.typ"
}

#include "content/index.typ"
