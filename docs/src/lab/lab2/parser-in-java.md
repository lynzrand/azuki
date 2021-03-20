# 写一个阳间的递归下降分析器

递归下降分析器怎么写，书上说的已经非常清楚了，这里只提几点在现代面向对象语言里写的时候需要注意的点。

## 分析器基础类型设计

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
