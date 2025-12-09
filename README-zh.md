# `tree++`: 适用于Windows更好的`tree`命令  

Windows平台上的`tree`命令自从近40年前发布以来几乎就没有改动. 在如今LLM的时代, 作为描述项目结构非常常用的工具, 仅有的`/f`和`/a`两个参数显然捉襟见肘.  

`tree++`是对`tree`的一次全面升级, 开源于[GitHub](https://github.com/Water-Run/treepp)包括:  

- 兼容原有的Windows`tree` , 并包括扩展指令集  
- 兼容Linux的`tree`, 包含主要参数和功能  
- 优化算法, 比`tree`更快, 尤其是在大且复杂的文件夹中  

`wintree++`使用`zig`实现, 并提供预编译版本在[GitHub Release]()中. 只需要下载并添加到环境变量(确保在`C:\Windows\System32`之前)即可使用. 你也可以自行编译.  

## 指令扩展  

关于Windows和Linux的`tree`命令使用, 参见:  

### `\v`: 获取版本  

```cmd
tree \v
```

```plaintext
wintree++ by WaterRun
ver 1.0
https://github.com/Water-Run/wintreepp
```
