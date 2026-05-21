---
doc_type: feature-brainstorm
feature: 2026-05-20-pdf-sign
status: confirmed
summary: 批量给 ESI 月报 PDF 在固定位置贴签名图,Rust 核心库 + Tauri GUI 分层架构
tags: [pdf, signature, rust, tauri, xfa, overlay, batch]
---

# PDF 批量签名 Brainstorm

> Stage 0 | 2026-05-20 | 下一步：design

## 想做什么、为什么

ESI 服务工程师每月要在 H5P9 月报 PDF 上签名提交。手动用 Adobe Acrobat 一份份贴图低效,需要一个内部工具批处理:多选 PDF + 选签名图(记住上次)+ 一键签所有,输出新 PDF。

后期作为 Tauri 桌面应用分发到团队成员机器,所以从架构上分两层:Rust 核心 PDF 签名 library + Tauri GUI 薄包装。

**关键转折**:

1. 起点问"做哪种签名"——从场景判定是**视觉签章**(贴图)而非 PKI 数字签名 / eIDAS 合规
2. spike 发现 PDF 是 XFA 表单格式(`xfa? True`, format=PDF 1.6),但 `is_form_pdf=False` 即无可填字段 widget。绕过办法:**不动 XFA 表单本身,直接在页面内容流叠图**——任何 PDF 库都能做,跨工具可见性更好
3. spike 确认 "ESI Engineer's Signature" 是真实文本(Page 2, x=[466.3, 573.0], y=[502.2, 514.6]),理论上可以文本搜索定位,但因为 PDF 模板固定,**直接走配置文件坐标更省心**——少一个失败点,Rust 也不需要做 PDF 文本提取
4. 用户原图里那种"带边框空白方框"在实际 PDF 中**不存在**——PDF 底部只有标签和日期文字,签名图直接贴在 Date 值和 Signature 标签之间的空白区即可

## 考虑过的方向

### 方向 A:Python 脚本 (pymupdf / pypdf + reportlab)

- 描述:Python 命令行脚本,批量处理 PDF
- 价值:生态成熟,30 行代码搞定
- 代价:不能直接发给团队成员用(还要装 Python 环境),跟后期 Tauri GUI 计划脱节
- 结论:**否决**——只适合本地 spike 验证,不适合产品化

### 方向 B:Rust 核心 + Tauri GUI(选定)

- 描述:Rust crate 实现 PDF 签名核心逻辑,Tauri 包装成跨平台桌面应用
- 价值:跨平台分发友好(Tauri 打包 .dmg / .msi / AppImage),无运行时依赖
- 代价:Rust PDF 生态比 Python 弱,要自己处理一些底层操作
- 结论:**选定**——契合用户长期规划

### 方向 C:Rust + PDFium / MuPDF 重型绑定

- 描述:用 `pdfium-render` 或 `mupdf-rs`
- 价值:功能全(能搜文本、渲染预览、加注解)
- 代价:需要分发动态库(PDFium 每平台一份);MuPDF 是 AGPL
- 结论:**否决**——固定位置场景用不到这些重型功能,跨平台打包麻烦

## 已敲定的设计点

| 设计点 | 决定 | 状态 |
|---|---|---|
| 项目语言 | Rust(核心)+ Tauri(GUI) | 已确认 |
| Rust PDF 库 | `lopdf`(纯 Rust,MIT,跨平台编译零依赖) | 已确认 |
| 位置定位策略 | 固定坐标走配置文件(非文本搜索) | 已确认 |
| 不动 XFA 表单 | 直接在 page content stream 叠图绘制命令 | 已确认 |
| 签名坐标(初值) | Page 2, x=466~573, y=460~500(PDF 坐标,左下原点) | 倾向(待 design 锁定) |
| 签名图格式 | PNG 透明背景 | 已确认 |
| 输入交互 | 多选 PDF + 选签名图(记住上次)+ 一键签 | 已确认 |
| 状态持久化 | Tauri 本地存储(`tauri-plugin-store` 或本地 JSON) | 倾向 |
| 许可证约束 | 团队内部使用,无商用约束 | 已确认 |

## spike 验证记录

- `spike-page2-preview.png` —— Page 2 渲染图,叠了候选签名框
  - 蓝实线:`ESI Engineer's Signature` 标签实际位置
  - 红实线:**候选 A(选定,与标签等宽,高 60)**
  - 绿虚线:候选 B(更宽,会覆盖 Date 值)——已否决
- 文本块布局结论(y=420~540):
  - y=420  说明文字(横跨两栏)
  - y=460  Date 值 "May 20, 2026"(左 x=305,右 x=683)
  - y=502  四个底部标签(Customer Sig / Date / ESI Eng Sig / Date)
- 签名图宽高比验证:用户给的"张项"签名约 350:130 = 2.7,按 107 pt 宽算高度 ≈ 40 pt,正好塞进 y=460~500

## 选定方向与遗留问题

**选定方向**:做一个 Rust 核心 PDF 签名 library,功能是"给指定 PDF 在配置坐标上叠一张 PNG 签名图,输出新 PDF",支持批处理。后续用 Tauri 包装成 GUI 应用,UI 只有"选 PDF / 选签名 / 签名"三步,选过的签名图路径记住下次默认。

**留给 design 的问题**:

1. **输出文件命名**:覆盖原文件 / 同目录加 `_signed` 后缀 / 让用户选输出目录
2. **签名图缩放策略**:按签名框 bbox 等比自适应 / 固定 pt 尺寸 / 用户可配
3. **异常 PDF 处理**:非 H5P9 模板 / 损坏 / 加密 —— 跳过 / 报错 / 强制贴
4. **Tauri 前端栈**:React / Vue / Svelte / vanilla(放最后定)
5. **坐标常量精确值**:用户确认 spike 预览图后,把红框坐标和缩放比例锁进 design 文档常量
6. **crate 与 Tauri 项目结构**:单 crate(lib + bin)/ workspace(core + tauri-app 两 crate)
