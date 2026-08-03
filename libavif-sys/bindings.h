 
// bindgen bindings.h --no-prepend-enum-name --allowlist-type="(avif|AVIF).*" --allowlist-function="(avif|AVIF).*" --allowlist-var="(avif|AVIF).*" --no-layout-tests --with-derive-default  -- -Ilibavif/include | sed -E 's/ ?\\\\brief ?// ' | sed -E 's/doc = " ?< /doc = "/' > src/ffi.rs

// Requires bindgen 0.70 or newer. Since libavif 1.2.0, avif.h forward-declares
// `struct avifImage` ahead of avifGainMap, which holds a pointer to it. Earlier
// bindgen emits that forward declaration as an opaque type and omits the real
// definition, so avifImage loses every field. bindgen exits successfully and
// prints no diagnostic, and the omission surfaces only as unresolved field
// accesses in dependent code.

#include "avif/avif.h"
