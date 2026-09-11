# 开发工具

日常构建和发布入口仍保留在仓库根目录：`Build-DevPackage.ps1`、`Build-ReleaseInstaller.ps1` 与相关依赖脚本。

以下命令从仓库根目录运行：

- `python Tools/Crossfire/Build-CrossfireExternalPacks.py --help`：把旧 CF 素材归档转换为独立图标包和语音包。
- `python Tools/Crossfire/Build-CrossfirePacks.py --help`：把分类目录中的 CF 图标套装批量打包。
- `pwsh -File Tools/Danmaku/Start-DanmakuAnnotationGui.ps1`：启动弹幕标注审核界面，需要 PowerShell 7 和 Python。

CF 工具说明见 [CF 图标包文档](../docs/crossfire-icon-packs.md)，弹幕工具说明见 [标注说明](../Widget/Danmaku/Annotation/README.md)。
