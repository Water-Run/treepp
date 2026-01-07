# `tree++`: 好的多的Windows`tree`命令

*[English](./README.md)*

Windows上的`tree`命令自从近40年前发布以来几乎就没有改动. 在如今LLM的时代, 作为描述项目结构非常常用的工具, 仅有的`/f`和`/a`两个参数的功能显然捉襟见肘. 同时, 它很慢.  

**`tree++`是对`tree`的一次全面升级**, 为Windows平台下的`tree`命令引入了:  

- ***扩展参数集, 支持功能涵盖包括显示文件大小, 递归深度限制, 修改输出风格, 将结果输出至文件, 及排除指定目录(包括遵循`.gitignore`)等常用功能***  
- ***支持多线程, 在大且复杂的目录中提供显著的性能提升***
- ***与原有的Windows`tree`命令参数和输出完全兼容, 并兼容使用Unix风格的参数(如`-f`和`--files`)***

**`tree++`使用`Rust`实现**, 开源于[GitHub](https://github.com/Water-Run/treepp).  

*性能对比(以`C:\Windows`为示例):*  

| 类型                | 耗时(`ms`)   | 倍率    |
| ----------------- | ---------- | ----- |
| 原生`tree`          | `34055.50` | 1.0x  |
| `treepp`(默认, 8线程) | `3480.12`  | 9.79x |
| `treepp`(1线程)     | `6687.58`  | 5.09x |

## 安装

从[Release](https://github.com/Water-Run/treepp/releases/tag/0.1.0)下载`tree++.zip`, 解压到合适目录, 并将目录添加至环境变量.  

开启Windows终端, 执行:  

```powershell
treepp /v
```

有输出:  

```plaintext
tree++ version 0.1.0

A much better Windows tree command.

author: WaterRun
link: https://github.com/Water-Run/treepp
```

即完成安装.  

随后, 你可以以和普通的Windows `tree`命令一样的方式使用:  

```powershell
treepp /f
```

## 速览

| 参数集(等价写法)                     | 说明                                         |
|-------------------------------|--------------------------------------------|
| `--help` `-h` `/?`            | 显示帮助信息                                     |
| `--version` `-v` `/V`         | 显示版本信息                                     |
| `--ascii` `-a` `/A`           | 使用 ASCII 字符绘制树                             |
| `--files` `-f` `/F`           | 显示文件                                       |
| `--full-path` `-p` `/FP`      | 显示完整路径                                     |
| `--human-readable` `-H` `/HR` | 以人类可读方式显示文件大小                              |
| `--no-indent` `-i` `/NI`      | 不显示树形连接线                                   |
| `--reverse` `-r` `/R`         | 逆序排序                                       |
| `--size` `-s` `/S`            | 显示文件大小(字节)                                 |
| `--date` `-d` `/DT`           | 显示最后修改日期                                   |
| `--exclude` `-I` `/X`         | 排除匹配的文件                                    |
| `--level` `-L` `/L`           | 限制递归深度                                     |
| `--include` `-m` `/M`         | 仅显示匹配的文件                                   |
| `--disk-usage` `-u` `/DU`     | 显示目录累计大小                                   |
| `--ignore-case` `-c` `/IC`    | 匹配时忽略大小写                                   |
| `--report` `-e` `/RP`         | 显示末尾统计信息                                   |
| `--prune` `-P` `/P`           | 修剪空目录                                      |
| `--sort` `-S` `/SO`           | 指定排序方式(`name`、`size`、`mtime` 等)            |
| `--no-win-banner` `-N` `/NB`  | 不显示 Windows 原生 tree 的样板信息                  |
| `--silent` `-l` `/SI`         | 终端静默(结合`output`指令使用)                       |
| `--output` `-o` `/O`          | 将结果输出至文件(`.txt`, `.json`, `.yml`, `.toml`) |
| `--thread` `-t` `/T`          | 扫描线程数(默认为8)                                |
| `--gitignore` `-g` `/G`       | 遵循`.gitignore`                             |
| `--quote` `-q` `/Q`           | 用双引号包裹文件名                                  |
| `--dirs-first` `-D` `/DF`     | 目录优先显示                                     |

> 完整参数集参阅: [tree++参数集文档](./OPTIONS-zh.md)
