<div align="center">
    <img src="asset/icon.png" width="100" height="100" alt="Vim Key Remap"/>
    <p style="font-size: 25px;">Vim Key Remap</p>
    <p> 一个简单的Windows平台CapsLock键映射工具</p>
</div>

## 简介

在 vim 和 平时使用键盘时，ESC 键和 Ctrl 键使用最为频繁。为了操作便捷，我产生了将短按 CapsLock 作为 ESC 键，长按 CapsLock 作为左 Ctrl 键的想法。这个工具实现了这个功能。

## 特性

-   **短按 CapsLock**：映射为 ESC 键
-   **长按 CapsLock**：映射为左 Ctrl 键
-   **系统托盘运行**：后台运行，托盘图标管理

## 使用

1. 下载最新版本的`vim-key-remap.exe`和`install-interception.exe`
2. 以管理的身份运行一个 CMD 窗口，进入`install-interception.exe`文件所在目录
3. 运行命令`install-interception.exe /install`安装驱动
4. 双击运行`vim-key-remap.exe`，看到后台运行的托盘图标即表示启动成功了

> [!TIP]
>
> -   工具仅在 Windows 平台下生效, 且仅支持 x64 架构
> -   后续如果需要卸载驱动，运行`install-interception.exe /uninstall`即可

## 感谢

本项目使用了 [Interception](https://github.com/oblitum/Interception) 库来实现键盘按键的底层拦截和修改，感谢该库的作者和贡献者。
