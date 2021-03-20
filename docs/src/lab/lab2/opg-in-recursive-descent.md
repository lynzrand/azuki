# 在递归下降里实现算符优先分析法是不是搞错了什么

> 不，你没有——因为它太常用了所以一群人研究了一大堆方法出来

## 基础但是够用——优先级爬升法

你还记得自顶向下分析不能处理什么情况吗？——对，左递归。

但是！在分析表达式的时候，绝大多数运算符都是左结合的——比如，`a+b+c` 会被分析成 `(a+b)+c` 而不是 `a+(b+c)`——这样分析的时候就需要处理左递归了。此外，运算符之间有优先级关系层级很多，也不方便递归下降分析，怎么办？

你可能想到了算符优先分析法 (OPG)。但是回忆一下，它是一个自底向上的算法，而且需要一个栈来存放中间结果，跟递归下降分析合不来。幸运的是，在 1979 年，Martin Richards 和 Colin Whitby-Strevens 提出了一种适合嵌入递归下降中的算符优先分析法，称为 **“优先级爬升法”** (Precedence Climbing)。

先放一下伪代码吧：

```py
# 解析一个表达式
def parse_expression():
    # 分析一个项
    term = parse_term()
    # 传入这一项和最低的优先级
    return climb(term, 0)

# 进行一次优先级爬升
# lhs 是当前算式的左手边
# pred 是当前可以解析的最小优先级
def climb(lhs, min_pred):
    # 查看下一个单词
    lookahead = peek_token()
    # 当下一个单词是二元运算符，且优先级不小于当前优先级时
    while is_binary_op(lookahead) && pred(lookahead) >= min_pred:
        # 记录这个运算符并前进一个单词
        op = lookahead
        next()
        # rhs 是当前算式的右手边。分析一个项
        rhs = parse_term()
        # 向前看一个单词
        lookahead = peek_token()

        # 当下一个单词是左结合的二元运算符，且优先级大于当前优先级
        # 或者是右结合的二元运算符，且优先级大于等于当前优先级时
        while is_binary_op(lookahead) && (
            (is_left_assoc(lookahead) && pred(lookahead) > min_pred) ||
            (is_right_assoc(lookahead) && pred(lookahead) >= min_pred)):
            # 解析一个更优先的表达式
            rhs = climb(rhs, pred(lookahead))
            # 向前看一个单词
            lookahead = peek_token()
        
        # 组合当前的表达式
        lhs = combine(lhs, op, rhs)

    # 最后 lhs 就是我们需要的表达式
    return lhs    
```

这个算法的前提条件是 **你要确定分析的所有带优先级的运算符都是二元中缀运算符**（绝大部分情况都是这样的），同时你有一个可以分析运算项（比如带括号的表达式或者变量）的函数。

直接把伪代码翻译到你编写的语言里就好了。

如果你对原理感兴趣的话，可以思考一下把左递归文法 `E -> E '+' T | T; T -> T '*' F | F` 转换成 EBNF `E -> T ('+' T)*; T -> F ('*' F)*` 之后，分析 `F * F` 怎么省掉 `E -> T` 那一步。

<!-- 
之后的思路很自然。比如说，我们有一个表达式，`a+b*c-d`，然后我们知道优先级上 `+` = `-`，`*` > `+`。

首先我们可以分析表达式最开始的一项 `a` 作为表达式的左手边。然后，我们就会读到加号 `+` 和另一项 `b` 我们保存这三个值。

在这个时候，我们就会遇到一个问题：`b` 到底是 `a` `+` 的右手边，还是另一个表达式的左手边？为了解决这个问题，我们需要读取再下一个符号，`*`。由于 `*` > `+`，`b` 就成为了新表达式的左手边——然后我们就回到了一开始。 -->

## 还能更给力一点吗？——Pratt 分析法

> 在这个实验里你们应该用不到 Pratt 分析法。
> 
> 推荐阅读: 
>  
> - [关于 Pratt 分析法的文章索引](https://www.oilshell.org/blog/2017/03/31.html)
> - [Pratt parsers: Expression parsing made easy](http://journal.stuffwithstuff.com/2011/03/19/pratt-parsers-expression-parsing-made-easy/)
> - [Simple but Powerful Pratt Parsing](https://matklad.github.io/2020/04/13/simple-but-powerful-pratt-parsing.html)

优先级爬升分析法已经够用了。大概。除非……你想让前置负号的优先级比加法还低，或者你想解析一点三元条件表达式，这些事情就在优先级爬升法的能力范围之外了。这个时候，我们就要请出 **Pratt 分析法**——优先级爬升分析法的一般情况。

TODO: 待补完
