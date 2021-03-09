fn @fib(i32) -> i32 {
bb0:
    %0 = i32 param 0
    %1 = i32 le %0 #1
    br bb1 if %1
    br bb2
bb1:
    return #1
bb2:
    %2 = i32 sub %0 #1
    %3 = i32 sub %0 #2
    %4 = i32 call @fib(%2)
    %5 = i32 call @fib(%3)
    %6 = i32 add %4 %5
    return %6
}
