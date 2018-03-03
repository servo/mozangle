#!/bin/sh
cd $(dirname $0)
bindgen \
    --opaque-type "std.*" \
    --whitelist-type "Sh.*" \
    --whitelist-type "SH.*" \
    -o bindings.rs \
    bindings.hpp \
    -- I../gfx/angle/checkout/include \
    -- -std=c++11
