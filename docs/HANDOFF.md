# Mad Library Server 交接文档

最后更新：2026-07-15

## 1. 项目定位

本仓库同时包含 Mad Library 团队服务端、浏览器管理后台和 Windows Tauri 桌面壳。

- Rust 服务端提供 HTTP API、管理后台静态文件、鉴权、资源库、成员、储存、备份和运行控制。
- React 管理后台同时运行在桌面 WebView 和普通浏览器中。
- Windows 桌面壳是拥有者使用的启动器和本机管理入口，不是唯一管理入口。
- 桌面安装版启动私有 PostgreSQL 与 Rust 服务；客户端和局域网浏览器只访问 HTTP 服务，不直接访问数据库。
- 默认 HTTP 地址为 `0.0.0.0:3789`，私有 PostgreSQL 仅监听 `127.0.0.1:54329`。

首个 Owner 只能从服务端主机的回环地址创建。初始化完成后，已授权管理员可从局域网浏览器访问后台。

## 2. 技术栈

- 服务核心：Rust、Axum、Tokio、SQLx、PostgreSQL、JWT
- 管理后台：React、TypeScript、Vite、Lucide React
- 桌面壳：Tauri 2、Rust、NSIS
- Windows 数据库：仓库内精简 PostgreSQL x64 运行时
- 二进制版本管理：Git LFS

## 3. 重要目录

```text
madlibrary-server/
|- src/                         Rust 服务端与数据库迁移
|- admin-ui/src/                管理后台源码
|- desktop/                     Tauri 桌面壳源码和安装配置
|- binaries/windows-x64/        随代码跟踪的精简 PostgreSQL 运行时
|- packaging/                   便携包定义和第三方运行时清单
|- scripts/                     开发、准备运行时和发布脚本
|- docs/                        架构、目录和交接文档
|- target/                      编译与桌面开发运行时，不提交
|- admin-ui/dist/               Vite 构建输出，不提交
`- artifacts/windows-x64/       最终安装包和校验文件，不提交
```

`desktop/bundle-resources/runtime/` 只是 Tauri 打包中间目录。除 `.gitkeep` 外均忽略，发布脚本从源码和 `binaries/` 重新生成内容。不要把安装后的数据库或测试数据放入仓库。

## 4. 本地开发

环境要求：Node.js/npm、稳定版 Rust/MSVC、Git LFS。正常开发和打包不需要本机安装 PostgreSQL。

首次拉取后执行：

```powershell
git lfs pull
cd .\admin-ui
npm ci
cd ..\desktop
npm ci
cd ..
```

启动完整桌面开发版：

```powershell
npm run desktop:dev
```

该命令会构建管理后台、准备 `target/desktop-runtime/`，然后启动 Tauri。不要同时再运行一份 `cargo run`，否则会争用端口或实例锁。

智能体注意：如果只在 `admin-ui/` 下执行 `npm run build`，Vite 只会更新 `admin-ui/dist/`。桌面开发运行时和 `http://127.0.0.1:3789/admin/` 实际读取的是 `target/desktop-runtime/admin-ui/` 的复制品。每次希望桌面端或 3789 后台看到最新前端时，构建后必须同步一次：

```powershell
robocopy admin-ui\dist target\desktop-runtime\admin-ui /E /R:0 /W:0 /NFL /NDL /NJH /NJS /NP
if ($LASTEXITCODE -gt 7) { exit $LASTEXITCODE }
```

也可以直接运行 `npm run desktop:dev`，该脚本会构建并准备 `target/desktop-runtime/`。若同步后页面仍旧，强刷浏览器或重启桌面窗口，让它重新加载新的 Vite hash 资源。

只开发管理后台时，可先启动服务端，再运行 Vite：

```powershell
cargo run --manifest-path .\Cargo.toml
cd .\admin-ui
npm run dev
```

Vite 默认在 `http://127.0.0.1:5179/admin/`，并将 API 代理到 `http://127.0.0.1:3789`。

## 5. 运行模型与数据目录

桌面安装版的可执行文件和 PostgreSQL 程序文件位于安装目录，只读运行数据统一位于：

```text
C:\ProgramData\Mad Library Server\
|- data\config\runtime.json     服务端口、数据库随机密码和 JWT 密钥
|- data\config\backup.json      自动备份设置
|- data\postgresql\             PostgreSQL 数据簇
|- data\storage\                默认文件储存目录
|- data\backups\                PostgreSQL 自定义格式备份文件
|- data\logs\postgresql.log     PostgreSQL 日志
`- logs\server.log              桌面壳启动的服务端日志
```

这是机器级数据目录，因此桌面壳和服务核心都通过实例锁保证同一数据目录只有一个服务实例。卸载或升级程序时不得自动删除该目录。

开发版由 `MADLIBRARY_HOME` 决定数据目录；桌面开发脚本使用 `target/desktop-runtime` 作为程序运行时，但持久数据边界仍由服务配置决定。排障时先看实际进程环境和日志，不要把 `target/` 当作发布物。

## 6. PostgreSQL 模式

`MADLIBRARY_POSTGRES_MODE` 支持：

- `auto`：默认模式；设置了 `MADLIBRARY_DATABASE_URL` 时使用外部数据库，否则启动随程序附带的 PostgreSQL。
- `bundled`：必须启动附带的 PostgreSQL。
- `external`：不启动附带数据库，必须提供 `MADLIBRARY_DATABASE_URL`。

`MADLIBRARY_SERVER_PORT` 可临时覆盖端口。安装版在设置页修改端口后写入 `data/config/runtime.json`，重启生效。局域网部署还需在 Windows 私有/域网络防火墙中放行选定 HTTP 端口，绝不能放行 `54329`。

## 7. 当前业务约束

- 用户只能看到自己被加入的资源库；服务端角色与资源库角色分开保存。
- 资源库可关闭。关闭后客户端 API 统一返回 `library_disabled`，客户端据此进入暂停页面并轮询恢复状态。
- 每个资源库绑定一个最终储存目录。不同资源库的最终目录不得相同，也不得互为父子目录。
- 空资源库可以直接更换储存；已有资源引用储存后，不应直接换绑，未来需走复制、校验、切换和回滚的迁移流程。
- 资源库删除仅管理员可执行，并可选择同时删除文件。删除文件属于高风险路径，必须继续保持权限与路径边界检查。
- 数据页支持每日自动备份、手动备份、下载、删除、恢复和服务器初始化。
- 自动备份默认每天 `02:00` 执行并保留 30 份，可配置范围为 1 至 365。
- 恢复前和初始化前会创建安全备份。恢复通过重启时应用待恢复记录完成。
- 只有 Owner 可以初始化服务器；初始化会清空业务数据并回到首次欢迎流程。

## 8. 管理后台结构

`admin-ui/src/App.tsx` 负责鉴权、初始化流程、路由状态和全局数据刷新。正式后台公共框架位于：

- `components/admin-shell.tsx`：侧栏、标题区、内容区和退出入口
- `components/desktop-titlebar.tsx`：Tauri 自定义标题栏、前进后退、折叠和窗口按钮
- `pages/`：资源库、用户、储存、统计、数据和设置
- `components/dialogs/`：编辑、删除、恢复、初始化和关机确认
- `api/`：请求、类型和端点边界

路由使用浏览器 History API。页面切换会调用数据刷新函数，但不会强制整页重载；点击当前侧栏项目也会刷新当前数据。新增页面应复用现有页面标题和 `Panel`/卡片样式，不要另建一套页面壳。

## 9. 构建与发布

构建 Windows 安装版：

```powershell
npm run release:windows
```

最终只从以下目录取发布物：

```text
artifacts/windows-x64/
|- Mad-Library-Server_0.1.0_windows-x64-setup.exe
`- SHA256SUMS.txt
```

NSIS 配置为 `installMode: both`，允许用户选择仅当前用户或所有用户安装。发布前必须确认 `pg_dump.exe`、`pg_restore.exe` 和 `dropdb.exe` 存在于精简 PostgreSQL `bin/` 中，否则备份、恢复或初始化流程不完整。

无界面服务器仍可使用 `scripts/build-windows-portable.ps1` 构建便携包。未来公网部署建议使用 Linux OCI 容器、外部 PostgreSQL、对象储存和反向代理 TLS；Docker 不是 Windows 桌面安装版的运行依赖。

## 10. 提交前验证

```powershell
cd .\admin-ui
npm run build
cd ..
cargo test --locked
cargo fmt --check
$env:CARGO_TARGET_DIR = (Join-Path (Get-Location) 'target\desktop')
cargo check --locked --manifest-path .\desktop\Cargo.toml
git diff --check
```

涉及桌面壳、端口、启动或关闭逻辑时，还应手工验证：单实例、浏览器访问、局域网地址、关闭窗口后子进程退出、端口释放、重启后新端口生效。

## 11. 已知风险与后续工作

- 发行版尚需代码签名、升级策略和第三方许可证审计。
- 商业席位授权、邀请制账号生命周期和审计事件仍需完成。
- 储存迁移、S3 兼容对象储存与客户端同步目录尚未实现。
- 公网部署清单、TLS、反向代理和外部数据库运维仍需补齐。
- README 中部分“首版范围”描述可能落后于当前备份功能；修改功能时应同步 README、路线图和本文档。
- 桌面壳关闭应优先调用受控制令牌保护的优雅关闭接口；若再次出现 PostgreSQL 残留进程，先修复进程生命周期，不要用全局进程清理掩盖问题。
