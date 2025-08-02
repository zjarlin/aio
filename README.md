# Vben Admin Tauri 桌面应用

<div align="center">
  <img alt="VbenAdmin Logo" width="215" src="https://unpkg.com/@vbenjs/static-source@0.1.7/source/logo-v1.webp">
  <br>
  <br>

[![license](https://img.shields.io/github/license/anncwb/vue-vben-admin.svg)](LICENSE)

</div>

## 简介

这是一个基于 [Vben Admin](https://github.com/vbenjs/vue-vben-admin) 构建的 Tauri 桌面应用程序模板。它将现代化的 Vue 3、Vite 和 TypeScript 技术栈与 Tauri 框架结合，创建了一个功能强大的桌面应用程序解决方案。

## 特性

- **现代化技术栈**：Vue 3、Vite、TypeScript
- **桌面应用支持**：使用 Tauri 构建跨平台桌面应用
- **开箱即用**：预配置的构建和开发环境
- **高性能**：Tauri 提供接近原生的性能
- **小体积**：相比 Electron，生成的应用程序体积更小
- **安全性**：Tauri 的安全模型保护应用免受攻击

## 安装和使用

### 环境要求

在开始之前，请确保您已安装以下依赖：

- [Rust](https://www.rust-lang.org/)
- [Node.js](https://nodejs.org/) (推荐使用 LTS 版本)
- [pnpm](https://pnpm.io/)

### 获取项目代码

```bash
git clone https://github.com/Lhy723/vben-admin-tauri-app.git
```

### 安装依赖

```bash
cd vben-admin-tauri-app

npm i -g corepack

pnpm install
```

### 开发模式

```bash
# 启动 Tauri 开发模式
pnpm tauri dev
```

### 构建应用

```bash
# 构建 Web 应用
pnpm build

# 构建 Tauri 桌面应用
pnpm tauri build
```

## 项目结构

```
.
├── apps/
│   ├── backend-mock/     # 模拟后端服务
│   └── web-ele/          # Web 应用前端代码
├── internal/             # 内部工具和配置
├── packages/             # 核心包和组件库
├── src-tauri/            # Tauri 应用源代码
│   ├── src/              # Rust 源代码
│   ├── tauri.conf.json   # Tauri 配置文件
│   └── Cargo.toml        # Rust 依赖配置
└── scripts/              # 构建和部署脚本
```

## 配置

### Tauri 配置

Tauri 配置文件位于 `src-tauri/tauri.conf.json`，您可以在此配置应用的基本信息、窗口设置、安全策略等。

主要配置项包括：

- `productName`: 应用名称
- `version`: 应用版本
- `identifier`: 应用唯一标识符
- `windows`: 窗口配置（标题、尺寸等）

### 应用配置

应用配置位于 `apps/web-ele/src/preferences.ts`，您可以在此配置应用的主题、布局、功能开关等。

## 开发指南

### 添加新页面

1. 在 `apps/web-ele/src/views/` 目录下创建新页面组件
2. 在 `apps/web-ele/src/router/routes/` 目录下添加路由配置
3. 如需要，更新菜单配置

### 自定义组件

自定义组件应放置在 `packages/` 目录下相应的包中：

- UI 组件: `packages/@core/ui-kit/`
- 业务组件: `packages/effects/common-ui/src/components/`
- 工具组件: `packages/utils/src/`

### 国际化

国际化文件位于 `apps/web-ele/src/locales/` 和 `packages/locales/src/langs/` 目录下。支持中文和英文，默认语言可以在配置文件中设置。

## 构建和发布

### 构建应用

```bash
# 构建 Web 应用
pnpm build

# 构建 Tauri 应用
pnpm tauri build
```

构建后的应用将位于 `src-tauri/target/release/bundle/` 目录下。

### 支持的平台

- Windows (MSI, EXE)
- macOS (DMG, APP)
- Linux (AppImage, Deb, RPM)

## 贡献

欢迎提交 Issue 和 Pull Request 来改进这个项目。

## 许可证

[MIT © Vben](./LICENSE)

---

**注意**: 这是一个基于 Vben Admin 的 Tauri 桌面应用模板。有关 Vben Admin 的更多信息，请访问 [Vben Admin 官方文档](https://doc.vben.pro/)。
