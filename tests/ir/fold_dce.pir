pilir 1
layout 0 "?" aggregate 0 0 0
layout 1 "i64" gpr_signed 64 3 0
layout 2 "bool" gpr_unsigned 1 0 0
int 0 6
int 1 7
int 2 40
intrinsic 0 rt_print
fn 0 "main" -> 0 {
  b0():
    v0 = const_bits #0 :1 !0
    v1 = const_bits #1 :1 !0
    v2 = mul v0, v1 :1 !0
    v3 = const_bits #2 :1 !0
    v4 = cmp.4 v2, v3 :2 !0
    v5 = add v2, v1 :1 !0
    cond_br v4, b1(), b2() :0 !0
  b1():
    v6 = sub v2, v0 :1 !0
    v7 = call_intrinsic v6 :0 !0
    return :0 !0
  b2():
    v8 = call_intrinsic v3 :0 !0
    return :0 !0
}
