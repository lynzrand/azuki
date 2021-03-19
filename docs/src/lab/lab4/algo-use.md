# 如何使用 SSA 生成算法

> 本文中的算法和内容主要来自 Matthias Braun, Sebastian Buchwald, Sebastian Hack 等人在 2013 年发表的论文 [_Simple and Efficient Construction of Static Single Assignment Form_][braun13]。如果你对算法的原理或者内核感兴趣，可以阅读这篇论文。当然了，SSA 代码的生成算法不止这一种。在本文结尾的扩展阅读中，你可以找到其他常用的生成算法。

---

生成 SSA 的算法有很多种。有的算法需要你先生成整个函数的中间表示才能工作，而有的需要在分析函数的过程中工作。[本文使用的算法][braun13]属于后者，需要你在 **顺序遍历语法树** 的同时调用相应的方法进行生成。下面我们来介绍一下这个算法如何使用。

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

为了让我们的生成算法能够正常工作，我们需要给基本块添加两个属性：`filled` 和 `sealed`。

**`filled`（已填满）表示我们已经生成了这个基本块中的所有指令。** 换句话说，一个被标记为 `filled` 的基本块中只能再填入对实际运算没有影响的 `Phi` 指令了。

**`sealed`（已密封）表示我们已经遍历过了这个基本块的所有直接前驱。** 换句话说，之后添加的基本块都不会直接跳转到已经被标记为 `sealed` 的基本块。通过 `sealed` 属性，我们可以保证 Phi 指令的来源不会发生变化。由于我们是顺序遍历的语法树，只有在遍历完一个基本块之后我们才会开始访问它的后继，所以所有标记为的 `sealed` 的基本块一定都已经被标记成了 `filled`[^filled]。

在算法的对外接口中，我们有 `markFilled(bb)` 和 `markSealed(bb)` 两个方法用来给基本块添加相应的标记。我们之后还会谈到它们。

[^filled]: 在某些情况下，除了当前正在使用的基本块。不同人对 `sealed` 标记的用法可能不一样。如果你选择只要能够确定所有前驱就加 `sealed` 的话，这句话有可能不适用于当前基本块；如果你选择在标记为 `filled` 之后再标记为 `sealed` 的话，这句话就适用于当前基本块。策略选择的不同不会影响生成的代码。

## 变量的声明与使用

要生成 SSA 代码，我们需要给在不同位置读取和写入的变量标记合适的编号，以及在合适的地方插入 `Phi` 指令。不过在此之前，我们需要先给每个原始的变量标记唯一的编号，方便算法使用。一般来说，用一个计数器给所有出现的变量顺序编号就足够了。[^var_counting]

在算法的接口中，我们提供了 `declareVar`、`readVar`、`writeVar` 三个方法来管理变量的声明和使用。

`declareVar(var, type)` 告诉算法，编号为 `var` 的变量具有 `type` 的类型。这个方法主要是为了之后生成指令的时候可以确定变量的类型。

`writeVar(var, inst, bb)` 告诉算法，在基本块 `bb` 中，指令 `inst` 的运算结果[^inst]是给变量 `var` 的一次写入。

`readVar(var, bb)` 表示我们在基本块 `bb` 内读取了变量 `var`。这个方法会根据当前已知的变量定义来确定我们读取的变量对应的是哪一个指令的。如果变量有多个来源，我们就会插入一条 Phi 指令来解决这个冲突。这个方法返回对应的指令编号。

[^var_counting]: 我们强烈不建议在这里使用变量名作为编号，因为变量会经常重名。

[^inst]: 当然了，因为在 SSA 代码中每一个 “变量” 都是唯一一条指令的运算结果，我们的中间代码库在实现的时候就没有给 “变量” 设计独立的标号，而是直接使用指令的编号作为对应 SSA “变量” 的编号。或者可以这么说——**在 SSA 中，我们根本不区分指令和变量**。

### 例子

对于如下的 C 代码：

```c
int a = 0;
int b = a + 1;
a = b + 1;
```

我们调用变量读写方法的顺序大致如下（省去了基本块参数和函数调用层级，只表示各个指令调用的时机和顺序）：

```c
{
    // int a = 0;
    declareVar(a, int); // 声明变量 a
    a_1 = /* 生成常数 0 的指令 */;
    writeVar(a, a_1);   // 我们写入了 a
}
{
    // int b = a + 1;
    declareVar(b, int); // 声明变量 b
    a_1 = readVar(a);   // 我们读取了 a
    b_1 = /* 生成相加的指令 */;
    writeVar(b, b_1);   // 我们写入了 b
}
{
    // a = b + 1;
    b_1 = readVar(b);   // 我们读取了 b
    a_2 = /* 生成相加的指令 */;
    writeVar(a, a_2);   // 我们写入了 a
}

```

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
visitIfCondition(/* 参数啥的 */) {
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
visitWhileLoop(/* 参数啥的 */) {
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

## 一点内部原理

> 这里讲的只是算法的大概思路，具体细节请看[论文][braun13]。

思考一下我们获取一个变量当前的值来源对应的指令编号时需要知道什么信息。

如果在同一个基本块之内的话，我们需要块内上一次赋值指令的编号；如果不在同一个基本块之内的话，我们需要知道这个基本块所有直接前驱里这个变量的定义。前者只要顺序遍历、跟踪写入指令的话很简单，后者在已知基本块的所有前驱的情况下也不难（挨个前驱调用这个算法，然后用一条 Phi 指令拼起来就好）。那么在前驱还不明确的情况下呢？——只要等到前驱都确定完了之后再来计算就可以了。

对，核心思想就是上面这么一点。我们可以把它跟算法提供的接口来进行一下对比：

- 跟踪每次赋值指令的编号由 `writeVar` 负责
- 确定基本块的前驱由 `addBranch` 负责
- 获取指令编号由 `readVar` 负责
- 在前驱都确定完成之后调用算法由 `markSealed` 负责

再加上起辅助作用的 `declareVar` 和 `markFilled`，嗯，这个算法就是这么简单。

> \- 既然算法这么简单，那为什么要封装好而不是让我们自己写呢？
> 
> \- 那是因为这个算法内部还有一些优化部分要处理，自己写还是麻烦了点。（真那么想自己写你去写 [Lab4EX][] 啊！）

## 扩展阅读

- Ron Cytron 等人在 1989 年发布的 SSA 生成算法（书上一般会讲这个）：[cytron89][]
- 本文中使用的算法: [braun13][]
- GCC 和 LLVM 使用的算法: [MemorySSA][mem-ssa]


[braun13]: https://pp.ipd.kit.edu/uploads/publikationen/braun13cc.pdf
[cytron89]: https://dl.acm.org/doi/pdf/10.1145/75277.75280?casa_token=jGJzayvh9fwAAAAA:-Y04UXeF00riSGWoHYViXiJ4FUAlpIYtRl9x67b0SC0qIQv7TTwxLfF4_dH0_lTdEQuGe96KtC9h
[mem-ssa]: https://www.airs.com/dnovillo/Papers/mem-ssa.pdf

[lab4ex]: ../lab4-ex.md
