# KillConfirmGameBar

简体中文 | [English](README.md)

KillConfirmGameBar 是适用于 Counter-Strike 2 和 CS:GO Legacy 的 Windows Xbox Game Bar 击杀确认悬浮窗。

它接收游戏状态，在击杀时播放对应音效并渲染动画。CS2 与 Legacy 共用一个小组件和本地服务，但两套游戏的兼容路径相互隔离。

本仓库是 [JamesHotten/csgo-cs2killpanel](https://github.com/JamesHotten/csgo-cs2killpanel) 的后续开发版本，基于 [eachkinji/CS2KillConfirmOverlay](https://github.com/eachkinji/CS2KillConfirmOverlay)。

## 主要功能

- Xbox Game Bar 悬浮窗，可调整位置、缩放、音量和音频输出设备。
- 支持普通击杀、爆头、刀杀、首杀、最终击杀、回合胜利和回合失败事件。
- 提供穿越火线、Valorant、CSOL、战地 1/4/5/2042、PUBG、三角洲行动、守望先锋、现代战争 2019、Apex、豆包、大狗叫和自定义模块表现风格。
- 提供多套穿越火线角色语音和 Valorant 武器终结音效。
- 提供 CSOL 1～10 杀图标、语音变体、特殊击杀优先级与首尾杀图标设置。
- BF1/BFV/BF4/BF2042/三角洲支持普通、爆头、刀杀和助攻的独立音效路由；不会改动动画事件。
- 设置主页支持自动或固定 100%～200% 的 Game Bar 控制面板高分辨率缩放。
- CF 与 Valorant 可单独启用助攻音效，CF 可分别设置爆头/刀杀音效及图标优先级。
- 兼容 CS2、CS:GO Legacy、原版机器人、接管机器人、观战、回放及受支持的机器人 Mod。
- 为每个观察目标保存独立击杀和助攻基线，避免观战或回放切换目标时串用计数。
- 设置主页可分别控制观战队友、回放视角和接管机器人击杀效果；三个开关默认开启，本地玩家事件始终保留。
- 设置可导出为带版本号的 JSON 并在另一台电脑导入；旧版设置会在启动或导入时自动迁移。
- 自定义模块支持 CS2 Customizer 图集、散帧、ZIP，以及 MP4/MOV/WebM/MKV/AVI 视频导入；当前等级缺少素材时会回退内置动画和默认音效。
- CFG 页面列出发现的全部 CS2/Legacy 安装路径及各自安装状态，可逐项选择和代写 CFG。
- 每种游戏风格的语音包和图标包选择会额外保存到 LocalState 备份文件，设置读取失败时可自动恢复。
- 在击杀发生时确定武器，击杀后快速切枪不会把效果错误标记成新武器。
- 对重复事件和回合结束顺序进行处理，避免胜利效果后再次播放同一击杀。
- 在受支持模式中显示武器、目标、胜负和连败经济奖励。

特殊资源取决于当前音效与特效包。如果某个包没有专属事件资源，悬浮窗会使用兼容的普通回退效果，不会故意播放空音效或空动画。

## 兼容范围

| 使用场景 | 普通击杀 | 接管机器人击杀 | 额外服务端组件 |
| --- | --- | --- | --- |
| CS2 官服或社区服 | 支持，来自客户端 GSI | 取决于服务器，不能读取远程日志 | 不需要 |
| CS2 本地监听服务器 | 支持 | 支持，需要已安装的本地日志配置 | 不需要 |
| CS:GO Legacy 本地或离线 | 支持，来自客户端 GSI | 支持，来自 Legacy Bridge | SourceMod |
| 本机 Legacy 专用服务器 | 支持 | 支持，读取服务器桥接日志 | 服务器安装 SourceMod |
| 远程 Legacy 服务器 | 客户端 GSI 支持 | 仅当桥接日志被安全同步或挂载到本机 | 服务器安装 SourceMod |

CS2 不使用 Legacy 的身份缓存或 SourceMod 去重路径。`Legacy Bridge` 只能安装到 CS:GO Legacy 客户端或 Legacy 服务器，不能安装到 CS2。

悬浮窗只渲染游戏主视角所代表的事件，包括正常游玩、观战、回放和受支持的机器人接管，不会为无关玩家播放效果。

完整的客户端、服务器和迁移步骤参见 [CS2 与 CS:GO Legacy 迁移和使用说明](docs/CS2_AND_LEGACY_MIGRATION.zh-CN.md)。
每种动画、音效、优先级、连杀窗口和资源回退选项参见 [特效与音效设置说明](docs/EFFECT_AUDIO_SETTINGS.zh-CN.md)。

## 使用要求

- Windows 10 或 Windows 11
- 已启用 Xbox Game Bar
- Counter-Strike 2 和/或 CS:GO Legacy

从源码构建还需要：

- Rust 工具链
- 安装 UWP、MSIX、Windows SDK 和 C++ 组件的 Visual Studio 或 Build Tools
- 构建可选 `.exe` 安装器时需要 Inno Setup 6
- 重新编译 Legacy Bridge 时需要兼容的 SourceMod 编译器

## 安装

普通用户应使用本仓库发布的安装包，运行安装器或转移包中的安装脚本。

安装完成后：

1. 按 `Win + G` 打开 Xbox Game Bar。
2. 打开 Kill Confirm Overlay 小组件，并按需固定。
3. 选择表现风格、音效包和音频输出设备。
4. 确认 GSI 已安装后启动游戏。

不要把解压后的 `InstalledPackage` 目录直接复制到另一台电脑。Game Bar 包必须完成注册，安装器或转移脚本还需要创建本机 Loopback 豁免。

## Game State Integration

本地服务只监听：

```text
http://127.0.0.1:10087/
```

安装器会尝试自动放置 `gamestate_integration_killconfirm.cfg`。手动安装位置为：

```text
<CS2 根目录>\game\csgo\cfg\gamestate_integration_killconfirm.cfg
<Legacy 根目录>\csgo\cfg\gamestate_integration_killconfirm.cfg
```

添加或修改 GSI 后必须完全退出并重新启动游戏。系统代理可能拦截本机 HTTP；如果小组件已连接但收不到游戏数据，可临时关闭 Clash 类工具的系统代理模式进行排查。

在 CS2 本地监听服务器中接管机器人时，服务器启动后执行：

```text
exec killconfirm_local_server
```

安装器会把该配置放入 CS2 的 `cfg` 目录。它启用本机服务器日志，以补充 GSI 缺失的接管事件；远程联机服务器不适用。

服务控制接口使用每次安装独立生成的令牌。不要复制或公开 `service-auth-token.txt`，也不要将所选本地端口暴露到局域网或公网。

## 经济奖励

默认的 `rules` 模式根据 Counter-Strike 规则计算奖励，不会把每一次金钱变化都当作奖励。

它会区分 CS2 与 CS:GO Legacy 的武器击杀、炸弹和人质目标、回合胜利、回合失败、下包补偿、连败补偿及人数短缺奖励。CS2 竞技/搭档还包含 CT 每消灭一名 T、全队每人 `$50` 的新增团队奖；Legacy 不会套用该规则。

军备竞赛、死亡竞赛、爆破、头号特训、训练及无法识别的自定义模式不会显示虚构的经济奖励，因为它们不属于当前支持的标准现金经济。

设置中仍提供实验性的 GSI 金钱差值模式用于验证，但推荐并默认使用 `rules`。

完整数值、两代差异和特殊边界见 [CS2 与 CS:GO Legacy 经济奖励规则](docs/CS2_CSGO_ECONOMY_RULES.zh-CN.md)。

## Legacy Bridge

Legacy 接管机器人后的击杀需要：

```text
KillConfirmService\legacy_bridge\killconfirm_bridge.smx
```

将它安装到 Legacy 客户端或服务器的 SourceMod `plugins` 目录。桥接器只转发接管后的击杀；普通真人击杀和观战目标击杀仍由 GSI 提供，避免一次击杀播放两遍。

如果 Legacy 专用服务器运行在本机，将服务器根目录逐行写入：

```text
%LOCALAPPDATA%\Packages\KillConfirmGameBar.Overlay_5jgcw66eyez0m\LocalState\legacy-bridge-servers.txt
```

当前桥接方式读取本机文件，不是没有鉴权的网络转发服务。远程服务器必须自行安全同步或挂载桥接日志。

## 从源码构建

先运行服务测试：

```powershell
cd KillConfirmService
cargo fmt -- --check
cargo check
cargo test
cd ..
```

构建带测试证书的 MSIX Bundle（不会安装）：

```powershell
.\Build-DevPackage.ps1 -Configuration Release
```

生成一键 `.exe` 安装器（带依赖新人版和无依赖更新版）：

```powershell
.\Build-ReleaseInstaller.ps1 -Configuration Release
```

首次 Release 构建会下载并校验固定版本的 LGPL FFmpeg，用于自定义模块视频导入；最终安装包已经携带 FFmpeg，用户安装时不需要联网下载。除非明确给 `Build-DevPackage.ps1` 传入 `-Install`，以上命令都不会安装或替换当前 Game Bar 包。

`-DisableSigning` 只适用于开发注册流程。公开分发应使用受信任的签名证书或签名服务。

## 项目结构

| 路径 | 用途 |
| --- | --- |
| `KillConfirmService` | Rust GSI 服务、音频引擎、事件状态、经济规则及兼容桥接 |
| `Widget` | UWP Xbox Game Bar 界面和动画渲染 |
| `Package` | MSIX 打包项目和清单 |
| `Installer` | 转移包及 Inno Setup 支持文件 |
| `SourceAssets/GameStyles` | 各风格的源音频、动画、图标和音效包 |
| `docs` | 迁移、双游戏部署和故障排查文档 |

构建脚本会从 `SourceAssets` 刷新打包资源。应修改源资源，不要直接修改小组件或服务输出目录中的生成副本。

## 故障排查

小组件能够打开但没有音效或动画时：

- 确认 GSI 位于当前实际运行版本的游戏目录。
- 修改 GSI 后完全重启游戏。
- 保持小组件打开或固定，并检查服务和 GSI 状态。
- 确认没有其他程序占用所选本地端口（默认 `127.0.0.1:10087`）。
- 临时关闭系统代理进行测试。
- Legacy 接管问题需检查 `sm plugins list` 和桥接日志。

日志位置：

```text
%LOCALAPPDATA%\Packages\KillConfirmGameBar.Overlay_5jgcw66eyez0m\LocalState\service.log
<Legacy 根目录>\csgo\addons\sourcemod\logs\killconfirm_bridge.log
```

报告问题时，请提供游戏版本、模式、是否接管机器人、击杀的大致时间、当前音效包，以及对应时间附近的日志。

## 致谢

Rust 服务基于 [st0nie/cskillconfirm](https://github.com/st0nie/cskillconfirm) 和 [st0nie/gsi-cs2-rs](https://github.com/st0nie/gsi-cs2-rs)。

项目还使用了 [MinecraftGD656/gd656killicon](https://github.com/MinecraftGD656/gd656killicon) 以及 [Steam 创意工坊项目 2721562982](https://steamcommunity.com/sharedfiles/filedetails/?id=2721562982) 的资源或设计参考。

## 许可证

本项目使用 [GNU Affero General Public License v3.0](LICENSE)。

本项目为社区项目，与 Valve、Microsoft、Xbox、Smilegate、Riot Games、Electronic Arts、Krafton、腾讯或其他游戏发行商均无官方关联。
