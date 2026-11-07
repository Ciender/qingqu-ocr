<p align="center">
  <img src="assets/app.svg" width="72" height="72" alt="轻取 OCR 图标">
</p>

<h1 align="center">轻取 OCR</h1>

<p align="center">截图取字 · 双栏翻译 · 智能排版 · 图片悬浮</p>

<p align="center">
  <img src="https://img.shields.io/badge/Windows-10%20%2F%2011%20x64-0078D4" alt="Windows 10 / 11 x64">
  <img src="https://img.shields.io/badge/version-0.3.0-2563EB" alt="版本 0.3.0">
  <img src="https://img.shields.io/badge/Rust-Win32-DEA584" alt="Rust + Win32">
  <img src="https://img.shields.io/badge/license-Noncommercial-64748B" alt="非商业许可">
</p>

<p align="center">
  <a href="README.en.md">English</a> ·
  <a href="#开始使用">开始使用</a> ·
  <a href="#界面预览">界面预览</a> ·
  <a href="#从源码构建">构建</a> ·
  <a href="#许可证">许可证</a>
</p>

轻取 OCR 是一款使用 **Rust + Win32** 开发的 Windows 桌面文字识别工具：从屏幕提取文字，在同一工作区完成编辑、翻译、排版、公式预览和表格复制，也可以将剪贴板内容固定为桌面便签或图片浮窗。

**OCR 通过你配置的云服务完成，需要网络及对应 API 权限。** 原创代码采用非商业许可。

## 主要功能

- **截图识别**：文字、表格、竖排、公式。
- **双栏翻译**：多服务、自动翻译、流式输出。
- **文本整理**：编辑、查找、分段、排版、简繁与标点转换。
- **文档处理**：公式 / Markdown 预览，表格复制到 Word / Excel。
- **剪贴板窗口**：文字便签、图片浮窗、独立 OCR。
- **桌面集成**：托盘、热键、悬浮条与浅深主题。

## 界面预览

<table>
  <tr>
    <td align="center"><a href="docs/screenshots/workspace.png"><img src="docs/screenshots/workspace.png" width="320" alt="原文与结果双栏工作区"></a><br><strong>双栏工作区</strong></td>
    <td align="center"><a href="docs/screenshots/formula-preview.png"><img src="docs/screenshots/formula-preview.png" width="320" alt="公式源码与原生预览"></a><br><strong>公式预览</strong></td>
  </tr>
  <tr>
    <td align="center"><a href="docs/screenshots/tables.png"><img src="docs/screenshots/tables.png" width="320" alt="表格原文与结构预览"></a><br><strong>表格识别</strong></td>
    <td align="center"><a href="docs/screenshots/task-bindings.png"><img src="docs/screenshots/task-bindings.png" width="320" alt="功能、API 服务与接口绑定"></a><br><strong>调用配置</strong></td>
  </tr>
  <tr>
    <td align="center"><a href="docs/screenshots/dark-theme.png"><img src="docs/screenshots/dark-theme.png" width="320" alt="深色主题下的公式预览"></a><br><strong>深色主题</strong></td>
    <td align="center"><a href="docs/screenshots/clipboard-windows.png"><img src="docs/screenshots/clipboard-windows.png" width="320" alt="文字便签、图片浮窗及其右键菜单"></a><br><strong>文字便签与图片浮窗</strong></td>
  </tr>
</table>

## 开始使用

1. 解压 `qingqu-ocr-0.3.0-windows-x64.zip`，运行 `qingqu-ocr.exe`。更新前先从托盘彻底退出旧版。
2. 打开 **设置 → API 服务**，填写你要使用的服务凭据，以及所需的地域、模型或地址。
3. 打开 **设置 → 调用配置**，按“功能 → API 服务 → 具体接口”选择各任务的绑定，保存并应用。
4. 按 **`Alt+Q`** 截图识别，或按 **`Alt+V`** 打开剪贴板文字 / 图片。

图片识别可以跟随文字配置，也可以独立设置。云服务可能收费，程序不提供 API Key；可用接口、语言及账户权限由服务商决定。

### 支持的服务

<table>
  <thead><tr><th>类别</th><th>服务</th><th>支持内容</th></tr></thead>
  <tbody>
    <tr><th colspan="3" align="left">OCR 识别</th></tr>
    <tr><td rowspan="6">OCR</td><td>百度 OCR</td><td>文字、表格、竖排、公式及文档分析</td></tr>
    <tr><td>腾讯云 OCR</td><td>文字、表格、竖排</td></tr>
    <tr><td>阿里云 OCR 独立接口</td><td>通用文字、高精度文档、表格、公式</td></tr>
    <tr><td>阿里云 OCR 统一识别</td><td>文字、手写、多语言、表格</td></tr>
    <tr><td>百炼 Qwen-OCR / Qwen-VL</td><td>按模型能力提供文字、表格、竖排与公式识别</td></tr>
    <tr><td>Mathpix</td><td>公式识别</td></tr>
    <tr><th colspan="3" align="left">文字翻译</th></tr>
    <tr><td rowspan="7">翻译</td><td>Google</td><td>网页服务 / 官方 API</td></tr>
    <tr><td>Microsoft</td><td>网页服务 / 官方 API</td></tr>
    <tr><td>Index Translate</td><td>独立地址、模型与密钥配置</td></tr>
    <tr><td>OpenAI 兼容</td><td>自定义兼容地址、模型及流式输出</td></tr>
    <tr><td>百度翻译</td><td>官方翻译 API</td></tr>
    <tr><td>阿里云翻译</td><td>官方翻译 API</td></tr>
    <tr><td>腾讯翻译</td><td>官方翻译 API</td></tr>
  </tbody>
</table>

各服务的接口、配置和额度说明，可点击软件中的 **“？”按钮** 查看。

### 默认快捷键

| 快捷键 | 操作 |
|---|---|
| `Alt+Q` | 截图识别 |
| `Alt+V` | 打开当前剪贴板文字 / 图片 |

其他功能可在设置中按需分配快捷键。

<details>
<summary><strong>排版、表格和图片的使用说明</strong></summary>

- **段落处理：** 自动分段结合阅读顺序、对齐、行距和可靠的接口信息；拆分保留原始 OCR 行，合并去掉全部换行。智能排版独立处理当前文字或选区，不改变后续识别的默认模式。外部粘贴文字可一键合并为一行，再用段落按钮恢复或整理。
- **文本保护：** 智能排版保护代码、公式、网址等结构；首行缩进、段间空行由设置控制。手工编辑后不使用旧识别内容覆盖当前文字。
- **表格复制：** 支持整表与选区，提供 RTF、Excel XML Spreadsheet 和 TSV，保留单元格换行、合并跨度、前导零及长编号。表格结构依赖云端返回结果，错字、漏格和错误合并仍需核对。
- **公式 / Markdown：** 支持常见分数、根式、上下标、标题、列表和代码。数学分隔符包括 `$...$`、`$$...$$`、`\(...\)`、`\[...\]`。HTML 按文字显示，不执行脚本或加载网络图片；复杂 TeX 宏不保证兼容。超过 256 KiB 或 8192 个格式片段时回退源码。
- **图片浮窗：** 支持 1–800% 手动比例、平滑滚轮缩放、左右旋转、适应屏幕、置顶、信息栏和主动识别。显示缩放不改变原图；识别使用完整原图并保留旋转方向，复制不包含边框或信息栏。
- **图片复制：** 同时提供无损 PNG、DIBV5、文件列表和路径文字，目标软件选择其支持的格式。纯文字目标可能收到 PNG 文件路径。
- **会话：** 收进托盘后保留内容与撤销记录；彻底退出后结束本次会话。相同图片可共享原图，各窗口分别记录旋转状态。

</details>

## 数据与隐私

OCR 图片和待翻译文字发送到你选择的服务；开启自动翻译后，识别结果会继续提交给翻译服务。**`Alt+V` 打开便签或图片本身不上传内容。**

偏好设置保存在 `%LOCALAPPDATA%\QingquOCR\settings.json`，凭据由 Windows DPAPI 加密，保存在同目录的 `credentials.bin`。可通过 `QINGQU_DATA_DIR` 指定独立数据目录。密钥可在配置页显示或隐藏，请勿分享含密钥的截图或凭据文件。

<details>
<summary><strong>本地图片和临时文件</strong></summary>

原图与撤销记录使用会话临时文件；原图在最后一个窗口或任务释放引用后清理。为使复制出的文件路径在退出后仍可用，图片复制会在系统临时目录 `QingquOCR-clipboard` 保留 PNG。后续成功复制时清理 7 天前的旧文件，并以 128 张 / 128 MiB 为缓存目标；当前或被占用文件可能暂时超限。复制过的敏感图片可能在退出后继续保留。

</details>

## 轻量化设计

本节面向实现与维护人员，按**资源模型、数据生命周期、渲染路径、调度策略和测量边界**说明当前设计。这里的“轻量化”指减少重复数据、约束可积累资源并缩短临时分配的生命周期；不把降低任务管理器中的单个数字作为优化目标。

### 1. 资源模型与评价口径

应用将资源分为四类：常驻的窗口与工作线程；由文本长度决定的编辑缓冲和当前历史状态；由唯一图片决定的文件映射；由活动视口和任务决定的临时帧、编码缓冲及网络数据。它们的增长变量不同，不能用一个“缓存上限”概括整个进程。

对一张 `W × H` 的 RGBA 原图，像素存储为 `4WH` 字节；对 `Vw × Vh` 的显示表面，一张 32 位 DIB 约需 `4VwVh` 字节。3840 × 2160 的单份像素约为 **31.64 MiB**，因此图片交互峰值通常由整帧和系统合成表面决定，而非几百 KiB 的滤波块。

| 资源 | 当前实现中的边界 | 不包含的部分 |
|---|---|---|
| 静止图片显示缓存 | UI 线程共享 4 MiB，按字节预算淘汰 | 原图映射、动画帧、Windows 合成表面 |
| 滤波输出块 | 至多 1024 × 64 × 4，即 256 KiB | 输入块、卷积支持区和库内部工作区 |
| 等待任务 | 每条工作通道保留 1 个待执行任务 | 当前执行任务、结果通知队列、任务载荷大小 |
| 撤销日志 | 差量内容和链式索引写入会话文件 | 当前文本、编辑器缓冲、操作分组快照 |
| 文档预览输入 | 源码至多 256 KiB；格式计划至多 8192 个片段 | 系统控件、解析过程和格式应用的实际分配 |

**4 MiB 是一个缓存策略参数，不是程序总内存承诺。** 文件映射页面、图形资源和系统共享页面还会影响工作集；Private Working Set、Private Bytes 与 Working Set 也不等价。

### 2. 界面职责划分与重复状态消除

纯文本交给静态链接的 Scintilla，行号使用控件内部 margin，与正文共用布局、折行和滚动状态；应用只在计算后的边栏宽度发生变化时更新 margin，避免维护另一套行号绘制与滚动同步逻辑。应用侧适配层处理 UTF-8 字节位置与 Windows UTF-16 选区之间的转换，不把两种位置单位混用。[实现：Scintilla 适配](src/native/scintilla.rs)、[编辑视图](src/native/editor_view.rs)。

公式和 Markdown 预览由 `DocumentView` 延迟创建，使用系统 Rich Edit / TOM2。重建时先生成格式计划，再暂停控件重绘、批量应用格式，最后统一恢复绘制；预览控件关闭内部撤销，复制操作保留独立的源码。这样避免为只读预览维护编辑历史，但预览创建后仍可能保留控件与 COM 状态，隐藏不等于卸载系统模块。[实现：文档预览](src/native/rendered.rs)。

### 3. 原图去重、映射与所有权

`StoredImage::intern` 以 `(width, height, MD5)` 定位候选，再逐字节比较像素确认相同内容，哈希只用于索引。索引保存 `Weak<StoredImage>`，窗口和任务持有 `Arc<StoredImage>`，因此索引本身不会延长原图生命周期。`ImageView` 仅保存共享原图、旋转次数与来源大小；多个窗口旋转时修改视图元数据，不各自持有一张旋转后的整图。

首次入库将 RGBA 写入带 `FILE_FLAG_DELETE_ON_CLOSE` 的会话文件，通过 `CreateFileMappingW(PAGE_READONLY)` 创建映射；读取时使用 `MapViewOfFile(FILE_MAP_READ)`。短期 `Pixels` 在析构时解除映射，连续缩放使用 `MappedPixels` 保留映射并持有原图强引用，减少逐帧映射与页面重新调入。最后一个强引用释放后，映射句柄和文件句柄依次回收。窗口缓存另以弱引用关联原图，最后一个图片窗口关闭时清理其显示资源，OCR 任务仍可继续持有原图。[实现：原图存储](src/image_store.rs)、[会话文件](src/session_file.rs)。

这不是全路径零拷贝：首次入库前仍有解码后的 `RgbaImage`；去重需要哈希和像素比较；旋转后复制、OCR 输入物化仍可能创建整图。文件映射把重复的长期堆副本转为共享、按需驻留的文件页面，也引入临时磁盘 I/O 和缺页成本。

### 4. 撤销日志的差量表示

撤销由应用侧 `Journal` 维护，关闭编辑器内部撤销，避免保存两份历史。对于相邻文本 `A`、`B`，算法寻找公共前缀和公共后缀，并将边界退回到合法 UTF-8 字符边界，只持久化中间被删除和插入的字节。每条记录包含 **80 字节头部**，保存前后链指针、差量位置、长度、选区、粘贴状态和段落来源偏移；记录空间近似为 `80 + removed_bytes + inserted_bytes`。

链式索引也放在日志文件中，内存不按撤销步数保存完整快照或索引节点。流式翻译和段落变换使用操作分组，组内更新当前文本，组结束时提交一个差量，保持一次操作对应一次撤销。普通键盘消息先检查修改标志，只有确实变更文本才读取全文并比较。[实现：会话撤销日志](src/native/history.rs)。

该方案降低的是历史链的常驻占用，仍保留编辑器文本、`Journal.current` 和必要的分组快照。一次实际变更最坏仍需 `O(N)` 全文读取与前后缀扫描，日志 I/O 也发生在 UI 消息路径；新分支使旧 redo 记录不可达时，文件暂不在线压缩。这些属于长文本响应与临时磁盘增长方面的技术债。

### 5. 分块重采样与显示缓存

图片渲染适配层调用 `fast_image_resize 6.1.0` 的运行时 CPU 指令集分派和 `Convolution(Bilinear)`，不启用 Rayon。缩小时滤波支持区随比例扩大，放大和缩小保留同一像素中心变换；窗口位置取整后的余量写入 `phase`，继续参与小数取样，避免每帧只根据整数几何重建坐标导致纹理跳动。

输出分块尺寸由 `columns = clamp(floor(512 × zoom), 1, 1024)`、`rows = clamp(floor(512 × zoom), 1, 64)` 计算。未旋转且完全位于原图内部的块直接借用源行；边界扩展和旋转路径只物化当前输入块。`render_strided` 将块结果写入带行跨度的 DIB，省去另一份全尺寸缩放图及其后的整图复制。256 KiB 仅是输出块上界；支持区会随缩小比例变化，不能据此声称整个滤波器只占 256 KiB。[实现：渲染适配](src/native/picture/resample.rs)、[分块卷积](vendor/picture-renderer/src/lib.rs)。

静止帧使用 `VecDeque` 管理近似 LRU：缓存键包含原图身份、旋转、视口、比例的浮点位模式、平移、小数相位和背景；命中后移到队尾，超预算时从队首淘汰。单帧超过 4 MiB 时绕过缓存，动画中间帧也不挤占静止图片缓存。连续交互复用映射、DIB 和卷积工作区，停止后释放临时对象。[实现：图片窗口](src/native/picture.rs)。

### 6. 缩放控制、稳定画布与一次提交

滚轮输入更新目标比例，而不是为每个输入包排队生成一帧：`target *= 1.1^(delta/120)`。动画以真实经过时间在对数比例空间收敛，更新式为 `z_next = z × exp(ln(target/z) × (1 - exp(-dt/0.060)))`；误差足够小或距最后输入达到 240 ms 时结束。原图锚点在同一指针位置的连续输入间保持，定时器仅在动画期间启用。若绘制耗时超过约 1/60 秒，重新安排计时器，给输入处理留出间隔；这不构成全场景 60 FPS 保证。

每帧先完成 DIB 绘制，再用 `UpdateLayeredWindow` 一次提交位置、尺寸和像素，避免“新窗口几何配旧画面”的中间状态。`PreparedFrame` 通过作用域受控的窗口属性暴露不可变完整帧，使同步重入的绘制消息不必再次借用正在更新的 `Picture` 状态；作用域退出时移除属性。上传失败则撤销预留画布并退回完整 GDI 路径。

外框随比例变化时，逻辑可见区域与底层表面分离：按 `1.25^k` 比例桶预测扩容，按 **256 px 网格**预留透明画布，图像、1 px 描边、信息栏和 16 px 阴影绘入同一表面。透明区域保留穿透命中，停止或取消时收紧；已达到固定视口的路径不需要同样的扩容。历史包络只有在不超过 32 MiB、且不超过当前预留区域额外 4 MiB 时才继续复用，**该条件不限制新预留区域或所有窗口的总分配**。[实现：帧提交与画布](src/native/picture.rs)、[阴影合成](src/native/picture_shadow.rs)。

稳定画布减少频繁表面重建和几何错位，但增加透明区清除、alpha 补齐和分层上传开销。大视口仍需整帧 DIB，Windows 还可能保留自己的合成表面；分块滤波不会消除这些成本。

### 7. 有界待执行任务与结果时序

后台执行分为本地操作、OCR、翻译三条固定通道，各由一个工作线程消费 `Mutex<(closed, Option<Job>)> + Condvar` 邮箱。每条通道最多保留一个等待任务；新任务替换旧等待项时，标记取消并释放旧输入。由此约束的是**待执行任务数量**，不包括正在执行的三个任务、每个载荷字节数和 `mpsc` 结果通知队列。

UI 通过递增任务 ID 和 `Arc<AtomicBool>` 管理过期结果。手动编辑或开始新任务后，旧任务即使完成也不能继续覆盖当前文本。流式译文使用共享的 `Option<(id, String)>` 最新值槽位，UI 取走最近的完整状态，避免把每次累积文本都排进结果队列；但每次进度仍会触发唤醒，字符串构造也仍有长度相关成本。[实现：工作通道](src/native/backend.rs)、[结果接收](src/native/main_window/jobs.rs)。

取消属于协作式取消。已进入的阻塞网络请求不会仅因标志置位就终止，仍可能占用该通道直到响应或超时。空闲线程在条件变量上等待，60 秒超时用于释放百度 HTTP 客户端；鉴权令牌按密钥匹配和有效期独立保存。它减少闲置连接保留，但超时唤醒仍存在，不能描述为完全无唤醒。[实现：鉴权与连接复用](src/ocr.rs)。

### 8. 截图、上传与剪贴板导出的内存边界

截图像素分配使用 `checked_mul`、`isize::MAX` 检查和 `try_reserve_exact`，保留显示器物理分辨率，由可用内存与系统分配结果决定是否继续。冻结截图阶段仍可能同时存在目标 RGBA 向量和 GDI 位图；区域裁剪、OCR 解码与网络包装也可能增加临时副本，不能把移除固定预算解释为零额外占用。[实现：屏幕捕获](src/platform.rs)。

OCR 准备与显示滤波分开：按接口策略约束尺寸、像素数与实际 PNG 字节；透明通道在原 RGBA 中合成到白底，使用 PNG Level 3 / Adaptive 编码，避免先复制为全图 RGB。`Limited` 写入器在字节预算耗尽时终止编码，再缩小重试；面积平均降采样直接产生目标图，不保留全宽浮点中间图。Base64、JSON 和表单转义仍有额外开销，PNG 预算不是完整请求峰值。[实现：上传预处理](src/ocr/input.rs)。

剪贴板 PNG 导出采用独立的磁盘缓存，重用文件前以 16 KiB 块比较内容，应用只保留最近一次导出的保护句柄。7 天、128 张、128 MiB 是后续成功复制时执行的清理策略，当前文件和被占用文件可暂时超过目标；该磁盘缓存与 4 MiB 显示缓存、随引用释放的原图会话文件不是同一资源。[实现：导出缓存](src/platform/clipboard_cache.rs)。

### 9. 构建配置与热点隔离

主程序 Release 使用 `opt-level = "s"`、Thin LTO、`codegen-units = 1`、符号裁剪和 `panic = "abort"`；`fast_image_resize` 与本地渲染适配单独设为 `opt-level = 3`，避免全程序以速度优先换取更大的指令体积。依赖仅启用实际使用的图片格式和协议功能，Scintilla 静态编译进 EXE。[配置：Cargo.toml](Cargo.toml)、[构建脚本](build.rs)。

这些配置是工程取舍：LTO 和单代码生成单元增加构建时间；静态 CRT 减少部署依赖，但不会降低每项运行时分配；`panic = "abort"` 避免展开成本，也意味着 panic 时不运行 Rust 析构。因此正常的 RAII 释放路径与异常终止路径需要分别评估，不能仅凭使用 Rust 推导所有异常场景都能完成应用层清理。

### 10. 当前测量结果与待偿还技术债

测量对象为 2026-11-08 的当前 Release，本机隔离桌面、空双栏主窗与悬浮条、独立配置，无 OCR、翻译或剪贴板写入，未裁剪工作集。两轮各约 3.002 秒的结果一致：

| 指标 | 观测值 | 解释 |
|---|---:|---|
| Working Set | 33.27 MiB | 包含当前驻留的私有和可共享页面 |
| Private Working Set | 5.01 MiB | 驻留且不可与其他进程共享的页面 |
| Private Bytes | 6.54 MiB | 进程私有提交，不要求全部驻留 |
| 空闲 CPU | 0.0% | 进程 CPU 时间差 / 墙钟时间，按一个逻辑核计 |
| GDI / USER 对象 | 97 / 87 | 两轮均相同；短采样不能证明长时间无泄漏 |

两个短窗口中的 0.0% 只表示该精度下未观察到 CPU 时间增长，不能证明绝对零唤醒。上述结果不是含图片、首次打开文档或真实云端请求时的峰值，也不足以单独证明某项优化的收益；优化归因需要同输入、同构建条件的前后对照。

当前技术债与验证方向包括：

1. **去重临界区与整图成本。** 哈希前仍有完整输入；候选像素比较和首次写盘处在全局索引锁内。大量图片同时入库时，需要评估锁占用，并将可移出的 I/O 与候选比较拆开；验收应覆盖同图命中、不同图并发及峰值提交。
2. **长文本增量记录。** 将 Scintilla 修改通知转换为精确插入 / 删除差量，可减少目前每次变更的全文读取与扫描；需要保持 UTF-8 / UTF-16 选区、IME、分组撤销和段落来源的一致性，并补充 redo 分支的日志回收策略。
3. **UI 线程上的整帧工作。** SIMD 和缓冲复用仍未把图片卷积移出 UI 线程，分层表面上传也可能随视口面积增长。后续应分别记录输入到提交的 p50 / p95 / p99、滤波耗时、上传耗时与活动峰值，而不是只报平均帧率。
4. **取消、队列和瞬时资源的统一预算。** 单槽邮箱只限制排队数量；阻塞请求、无界通知通道和多个活动图片仍可能保留大载荷。可进一步引入可中断请求、通知背压及活动任务按字节计的预算，并核验旧结果不覆盖新编辑。
5. **Win32 / FFI 生命周期审计。** 映射视图、DIB/DC、窗口属性中的临时指针和同步重入依赖明确的所有权不变量。需要持续检查窗口销毁、分配失败和回退路径，验证句柄恢复到预热基线；文件映射与系统合成资源也应纳入测量。

## 从源码构建

### 构建环境

| 项目 | 要求 |
|---|---|
| Rust / Cargo | **1.97.1 或更新的稳定版**；主程序和本地渲染模块均在 `Cargo.toml` 声明 `rust-version = "1.97.1"` |
| 目标平台 | `x86_64-pc-windows-msvc`，Rust 2021 Edition |
| C++ 工具链 | Visual Studio 2022 Build Tools 或更新版本，安装“使用 C++ 的桌面开发”，包含 MSVC x64/x86 工具和 C++17 支持 |
| Windows SDK | Windows 10 / 11 SDK；当前构建使用 **10.0.26100.0** |
| 发布打包 | PowerShell **7.0+**；直接构建 EXE 不需要额外打包工具 |

Rust 1.97.1 是本项目当前声明并验证的最低受支持版本。MSVC 环境安装方式见 [rustup 官方说明](https://rust-lang.github.io/rustup/installation/windows-msvc.html)。

### 安装工具链与获取依赖

安装 [rustup](https://rustup.rs/) 和上述 C++ 工具后，在仓库根目录执行：

```powershell
rustup toolchain install stable-x86_64-pc-windows-msvc --profile minimal --component rustfmt --component clippy
rustc +stable-x86_64-pc-windows-msvc --version
cargo +stable-x86_64-pc-windows-msvc fetch --locked --target x86_64-pc-windows-msvc
```

确认 Rust 版本不低于 1.97.1。首次获取第三方 Rust 库需要网络；具体版本由 [Cargo.lock](Cargo.lock) 锁定，运行依赖和构建依赖均在 [Cargo.toml](Cargo.toml) 声明。无需手动逐个安装 Rust 库。

### 编译与运行

在 **Developer PowerShell for Visual Studio** 或已配置 MSVC 环境的 PowerShell 中执行：

```powershell
cargo +stable-x86_64-pc-windows-msvc build --release --locked
.\target\release\qingqu-ocr-native.exe
```

产物为 `target/release/qingqu-ocr-native.exe`，发布包内统一命名为 `qingqu-ocr.exe`。`.cargo/config.toml` 启用静态 CRT；Scintilla 由 C++17 编译器静态编译进程序。依赖缓存齐全后，可给构建命令添加 `--offline` 离线编译。

### 检查与打包

```powershell
cargo +stable-x86_64-pc-windows-msvc fmt --all -- --check
cargo +stable-x86_64-pc-windows-msvc check --locked
cargo +stable-x86_64-pc-windows-msvc clippy --locked -- -D warnings
pwsh -File scripts/package.ps1
```

打包脚本读取构建产物和锁定依赖，检查第三方许可后，在 `dist/` 生成 Windows x64 ZIP 及 SHA-256 文件；包含 EXE、双语 README、示范图和许可文本。打包不会覆盖已有同名 ZIP。

<details>
<summary><strong>依赖与代码结构</strong></summary>

普通 Rust 库由 Cargo 获取；以下本地内容已随源码提供：

- `vendor/scintilla/`：Scintilla 5.6.9 的核心、头文件和 Win32 源码。
- `vendor/picture-renderer/`：本项目的 Rust 图片渲染适配模块。
- `vendor/text-layout/`：移植算法的来源和许可；Rust 实现在 `src/text/`。
- `assets/`：构建所需 SVG、主题、应用图标与 Windows manifest。

主要依赖及用途（版本对应当前锁文件）：

| 用途 | 依赖 |
|---|---|
| Windows 接口 | `windows 0.62.2`、`windows-sys 0.61.2` |
| 图片与剪贴板 | `image 0.25.10`、`fast_image_resize 6.1.0`、`arboard 3.7.0` |
| 网络与异步请求 | `reqwest 0.13.5`、`tokio 1.54.1`、`async-openai 0.42.3` |
| 文档解析 | `pulldown-cmark 0.13.6`、`quick-xml 0.41.0` |
| 排版与文字转换 | `ferrous-opencc 0.4.0`、`regex 1.13.1`、`textwrap 0.16.4`、`unicode-segmentation 1.13.3` |
| 配置与数据 | `serde 1.0.229`、`serde_json 1.0.154`、`secrecy 0.10.3`、`base64 0.22.1`、`md5 0.8.1`、`futures-util 0.3.34` |
| 构建资源 | `cc 1.10.0`、`resvg 0.49.0`、`winresource 0.1.34` |

完整传递依赖可通过 `cargo tree --locked` 查看。第三方库未全部复制进仓库；其版本、来源和适用许可分别见 Cargo 锁文件及 `licenses/`。

| 路径 | 内容 |
|---|---|
| `src/native/` | 主窗、便签、图片、设置与编辑器 |
| `src/config/`、`src/ocr/`、`src/ocr_cloud/` | 任务配置与识别服务适配 |
| `src/platform/`、`src/text/` | 截图、剪贴板、排版与转换 |
| `licenses/`、`docs/screenshots/` | 第三方声明与界面示范 |

</details>

## 反馈与贡献

报告问题时，请提供 Windows / 程序版本、DPI、服务与模型、复现步骤及预期结果，附不含敏感信息的最小样例。请勿提交 API Key、`credentials.bin` 或私人原始文档。提交代码前请阅读项目及第三方许可，并运行上述基础检查。

## 许可证

作者有权授权的原创代码、文档与原创图形采用 **[PolyForm Noncommercial 1.0.0](LICENSE)**，**不授予商业使用许可**。本项目提供源码，使用范围受该许可限制。

第三方组件继续适用原许可证，保留原有商用权利；项目的非商业条款不替换 MIT、Apache-2.0、BSD 等第三方许可。Umi-OCR / GapTree 派生部分保留 MIT 来源与版权声明。分发时请保留 [NOTICE.txt](NOTICE.txt)、[THIRD_PARTY_NOTICES.txt](THIRD_PARTY_NOTICES.txt) 及 [licenses/](licenses/) 中适用的声明。

感谢 Scintilla、Umi-OCR / GapTree、fast_image_resize、Fluent UI System Icons、VS Code、Catppuccin 及 Rust 生态的维护者。
