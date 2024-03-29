# Lab5: 代码优化

难度：较难

## 实验要求

**你的程序在读入一个 c0 源文件之后，应能针对代码执行一定的优化，减少执行的指令数量。**

你有两周时间完成这个实验。

## 评分标准

- 你的代码将在 OJ 上运行，并记录执行过的指令的加权总数量。加权的方式见下方说明。
- 你每一个测试点的最终得分是 **参考实现的指令数量 / 你的指令数量 * 测试点基础分**。

### 加权方式

运算指令：

| 指令          | 权重 |
| ------------- | ---: |
| 加法          |    1 |
| 减法          |    1 |
| 移位          |    1 |
| 乘法 [^arith] |    3 |
| 除法 [^arith] |    7 |

跳转指令：

| 指令                                  |       权重 |
| ------------------------------------- | ---------: |
| 条件跳转                              |          5 |
| 非条件跳转                            |          3 |
| (跳转目标是当前的下一个基本块)[^jump] |          1 |
| (距离上次跳转 n 条指令, n < 8)[^jump] | 额外加 8-n |

其他指令：

| 指令               |    权重 |
| ------------------ | ------: |
| Phi [^phi]         |       0 |
| 赋值               |       0 |
| 函数调用, n 个参数 | n*2 + 7 |
| 其他               |       1 |

[^arith]: 这是为了模仿现代处理器的特点。现代处理器中，乘除法运算需要消耗的时钟周期长于加减法。例如，ARMv7 架构的 Cortex-A73 处理器中加减法指令消耗 1 个时钟周期，乘法约 3 -- 5 个时钟周期，除法约 10 -- 15 个。

[^jump]: 这是为了模仿现代处理器的特点。现代处理器通常具有很长的流水线（快回去看 CSAPP），连续跳转时可能需要重新填满整个流水线，最多可能要消耗十几个时钟周期。当然，作为一个简化的判断标准，这里没有模拟分支预测的成功与失败的情况，而是用一个固定的值提示你少跳转。

[^phi]: Phi 指令本质上只是一个赋值，在真实的编译器中通过寄存器分配等操作可以基本消除掉，因此不计入指令总数。
