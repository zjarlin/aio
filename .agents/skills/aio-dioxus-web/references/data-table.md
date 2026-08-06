# DataTable 使用约定

## 所有权

- 组件入口：`app/plugins/studio/src/components/data_table/component.rs`
- 纯布局内核：`app/plugins/studio/src/components/data_table/layout.rs`
- 组件样式：`app/plugins/studio/src/components/data_table/style.css`
- 当前使用者：Studio REST 功能定义、发布后的 CRUD/TreeTable 运行时

DataTable 是上游 Dioxus Components 没有 Table 时的仓库级复合组件。它继续复用官方 Button、Input、Checkbox 等基础控件，不发展第二套基础设计系统。

## 核心 API

| API | 责任 |
| --- | --- |
| `DataTableColumn::leaf/group` | 声明叶子列与任意深度树形表头 |
| `width/align/fixed/editable` | 声明稳定尺寸、对齐、固定列和编辑能力 |
| `row_key` | 为每行提供稳定业务键 |
| `render_header` | 叶子表头插槽，例如排序按钮 |
| `render_cell` | 只负责业务单元格展示 |
| `can_edit` | 按行和列判定是否允许编辑 |
| `render_editor` | 返回官方 Input/Checkbox 等编辑控件 |
| `DataTableSpan` | 声明 rowspan/colspan；冲突或越界直接显示配置错误 |
| `right_panel` | 只承载非表单的辅助信息；窄视口自动移到表格下方 |
| `selected_row_key/on_row_select` | 行选择与右侧编辑区联动 |

## 约束

- 固定左列必须连续位于最左侧，固定右列必须连续位于最右侧。
- 跨列合并不能覆盖固定列；跨行合并允许覆盖普通列或单个固定列。
- 页面不直接拼接 sticky 偏移、rowspan 或树形表头行；全部交给布局内核。
- 可编辑单元格默认双击或按 Enter/F2 进入编辑；业务需要时显式改为 Click。
- 编辑器自行负责校验和持久化，完成后调用 `edit.close`；DataTable 不知道业务 API。
- 操作按钮放在固定右列并打开 Dialog；完整 Form 不得放进 `right_panel` 或常驻页面。
- 表格外层不得产生页面级横向滚动；横向滚动所有权属于 DataTable viewport。

## 验证

至少覆盖：

- 树形表头生成的 rowspan/colspan 正确。
- 左右固定列偏移正确，滚动后仍可见。
- 合并矩阵覆盖单元格并拒绝冲突、越界和跨固定列合并。
- 点击、双击、Enter/F2 与 Escape 编辑行为符合配置。
- 右侧面板在宽屏固定，窄屏移到表格下方。
- 浏览器控制台无错误，页面根节点无横向溢出。
