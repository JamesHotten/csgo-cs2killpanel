# 测试目录

- `Regression/`：PowerShell 与 MSBuild 回归测试，从仓库根目录运行，例如 `pwsh -File Tests/Regression/Test-DanmakuPacing.ps1`。
- `CustomSequences/`：自定义模块的 C# 测试工具和参考实现对照脚本。
- `fixtures/`：Rust 服务与音频清单测试使用的固定输入。

本次启动与预览修复可用 `Regression/Test-DanmakuLaneLayout.ps1` 验证实际轨道间距，用 `Regression/Test-OverwatchPreview.ps1` 验证守望先锋首次加载期间的连续事件、最新五条保留、助攻标记和取消后的状态隔离。

测试脚本自行从当前位置解析仓库根目录，不依赖当前工作目录。构建产物统一写入仓库根目录的 `Output/`，不得提交。
