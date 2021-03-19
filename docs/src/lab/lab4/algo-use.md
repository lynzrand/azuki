# 如何使用 SSA 生成算法

> 本文中的算法和内容主要来自 Matthias Braun, Sebastian Buchwald, Sebastian Hack 等人在 2013 年发表的论文 [_Simple and Efficient Construction of Static Single Assignment Form_][braun13]。如果你对算法的原理或者内核感兴趣，可以阅读这篇论文。SSA 代码的生成算法不止一种，你可以在文章末尾找到更多信息。

---

生成 SSA 的算法不止一种。有的算法需要你先生成整个函数的中间表示才能工作，而有的需要在分析函数的过程中工作。[本文使用的算法][braun13]属于后者，需要你在 **顺序遍历语法树** 的同时调用相应的方法进行生成。下面我们来介绍一下这个算法如何使用。

我们提供的中间代码库中有一个名叫 `FunctionBuilder` 的类，这个类封装了 SSA 中间代码的生成算法，也是我们讲解的主体。
<!-- 
它提供了这些对外接口：

- `declareVariable`
- `readVariable`
- `writeVariable`
- `markFilled`
- `markSealed`
- `addBranch` -->

## 基本块的分类

在中间代码的生成过程中，我们需要给基本块添加两个属性：`filled` 和 `sealed`。这两个属性可以指导中间代码生成器输出合适的代码。

**`filled`（已填满）表示我们已经生成了这个基本块中的所有指令。** 换句话说，一个被标记为 `filled` 的基本块中只能再填入对实际运算没有影响的 `Phi` 指令了。

**`sealed`（已密封）表示这个我们已经遍历过了这个基本块的所有直接前驱。** 换句话说，之后添加的基本块都不会直接跳转到已经被标记为 `sealed` 的基本块。通过 `sealed` 属性，我们可以保证 Phi 指令的来源不会发生变化。

在算法的对外接口中，我们有 `markFilled(bb)` 和 `markSealed(bb)` 两个方法用来给基本块添加相应的标记。我们之后还会谈到它们。

## 变量的声明与使用

要生成 SSA 代码，我们需要给在不同位置读取和写入的变量标记合适的编号，以及在合适的地方插入 `Phi` 指令。不过在此之前，我们需要先给每个原始的变量标记唯一的编号，方便算法使用。一般来说，用一个计数器给所有出现的变量顺序编号就足够了。[^var_counting]

在算法的接口中，我们提供了 `declareVar`、`readVar`、`writeVar` 三个方法来管理变量的声明和使用。

`declareVar(var, type)` 告诉算法，编号为 `var` 的变量具有 `type` 的类型。这个方法主要是为了之后生成指令的时候可以确定变量的类型。

`writeVar(var, inst, bb)` 告诉算法，在基本块 `bb` 中，指令 `inst` 的运算结果[^inst]是给变量 `var` 的一次写入。

`readVar(var, bb)` 表示我们在基本块 `bb` 内读取了变量 `var`。这个方法会根据当前已知的变量定义来确定我们读取的变量对应的是哪一个指令的。如果变量有多个来源，我们就会插入一条 Phi 指令来解决这个冲突。这个方法返回对应的指令编号。

[^var_counting]: 我们强烈不建议在这里使用变量名作为编号，因为变量会经常重名。

[^inst]: 当然了，因为在 SSA 代码中每一个 “变量” 都是唯一一条指令的运算结果，我们的中间代码库在实现的时候就没有给 “变量” 设计独立的标号，而是直接使用指令的编号作为对应 SSA “变量” 的编号。或者可以这么说——**在 SSA 中，我们根本不区分指令和变量**。

## 控制流的翻译

我们还有最后一个接口方法要介绍。`addBranch(from, to)` 告诉算法，存在一条从基本块 `from` 指向基本块 `to` 的跳转。

控制流的翻译方式实际上非常直接，我们只需要在合适的时间调用相应的方法就可以了。下面是一些例子：

### If 控制流

对于一个如下的 if 控制流，

```c
// bb_start
if /* cond */ {
    // bb_if
    /* if_body */
    // bb_if_end
} else {
    // bb_else
    /* else_body */
    // bb_else_end
}
// bb_next
```

我们调用相应方法的顺序大概是这样的（省去了生成指令和创建基本块的过程）：

```c
/* 返回值 */ visitIfCondition(/* 参数啥的 */) {
    // 生成 cond 的指令
    markFilled(bb_start);
    markSealed(bb_start);

    // 生成 bb_start 跳转到 bb_if 的指令
    addBranch(bb_start, bb_if);
    // 生成 if_body，以 bb_if_end 结尾
    // 生成 bb_if_end 跳转到 bb_next 的指令
    markFilled(bb_if_end);
    markSealed(bb_if_end);
    addBranch(bb_if_end, bb_next);

    // 生成 bb_start 跳转到 bb_else 的指令
    addBranch(bb_start, bb_else);
    // 生成 else_body，以 bb_else_end 结尾
    // 生成 bb_else_end 跳转到 bb_next 的指令
    markFilled(bb_else_end);
    markSealed(bb_else_end);
    addBranch(bb_else_end, bb_next);
}
```

### While 控制流

对于一个如下的 While 控制流，

```c
// bb_start
while /* cond, bb_cond */ {
    // bb_while
    /* while_body */
    // bb_while_end
}
// bb_next
```

我们调用相应方法的顺序大概是这样的（同上）：

```c
/* 返回值 */ visitWhileLoop(/* 参数啥的 */) {
    markFilled(bb_start);
    markSealed(bb_start);

    // 生成 cond 所在的基本块和指令
    markFilled(cond);
    // 生成 bb_cond 跳转到 bb_while 的指令
    addBranch(bb_cond, bb_while);
    // 生成 bb_cond 跳转到 bb_next 的指令
    addBranch(bb_cond, bb_next);

    // 生成 while_body，以 bb_while_end 结尾
    // 生成 bb_while_end 跳转到 bb_next 的指令
    markFilled(bb_while_end);
    markSealed(bb_while_end);
    addBranch(bb_while_end, bb_cond);

    // 注意：直到现在，bb_cond 的所有前驱才都遍历完毕
    markSealed(bb_cond);
}
```


## 扩展阅读

- Ron Cytron 等人在 1989 年发布的 SSA 生成算法：[cytron89][]
- 本文中使用的算法: [braun13][]
- GCC 和 LLVM 使用的算法: [MemorySSA][mem-ssa]


[braun13]: https://pp.ipd.kit.edu/uploads/publikationen/braun13cc.pdf
[cytron89]: https://dl.acm.org/doi/pdf/10.1145/75277.75280?casa_token=jGJzayvh9fwAAAAA:-Y04UXeF00riSGWoHYViXiJ4FUAlpIYtRl9x67b0SC0qIQv7TTwxLfF4_dH0_lTdEQuGe96KtC9h
[mem-ssa]: https://www.airs.com/dnovillo/Papers/mem-ssa.pdf
