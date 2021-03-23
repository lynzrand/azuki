(fn @fib (i32) i32
    (bb0 (
        (%0 i32 param 0)
        (%1 i32 le %0 1))
        (br (if %1 bb1) bb2))
    (bb1 ()
        (return 1))
    (bb2 (
        (%2 i32 sub %0 1)
        (%3 i32 sub %0 2)
        (%4 i32 call @fib (%2))
        (%5 i32 call @fib (%3))
        (%6 i32 add %4 %5))
        (return %6)))

(fn @main () i32 
    (bb0 (
        (%0 i32 call @fib (15)))
        (return %0)))

(%5678 nil call @intrinsinc:putint (123))

i32
b32
(ptr <type>)
(fn (<param>*) <ret>)
()
