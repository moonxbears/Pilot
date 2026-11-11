pilir 1
layout 0 "?" aggregate 0 0 0
layout 1 "i64" gpr_signed 64 3 0
layout 2 "bool" gpr_unsigned 1 0 0
int 0 6
int 1 7
int 2 40
int 3 42
int 4 1
int 5 49
int 6 36
intrinsic 0 rt_print
fn 0 "main" -> 0 {
  b0():
    jump b1() :0 !0
  b1():
    v6 = const_bits #6 :1 !0
    v7 = call_intrinsic v6 :0 !0
    return :0 !0
  b2():
    unreachable :0 !0
}
