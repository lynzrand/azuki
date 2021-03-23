(fn fib (i32) i32
    (bb0 (
        (%0 i32 param 0)
        (%1 b32 le %0 1))
        (brif %1 bb1 bb2))
    (bb1 (
        (%7 i32 1))
        (br bb3))
    (bb2 (
        (%2 i32 sub %0 1)
        (%3 i32 sub %0 2)
        (%4 i32 call fib (%2))
        (%5 i32 call fib (%3))
        (%6 i32 add %4 %5))
        (br bb3))
    (bb3 (
        (%7 i32 phi (bb1 %7) (bb2 %6)))
        (return %7)))

(fn main () i32 
    (bb0 (
        (%0 i32 call fib (15)))
        (return %0)))

(%5678 nil call @intrinsinc:putint (123))

i32
b32
(ptr <type>)
(fn (<param>*) <ret>)
()
