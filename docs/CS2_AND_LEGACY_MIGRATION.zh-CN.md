# KillConfirmGameBar：CS2 与 CS:GO Legacy 迁移和使用说明

本文说明如何把当前版本迁移到另一台 Windows 电脑、另一个 Counter-Strike 安装目录，或一台本机 CS:GO Legacy 专用服务器。当前版本允许 CS2 与 CS:GO Legacy 同时存在；两者共用 Xbox Game Bar 小组件和本地服务，但事件来源不同。

## 1. 工作方式

| 场景 | 基础事件来源 | 受控人机击杀来源 | 是否需要 SourceMod |
| --- | --- | --- | --- |
| CS2 | 客户端 GSI | 本地监听服务器使用 GSI + 本地服务器日志；远程服务器不保证支持 | 否 |
| CS:GO Legacy 本地/离线模式 | 客户端 GSI | Legacy Bridge 日志 | 是 |
| CS:GO Legacy 专用服务器 | 客户端 GSI | 服务器 Legacy Bridge 日志 | 服务器需要 |

本地服务监听 `127.0.0.1:3000`。CS2 始终走原来的 GSI 路径；Legacy 专用的身份缓存、SourceMod 日志和短时去重不会应用到 CS2。

Legacy Bridge 只转发“玩家已接管机器人”之后的击杀。普通真人击杀和观察目标击杀仍由 GSI 处理，以避免同一次击杀播放两遍。

## 2. 需要迁移的文件

| 文件 | 用途 |
| --- | --- |
| Windows 安装器或 `Build-TransferPackage.ps1` 生成的转移包 | 安装 Game Bar 小组件、本地服务、音效和动画 |
| `KillConfirmService/gsi/gamestate_integration_killconfirm.cfg` | CS2 和 Legacy 客户端的 GSI 配置 |
| `KillConfirmService/legacy_bridge/killconfirm_bridge.smx` | Legacy SourceMod 运行插件 |
| `KillConfirmService/legacy_bridge/killconfirm_bridge.sp` | SourcePawn 源码；运行时不是必需文件 |

不要只复制 `InstalledPackage` 到另一台电脑。Game Bar 包需要注册、证书和 loopback exemption；应使用安装器或转移包里的安装脚本。

`service-auth-token.txt` 是本机控制令牌，不应共享或复制到其他电脑。新电脑首次运行时会自行生成。

## 3. 安装公共组件

在目标电脑上运行发布版安装器，或者在源码根目录构建转移包：

```powershell
.\Build-TransferPackage.ps1
```

运行转移包中的安装脚本后：

1. 按 `Win + G` 打开 Xbox Game Bar。
2. 打开 Kill Confirm Overlay 小组件。
3. 确认小组件显示本地服务已连接。
4. 在控制面板选择音效包和连续击杀规则。

安装脚本会为包族 `KillConfirmGameBar.Overlay_5jgcw66eyez0m` 添加本机 loopback exemption。缺少该权限时，小组件无法访问 `127.0.0.1:3000`。

### 3.1 各场景快速开启检查表

以下步骤中的“击杀人机”是指玩家仍控制自己的角色、目标是机器人；“接管人机”是指玩家死亡后控制队友机器人，两者不要混淆。

| 场景 | 必须开启或安装的内容 | 启用方法 |
| --- | --- | --- |
| CS2 普通对局、击杀普通人机、观战或回放 | Game Bar 小组件 + CS2 GSI | 安装公共组件，把 GSI 配置放入 CS2 `cfg`，完全重启 CS2，然后用 `Win + G` 打开并固定小组件 |
| CS2 本地/离线接管人机 | 上一项 + 本地服务器日志 | 每次启动本地监听服务器后在 CS2 控制台执行 `exec killconfirm_local_server` |
| CS2 官方或远程社区服务器接管人机 | 只能使用客户端 GSI | 无法执行或读取服务器日志时，接管击杀不保证能够识别；不要安装 Legacy Bridge |
| Legacy 普通对局、击杀普通人机、观战或回放 | Game Bar 小组件 + Legacy GSI | 把 GSI 配置放入 Legacy `cfg`，完全重启 Legacy，然后打开并固定小组件 |
| Legacy 本地/离线接管人机（含支持的机器人 Mod） | 上一项 + Metamod:Source + SourceMod + Legacy Bridge | 把 `killconfirm_bridge.smx` 放入本地 Legacy 的 SourceMod `plugins`，重新加载插件或重启游戏/地图 |
| 本机 Legacy 专用服务器接管人机 | 客户端 GSI + 服务器 SourceMod/Bridge + 本地日志路径 | 在服务器安装 Bridge，把服务器根目录写入 `legacy-bridge-servers.txt`，再重启本地服务或重新打开小组件 |
| 另一台电脑上的 Legacy 专用服务器接管人机 | 上一项 + 安全可见的 Bridge 日志 | 当前版本没有网络桥接；必须把服务器日志安全同步或挂载到运行 Game Bar 的电脑 |

所有场景都使用同一套特效和音效设置。打开小组件后选择游戏风格、语音包和图标包；风格专用选项在“高级特效”面板中。普通功能不需要分别为 CS2 和 Legacy 重复设置。

## 4. CS2 部署

CS2 不需要 SourceMod。把 GSI 配置放入：

```text
<CS2 根目录>\game\csgo\cfg\gamestate_integration_killconfirm.cfg
```

常见 Steam 路径示例：

```text
C:\Program Files (x86)\Steam\steamapps\common\Counter-Strike Global Offensive\game\csgo\cfg\
```

配置中的服务地址和令牌应保持为：

```text
"uri" "http://127.0.0.1:3000/"
"auth"
{
  "token" "killconfirm"
}
```

放入配置后必须完全退出并重新启动 CS2。CS2 与 Legacy 的 GSI 配置可以同时存在，无需切换。

验证方法：

1. 打开 Game Bar 小组件。
2. 进入一局 CS2。
3. 小组件的 GSI 状态应变为最近收到数据。
4. 分别测试普通击杀、爆头、刀杀和回合最后一杀。

不要把 `killconfirm_bridge.smx` 安装到 CS2。当前桥接器使用 Source 1/CS:GO Legacy 的 SourceMod 事件，只用于 Legacy。

## 5. CS:GO Legacy 客户端部署

### 5.1 安装 GSI

把同一份 GSI 配置放入：

```text
<Legacy 根目录>\csgo\cfg\gamestate_integration_killconfirm.cfg
```

当前电脑示例：

```text
D:\SteamLibrary\steamapps\common\csgo legacy\csgo\cfg\
```

放置后完全重启 CS:GO Legacy。

### 5.2 安装受控人机桥接器

如果要在本地机器人模式、回防模式或带机器人 Mod 的本地服务器中支持“接管机器人后击杀”，目标 Legacy 安装必须已有 Metamod:Source 和 SourceMod。

复制运行插件：

```text
killconfirm_bridge.smx
  -> <Legacy 根目录>\csgo\addons\sourcemod\plugins\killconfirm_bridge.smx
```

源码可选复制到：

```text
killconfirm_bridge.sp
  -> <Legacy 根目录>\csgo\addons\sourcemod\scripting\killconfirm_bridge.sp
```

服务器或本地游戏已经运行时，在 SourceMod 控制台执行：

```text
sm plugins reload killconfirm_bridge
sm plugins list
```

首次安装也可以重启地图或游戏，让 SourceMod 自动加载 `plugins` 目录中的插件。正常日志应包含：

```text
READY|version=0.2.0
```

日志位置：

```text
<Legacy 根目录>\csgo\addons\sourcemod\logs\killconfirm_bridge.log
```

当前桥接器同时兼容原版机器人和会给机器人模拟真人 SteamID 的 Mod；判断依据是 `bot_takeover`，不是 SteamID 是否以 BOT 表示。

## 6. CS:GO Legacy 专用服务器部署

### 6.1 安装服务器插件

专用服务器必须安装 Metamod:Source 和 SourceMod。将文件复制到服务器实际游戏根目录，而不是外层管理目录：

```text
<服务器根目录>\csgo\addons\sourcemod\plugins\killconfirm_bridge.smx
```

当前服务器实际根目录为：

```text
D:\CSGOServer\server
```

因此插件路径为：

```text
D:\CSGOServer\server\csgo\addons\sourcemod\plugins\killconfirm_bridge.smx
```

服务器启动后用服务器控制台验证：

```text
sm plugins list
sm plugins info killconfirm_bridge
```

### 6.2 告诉本地服务服务器在哪里

创建或编辑：

```text
%LOCALAPPDATA%\Packages\KillConfirmGameBar.Overlay_5jgcw66eyez0m\LocalState\legacy-bridge-servers.txt
```

每行填写一个本机 Legacy 专用服务器路径，可填写实际根目录，也可填写包含 `server` 子目录的外层目录：

```text
# 本机专用服务器
D:\CSGOServer

# 也可以直接写实际根目录
# E:\Servers\CSGO\server
```

修改列表后重启 KillConfirm 本地服务或重新打开 Game Bar 小组件。服务日志会先显示：

```text
Legacy controlled-bot bridge waiting for server log: ...
```

服务器启动并创建日志后应变为：

```text
Legacy controlled-bot bridge connected: ...
```

服务从日志文件末尾开始监听，不会把服务器以前的击杀重新播放。

### 6.3 本机与远程服务器限制

当前桥接方式读取 SourceMod 生成的本地文件。因此：

- 游戏客户端、Game Bar 服务和专用服务器位于同一台 Windows 电脑时，可以直接使用。
- 专用服务器位于另一台电脑时，客户端无法直接看到服务器日志。需要把日志安全同步/挂载到客户端，或另行实现带身份验证的网络事件传输；当前版本没有内置远程桥接协议。
- 公网服务器不应直接开放本地服务的 `3000` 端口。该端口设计为仅绑定 `127.0.0.1`。

## 7. 同时支持 CS2 与 Legacy

推荐同时保留：

```text
<CS2>\game\csgo\cfg\gamestate_integration_killconfirm.cfg
<Legacy>\csgo\cfg\gamestate_integration_killconfirm.cfg
```

并只在 Legacy 客户端或 Legacy 服务器安装 `killconfirm_bridge.smx`。本地服务会根据 GSI provider 区分 CS2 与 Legacy：

- CS2 不读取 Legacy 身份缓存。
- CS2 不参与 SourceMod 桥接去重。
- SourceMod 日志只补充 Legacy 的受控人机击杀。
- 普通 GSI 音效、动画和武器判断继续共用同一套服务；经济奖励会根据 GSI 来源自动选择 CS2 或 CS:GO Legacy 的规则版本。

同一时间建议只运行一个会向 `127.0.0.1:3000` 发送 GSI 的 Counter-Strike 客户端。

## 8. 从源码构建和移植

### 8.1 构建本地服务和 Game Bar 包

环境要求：

- Rust 工具链
- Visual Studio/Build Tools，包含 C++、UWP/MSIX 工具
- 构建安装器时需要 Inno Setup 6

常用命令：

```powershell
cd KillConfirmService
cargo test
cargo build --release

cd ..
.\Build-IntegratedPackage.ps1
.\Build-TransferPackage.ps1
```

### 8.2 编译 Legacy Bridge

使用目标 SourceMod 自带的编译器，以 SourceMod 1.12 为例：

```powershell
& '<SourceMod>\scripting\spcomp64.exe' `
  '.\KillConfirmService\legacy_bridge\killconfirm_bridge.sp' `
  -i '<SourceMod>\scripting\include' `
  -o '.\KillConfirmService\legacy_bridge\killconfirm_bridge.smx'
```

编译应无 warning/error。将生成的 `.smx` 部署到目标 Legacy 的 `plugins` 目录。

### 8.3 保持双版本隔离的开发约束

继续开发时应遵守：

1. CS2 事件处理仍以 GSI 为唯一来源。
2. Legacy Bridge 只处理 `controlled=1` 的受控人机击杀。
3. Legacy 专用兼容和去重必须以 `is_legacy` 为条件。
4. 不要用玩家当前手持武器替代死亡事件里的击杀武器。
5. 新改动至少运行：

```powershell
cargo fmt -- --check
cargo check
cargo test
git diff --check
```

还应在 CS2、Legacy 本地机器人和 Legacy 专用服务器各进行一次实机击杀测试。

## 9. 故障排查

### Game Bar 无法打开

先尝试 `Win + G`。如果 Game Bar 进程挂起，可在 PowerShell 重启宿主：

```powershell
Get-Process GameBar,GameBarFTServer -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Process 'ms-gamebar:'
```

### 游戏有 GSI，但没有音效或动画

- 检查 GSI 配置是否放在当前实际运行版本的 `cfg` 目录。
- 放置配置后完全重启游戏。
- 确认没有其他程序占用 `127.0.0.1:3000`。
- 系统代理可能阻止本机 GSI；可临时关闭 Clash 类系统代理测试。
- 检查 Game Bar 小组件是否保持打开或固定。

### Legacy 普通击杀有效，但接管机器人后无效果

- 检查 `sm plugins list` 中是否存在 `KillConfirm Legacy Bridge 0.2.0`。
- 检查 `killconfirm_bridge.log` 是否出现 `BOT_TAKEOVER`。
- 专用服务器需要在服务器端安装 `.smx`，只安装到客户端无效。
- 检查 `legacy-bridge-servers.txt` 是否指向正确服务器根目录。
- 检查本地服务日志是否出现对应的 `bridge connected`。

### 一次击杀播放两次

- 确保只加载一份 `killconfirm_bridge.smx`。
- 删除或停用旧版桥接器，使用 `0.2.0`。
- 执行 `sm plugins reload killconfirm_bridge` 或重启服务器。

### 日志位置

本地服务日志：

```text
%LOCALAPPDATA%\Packages\KillConfirmGameBar.Overlay_5jgcw66eyez0m\LocalState\service.log
```

Legacy Bridge 日志：

```text
<Legacy 或服务器根目录>\csgo\addons\sourcemod\logs\killconfirm_bridge.log
```

报告问题时请提供游戏模式、是否接管机器人、击杀时间、当前音效包，以及上述两份日志中对应时间附近的内容。

