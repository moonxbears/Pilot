pilir 1
layout 0 "?" aggregate 0 0 0
layout 1 "i64" gpr_signed 64 3 0
layout 2 "bool" gpr_unsigned 1 0 0
int 0 6
int 1 7
int 2 0
int 3 1
intrinsic 0 rt_print
fn 0 "main" -> 0 {
  b0():
    v0 = const_bits #0 :1 !0
    v1 = const_bits #1 :1 !0
    v2 = mul v0, v12 :1 !0
    v3 = call_intrinsic v2 :0 !0
    v4 = const_bits #2 :1 !0
    v12 = const_bits #3 :1 !0
    jump b1(v4, v4) :0 !0
  b1(v5:1, v6:1):
    v7 = cmp.2 v5, v0 :2 !0
    cond_br v7, b2(), b3() :0 !0
  b2():
    v8 = add v6, v5 :1 !0
    v9 = const_bits #3 :1 !0
    v10 = add v5, v9 :1 !0
    jump b1(v10, v8) :0 !0
  b3():
    v11 = call_intrinsic v6 :0 !0
    return :0 !0
}
