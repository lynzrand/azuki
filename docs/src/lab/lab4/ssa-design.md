
# 关于为什么要用 SSA 格式

> 简而言之，方便优化。

---

你可能发现了，我们介绍这个项目的中间代码时提到，中间代码是 SSA（静态单赋值）格式，而不是一般的四元式。SSA 是什么呢？我们先粘一段维基百科：

> **在编译器的设计中，静态单赋值形式（static single assignment form，通常简写为 SSA form 或是 SSA）是中间表示（IR，intermediate representation) 的特性，每个变量仅被赋值一次。** 在原始的IR中，已存在的变量可被分割成许多不同的版本，在许多教科书当中通常会将旧的变量名称加上一个下标而成为新的变量名称，以此标明每个变量及其不同版本。

我们为什么要用这种奇怪的表示方法呢？

我们举个例子啊——比如有这么一段普通四元式中间代码：

```c
c = 1 + 2   // (1)
c = a + b   // (2)
d = c       // <-- 这里
```

那么你的编译器分析到标注的地方时，它用的是 `(1)` 还是 `(2)` 处的 `c` 呢？

你可能会说，很明显 `(2)` 处 `c` 被重新赋值了，所以答案是 `(2)` 处。对，那么很自然地，我们可以更进一步，把重新赋值的变量给改个名字，比如把 `(1)` 处的变量叫 `c_1`、`(2)` 处的叫 `c_2`。于是，我们的代码就变成了这样：

```c
c_1 = 1 + 2
c_2 = a + b
d   = c_2    // <-- 很明确这里用的是 c_2
```

这个时候我们得到的基本就是 SSA 格式的代码了。

从定义就可以看出来，SSA 格式的代码里，每个变量被且仅被赋值过了一次。通过这个操作，我们避免了在分析中间代码的时候寻找变量最后一次赋值的位置，从而也降低了代码优化的难度和运算量（在做 lab5 的时候应该可以清楚地体会到）。只不过目前的格式还有一点问题，他不能表示下面这样的代码：

```c
int r;
if (a > 10) {
    r = a;
} else {
    r = b;
}
// 这里的 r 怎么表示？
```

为了解决这个问题，我们引入一个新的四元式指令—— Phi (Φ)。使用 Phi 指令，我们可以根据控制流的不同来给变量分配不同的值，就像下面这样（`bb___` 表示对应编号的基本块）：

```c
bb0:
    if a > 10 goto bb1
    else goto bb2 
bb1:
    r1 = a
    goto bb3
bb2:
    r2 = b
    goto bb3
bb3:
    // 如果控制流来自 bb1 则使用 r1 的值
    // 如果控制流来自 bb2 则使用 r2 的值
    r = Phi[(bb1, r1), (bb2, r2)]
```

有了 Phi 指令，我们就可以用 SSA 格式的中间代码表示任意的控制流了。

## 一些例子

下面举一些 C 式的控制流以及它的其中一种含义相同的 SSA 中间代码：

### 单侧 if 控制流

```c
int i = 3;
if (a > 0) {
    i = 6;
}
// i
```

```c
bb0:
    i_1 = 3
    if a > 0 goto bb1
    else goto bb2
bb1:
    i_2 = 6;
    goto bb2
bb2:
    i = Phi[(bb0, i_1), (bb1, i_2)]
```

### while 循环

```c
int i = 0;
while (i < 10) {
    i = i + 1;
}
// i
```

```c
bb0:
    i_0 = 0
    goto bb1
bb1:
    if i < 10 goto bb2
    else goto bb3
bb2:
    i_1 = Phi[(bb0, i_0), (bb1, i_2)]
    i_2 = i_1 + 1
    goto bb1
bb3:
    i = Phi[(bb0, i_0), (bb1, i_2)]
```

> 关于这个例子里 `i_1` 为什么能得到还没声明过的 `i_2` 的值，在 [如何使用 SSA 生成算法][algo-use] 里面我们会简要讲解一下。

[algo-use]: (algo-use.md)