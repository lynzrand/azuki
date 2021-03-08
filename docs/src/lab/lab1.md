# Lab1: 词法分析

难度：简单

嗨！

第一个实验终于要开始了。这次实验的内容——鉴于你们也没学别的——是词法分析。词法分析本身不难，基本就是匹配正则表达式，用自动机也好用手写也好都不麻烦。预祝实验顺利！

[实验指导](lab1-guide.md)

## 实验要求

**你的程序在读入一个 c0 源文件之后，应能输出词法分析之后得到的单词 (Token) 序列。** 你需要丢弃所有表示空白符（如空格、回车）的单词，并将其他单词使用默认格式化每行一个按顺序输出。

你有两周时间完成这个实验。

## 评分标准

- **100% 正确性**：你的代码应能通过 OJ 的检验

## 样例

输入：

```rust,noplayground
fn foo(x: int) -> int {
    return 42 + x * 21;
}
```

输出：

```
Fn
Ident "foo"
LParen
Ident "x"
Colon
Ident "int"
RParen
Arrow
Ident "int"
LBrace
Return
IntLiteral "42"
Plus
Ident "x"
IntLiteral "8"
Semicolon
RBrace
```
