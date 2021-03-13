# LAB2 实现指导

## 实现递归下降分析 (Recursive Descent Parsing)

> 其实书上写的已经够清楚了，这里就讲一下具体实现时的一些要点吧

### 分析器类的设计

一个分析器类大概长这样：

```java
public final class Parser {
    // 在这里手动实现一个 peekable
    private Lexer lexer;
    private Token peeked = null;

    /** 下一个单词 */
    private Token next() {
        if (peeked != null) {
            Token token = peeked;
            peeked = null
            return token;
        } else {
            return lexer.next();
        }
    }

    /** 预读下一个单词 */
    private Token peek() {
        if (peeked == null) {
            peeked = lexer.next();
        }
        return peeked;
    }
}
```

另外，加一点辅助函数会更舒适一点：

```java
/** 如果类型相符，则吃掉下一个单词 */
private Token eatIf(TokenKind kind) {
    if (peek().kind == kind) {
        return next();
    } else {
        return null;
    }
}

/** 如果类型相符，则吃掉下一个单词，否则抛出异常 */
private Token eatOrThrow(TokenKind kind) {
    if (peek().kind == kind) {
        return next();
    } else {
        return new RuntimeException("Wrong token kind");
    }
}
```

然后我们就可以开始写递归下降了——

> 此处请参考书上内容

### 表达式分析与优先级爬升分析法

你还记得自顶向下分析不能处理什么情况吗？——对，左递归。

但是！在分析表达式的时候，绝大多数运算符都是左结合的——比如，`a+b+c` 会被分析成 `(a+b)+c` 而不是 `a+(b+c)`——这样分析的时候就需要处理左递归了。此外，运算符之间有优先级关系层级很多，也不方便递归下降分析，怎么办？

你可能想到了算符优先分析法 (OPG)。但是回忆一下，它是一个自底向上的算法，而且需要一个栈来存放中间结果，跟递归下降分析合不来。幸运的是，在 1979 年，Martin Richards 和 Colin Whitby-Strevens 提出了一种适合嵌入递归下降中的算符优先分析法，称为 “优先级爬升法” (Precedence Climbing)。

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

这个算法的前提条件是你要确定分析的所有带优先级的运算符都是二元中缀运算符（绝大部分情况都是这样的），同时你有一个可以分析运算项（比如带括号的表达式或者变量）的函数。

之后的思路很自然。比如说，我们有一个表达式，`a+b*c-d`，然后我们知道优先级上 `+` = `-`，`*` > `+`。

首先我们可以分析表达式最开始的一项 `a` 作为表达式的左手边。然后，我们就会读到加号 `+` 和另一项 `b` 我们保存这三个值。

在这个时候，我们就会遇到一个问题：`b` 到底是 `a` `+` 的右手边，还是另一个表达式的左手边？为了解决这个问题，我们需要读取再下一个符号，`*`。由于 `*` > `+`，`b` 就成为了新表达式的左手边——然后我们就回到了一开始。
