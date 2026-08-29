# 阿飘道长桌宠

一个由 Vue、Rust 和 Tauri 构建的治愈系跨平台桌面宠物。阿飘道长会常驻在桌面边缘，自动行走、呼吸、眨眼，并通过托盘菜单、右键菜单和提醒窗口与你保持轻量互动。

> 当前版本：`1.1.0`  
> 应用标识：`com.grass.tuanzi`  
> 界面语言：简体中文（`zh-CN`）

## 功能

- **透明桌宠窗口**：无边框、透明、置顶显示，不占用任务栏位置。
- **自主活动**：支持待机、呼吸、眨眼、行走、贴边探头等动画状态。
- **拖拽移动**：按住桌宠即可拖到桌面任意位置，支持边缘吸附。
- **轻量互动**：点击桌宠触发反馈动画；内置“念诵金光咒”互动配置和反馈文案。
- **提醒**：创建自定义时间提醒，到点触发桌宠动画和系统通知。
- **道观面板**：查看好感度、心情、今日互动次数和陪伴时长。
- **桌宠设置**：可配置边缘吸附、始终置顶、鼠标穿透和桌宠大小（小 / 中 / 大）。
- **系统托盘**：从托盘显示或隐藏桌宠、打开道观和提醒窗口，或退出应用。
- **多实例**：可再召唤新的阿飘；多个实例共享设置、提醒和互动统计。
- **本地持久化**：设置、提醒和陪伴数据保存在本机应用数据目录，不依赖远程服务。

## 技术栈

- [Vue 3](https://vuejs.org/) + TypeScript：界面、动画状态机和多窗口页面。
- [Tauri 2](https://tauri.app/)：桌面窗口、托盘、通知、进程和 IPC 能力。
- Rust：窗口管理、拖拽、自动行走、提醒调度和本地状态持久化。
- Vite：前端开发服务器和多页面构建。
- WebdriverIO：Tauri 端到端测试。
- pnpm：JavaScript 依赖和脚本管理。

## 快速开始

### 环境要求

- Node.js `24`（项目要求 Node.js `>=22`，仓库通过 `.node-version` 固定为 `24`）
- pnpm `11.17.0`
- Rust `1.96.1`，工具链配置见 [`rust-toolchain.toml`](./rust-toolchain.toml)
- Tauri 2 所需的系统开发依赖

在 macOS 和 Windows 上，通常只需要安装 Node.js、pnpm、Rust 和 Tauri 的构建前置依赖即可。Linux 开发还需要 WebKitGTK、GTK、AppIndicator、OpenSSL 和 `patchelf` 等系统包。

### 安装依赖

```bash
corepack enable
pnpm install --frozen-lockfile
```

### 启动开发环境

```bash
pnpm dev
```

该命令会启动 Vite 开发服务器，并通过 Tauri 打开桌宠应用。也可以使用等价命令：

```bash
pnpm start
# 或
pnpm run tauri:dev
```

## 构建与打包

### 构建前端

```bash
pnpm run vue:build
```

构建产物输出到 `dist-vue/`。项目使用 Vite 多页面入口，包含：

- `index.html`：桌宠窗口
- `dashboard.html`：道观面板
- `reminder.html`：提醒窗口
- `context-menu.html`：右键菜单窗口

### 构建 Tauri 应用

```bash
# 构建未打包的 Tauri 可执行文件
pnpm run tauri:build

# 构建 Tauri 安装包 / 发布包
pnpm run tauri:bundle
```

常用平台命令：

```bash
pnpm run package:win
pnpm run make:win
pnpm run package:mac
pnpm run make:mac
```

仓库当前以 Windows x64 为主要启用目标；CI 和发布流程中也保留 macOS 构建与检查。Linux AppImage / DEB 发布目前暂时关闭，恢复时需要同步更新发布矩阵和资产校验。

构建产物通常位于 `src-tauri/target/` 下。未签名构建可能会触发操作系统的安全提示，正式分发前请根据目标平台配置代码签名和公证。

## 应用内更新

应用内更新使用 Tauri updater，并从 GitHub Releases 的 `latest.json` 获取签名发布信息。每次打开“道观”面板都会自动检查一次，也可以在“应用更新”区域手动检查、下载并重启安装。

首次启用发布更新前，需要把本机临时目录中的 `/tmp/grass-pet-updater.key` 内容保存为 GitHub Actions Secret `TAURI_SIGNING_PRIVATE_KEY`，并设置 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`（当前生成的开发密钥无密码时留空即可）。对应的公钥已经写入 [`src-tauri/tauri.conf.json`](./src-tauri/tauri.conf.json)，不要提交私钥或更换公钥后继续使用旧签名密钥。

发布工作流会通过 `tauri-apps/tauri-action` 生成安装包签名和 `latest.json`。如果更新检查提示暂无发布元数据，请先确认对应版本的 GitHub Release 已发布，并且 Actions 中配置了上述签名 Secret。

## 测试与质量检查

运行前端单元测试：

```bash
pnpm test
```

运行前端和 Rust 测试：

```bash
pnpm run test:tauri
```

运行完整检查：

```bash
pnpm run check
```

完整检查包含 lint、TypeScript 类型检查、Rust 检查、Tauri IPC 契约校验、桌宠规格校验、素材引用校验，以及 UI / 体验 / 素材 QA。

Tauri 端到端测试：

```bash
pnpm run test:e2e
```

其他常用检查命令：

```bash
pnpm run lint
pnpm run check:frontend
pnpm run check:rust
pnpm run fmt:check
pnpm run preflight
pnpm run doctor
pnpm run qa:ui
pnpm run qa:experience
pnpm run qa:assets
```

`doctor` 会生成 `.build/doctor-tauri-report.json`；QA 报告和临时构建目录默认不会提交到 Git。

## 项目结构

```text
.
├── index.html                    # 桌宠入口
├── dashboard.html                # 道观入口
├── reminder.html                 # 提醒入口
├── context-menu.html             # 右键菜单入口
├── src-vue/
│   ├── pet/                      # 桌宠界面、动画和状态机
│   ├── dashboard/                # 道观面板
│   ├── reminder/                 # 提醒窗口
│   ├── context-menu/             # 右键菜单
│   └── shared/                   # Vue 与 Tauri 之间的 API 封装
├── src/
│   ├── assets/pet/               # 桌宠动画帧和角色素材
│   └── shared/                   # 前后端共享类型与工具
├── src-tauri/
│   └── src/
│       ├── commands.rs           # Tauri IPC 命令
│       ├── windows.rs            # 窗口、拖拽、自动行走和命中检测
│       ├── reminders.rs          # 提醒调度器
│       ├── state.rs              # 本地状态与统计持久化
│       ├── tray.rs               # 系统托盘菜单
│       └── context_menu.rs       # 右键菜单窗口定位和控制
├── pet-spec.json                 # 角色、动画、主题和构建规格
├── tools/                        # 素材处理、规格校验和 QA 工具
├── tests/unit/                   # TypeScript 单元测试
├── tests/tauri-e2e/              # Tauri 端到端测试
└── qa/manual-checklist.md        # 真人验收清单
```

## 配置与数据

角色、动画状态、主题色、互动定义和构建目标集中在 [`pet-spec.json`](./pet-spec.json) 中。修改角色素材或动画状态时，应同时运行：

```bash
node tools/validate-spec.mjs
node tools/validate-asset-links.mjs
pnpm run qa:assets
```

应用运行数据由 Rust 端统一管理，主要包括：

- `settings`：边缘吸附、置顶、鼠标穿透和桌宠大小；
- `reminders`：提醒内容、时间和创建时间；
- `stats`：好感度、心情、今日互动次数和陪伴时长。

数据写入 Tauri 的应用数据目录，并采用临时文件加重命名的方式进行原子保存。端到端测试可以通过 `GRASS_PET_E2E_DATA_DIR` 使用隔离的数据目录。

## 开发约定

1. 前端与 Rust 之间新增或修改 IPC 时，同时更新 [`src/shared/contracts.ts`](./src/shared/contracts.ts)、Vue API 封装和 Rust 命令注册。
2. 修改窗口行为、拖拽、贴边或鼠标穿透逻辑后，至少运行 `pnpm run test:tauri`，并检查 Tauri E2E 场景。
3. 新增或替换 PNG 素材后，确保素材被 `pet-spec.json` 引用，并运行素材校验。
4. 不要手工把 ZIP 重命名为安装包；安装版、便携版和签名产物应通过 Tauri 的对应构建流程生成。
5. 提交前建议运行 `pnpm run check`，并根据 [`qa/manual-checklist.md`](./qa/manual-checklist.md) 完成目标平台验收。

## 常见问题

### 为什么启动后看不到桌宠？

桌宠窗口默认是透明、无边框并且不显示在任务栏中。可以从系统托盘打开或显示桌宠；如果仍不可见，请先运行：

```bash
pnpm run doctor
```

并检查 Tauri 日志及生成的诊断报告。

### 为什么提醒没有弹出系统通知？

首次使用提醒时，应用会请求系统通知权限。请在操作系统设置中允许“阿飘道长桌宠”发送通知；即使拒绝通知权限，提醒窗口中的到期事件仍由应用调度器处理。

### 如何清理本地状态？

退出应用后，删除 Tauri 应用数据目录中的状态文件即可重置设置、提醒和统计。不同操作系统的目录位置不同，建议通过系统应用数据目录查找应用标识 `com.grass.tuanzi`。删除前请确认不需要保留提醒和陪伴数据。

## 许可证

当前项目在 [`package.json`](./package.json) 中声明为 `UNLICENSED`，暂未发布开源许可证。未经项目维护者授权，请不要将项目或角色素材用于再分发。
