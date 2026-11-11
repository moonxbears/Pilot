pilir 1
layout 0 "?" aggregate 0 0 0
layout 1 "i64" gpr_signed 64 3 0
layout 2 "i8" gpr_signed 8 0 0
int 0 100
int 1 0
int 2 7
int 3 64
intrinsic 0 rt_print
fn 0 "main" -> 0 {
  b0():
    v0 = const_bits #0 :2 !0
    v1 = add v0, v0 :2 !0
    v2 = const_bits #1 :1 !0
    v3 = const_bits #2 :1 !0
    v4 = div v3, v2 :1 !0
    v5 = const_bits #3 :1 !0
    v6 = shl v3, v5 :1 !0
    v7 = call_intrinsic v1 :0 !0
    v8 = call_intrinsic v6 :0 !0
    return :0 !0
}
