 
// bindgen bindings.h --no-prepend-enum-name --allowlist-type="(avif|AVIF).*" --allowlist-function="(avif|AVIF).*" --allowlist-var="(avif|AVIF).*" --no-layout-tests --with-derive-default  -- -Ilibavif/include | sed -E 's/ ?\\\\brief ?// ' | sed -E 's/doc = " ?< /doc = "/' > src/ffi.rs
//
// Requires bindgen >= 0.70. Do NOT regenerate with 0.69.4 (what the 1.0.4-era
// bindings were built with): from libavif 1.2.0 onwards avif.h forward-declares
// `struct avifImage;` ahead of avifGainMap, which holds an avifImage*, and 0.69.4
// emits the *forward declaration* as an opaque `avifImage { _address: u8 }` and
// drops the real definition entirely. It exits 0 and prints nothing, so the only
// symptom is every field access (`(*img).width`) failing to compile downstream --
// or, worse, compiling against the wrong layout if the opaque type is ever cast.
// Verified with a three-line repro: a forward-declared struct that is later
// defined comes out opaque on 0.69.4 and complete on 0.72.1.

#include "avif/avif.h"
