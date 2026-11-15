pilir 1
layout 0 "?" aggregate 0 0 0
layout 1 "i64" gpr_signed 64 3 0
int 0 5
int 1 2
intrinsic 0 rt_print
global 0 "count" 1
fn 0 "main" -> 0 {
  b0():
    v0 = global_addr :1 !0
    v1 = const_bits #0 :1 !0
    store v0, v1 :0 !0
    v2 = call_direct.1 :0 !0
    v3 = call_direct.1 :0 !0
    v4 = load v0 :1 !0
    v5 = call_intrinsic v4 :0 !0
    return :0 !0
}
fn 1 "bump" -> 0 {
  b0():
    v0 = global_addr :1 !0
    v1 = load v0 :1 !0
    v2 = const_bits #1 :1 !0
    v3 = add v1, v2 :1 !0
    store v0, v3 :0 !0
    return :0 !0
}
