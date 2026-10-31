| `Ast` | `-Z unpretty=ast-tree` | 词法/语法解析后的**带 span 的 AST 调试树**（`Expr`/`Item`/`Block` 等带 node id） | stdout |
| `Hir` | `-Z unpretty=hir` | 脱糖 + 名称解析 + **类型信息**的高层 IR | stdout |
| `Mir` | `-Z unpretty=mir` | **MIR 文本**：`fn foo(_1: i32) -> i32 { let mut _0: i32; bb0: ... }` | stdout |
| `MirCfg` | `-Z unpretty=mir-cfg` | MIR 的 **DOT/graphviz 风格控制流图** | stdout |
| `Expanded` | `-Z unpretty=expanded` | 宏展开后的源码（`println!` 变成 `$crate::io::_print(...)`） | stdout |
| `Llvm` | `--emit=llvm-ir` | `.ll` 文本（LLVM IR，SSA 形式） | **文件**（需 `-o`） |
| `Asm` | `--emit=asm` + `-C llvm-args=-x86-asm-syntax=intel` | `.s` 汇编 | **文件**（需 `-o`） |

**三个必须主动讲的技术点**：
1. **`unpretty` 不是独立参数，而是 `-Z` 的值**：命令行必须写成 `-Z unpretty=mir`（两个 arg：`"-Z"`, `"unpretty=mir"`），写成 `-Zunpretty=mir`（一个 arg）也能被 rustc 接受，但 `Command` 里分开写更清晰、可读。代码里 `vec!["-Z", "unpretty=ast-tree"]` 就是分开的。
2. **`--emit=mir` 不存在**：rustc 的 `--emit` 合法值里没有 `mir`（有 `metadata`/`link`/`dep-info`/`mir`… 实际上 `--emit=mir` 在很老的版本有过、现在不可用；文本 MIR 只能通过 `-Zunpretty=mir`）。所以**MIR 只能走 stdout 通道** —— 这直接决定了 L117/L162 两个分支必须存在。
3. **`-C llvm-args=-x86-asm-syntax=intel`**：LLVM 的 x86 后端默认输出 AT&T 语法（`movl $1, %eax`），这个参数切成 Intel 语法（`mov eax, 1`），对教学更友好。注意这个参数**只对 Asm 阶段加**，因为 LLVM IR 阶段不需要。

**追问预案**：
- 「返回 `Vec<&'static str>` 而不是 `Vec<String>`？」→ 因为这些参数全是编译期字面量，`&'static str` 零分配，只在 L123 `cmd.arg(arg)` 时借用。**代价**：如果将来某个参数需要拼运行时值（比如带用户选项的 `-C opt-level=N`），就得改成 `Vec<String>` 或 `Cow<'static, str>`。
- 「为什么用 `match self` 而不是给枚举加方法/散列表？」→ 穷尽性检查：`match` 在加新变体时**编译期报错**，编译器强制我补映射。用 `HashMap` 就失去这层保护，漏了只会在运行时变成「没有参数」。

#### 2.1.4 阶段展示文案（L46-68）

(Showing lines 318-335 of 3082. Use offset=336 to continue.)