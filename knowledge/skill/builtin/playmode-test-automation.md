---
id: kd_skill_builtin_playmode_test_automation
type: skill
path: builtin/playmode-test-automation.md
title: PlayMode Test Automation
injectMode: none
summaryEnabled: true
commandEnabled: true
readOnly: false
aiMaintained: false
skillEnabled: true
skillSurface: command
commandTrigger: /playmode-test-automation
argumentHint: <task or test group description>
createdAt: 1778303603129
updatedAt: 1778303603130
---

# playmode-test-automation

## Summary
为 Unity 项目读取、扩展和自动运行 PlayMode 集成测试，并稳定落盘结果与诊断失败。

## Content
## When to use

- 需要为 Unity 项目读取、理解、扩展或新建 PlayMode 集成测试。
- 需要自动运行现有 Test Runner 用例，而不是只手动点击 Unity Test Runner。
- 需要把 PlayMode 跑测结果稳定落盘，避免 PlayMode / 域重载 / 外部连接中断导致结果丢失。
- 需要对项目核心业务流做 P0/P1/P2 分层集成保障。

## When NOT to use

- 任务只要求纯 EditMode 单元测试，且不涉及真实场景、UI、流程、实体或运行态交互。
- 任务只是修单个测试断言，不需要建立通用跑测流程。
- 任务只需临时手动跑一次测试，不需要可复用的自动化与结果落盘能力。
- 请求是性能分析、Profiler 抓取或运行时调试，应改用对应 Skill。

## Instructions

1. 先做项目研究，再决定是“读已有测试并复用”还是“补建测试”。
   - 先查项目知识库里是否已有主流程、UI、Procedure/FSM、场景切换、实体生成、资源更新相关记录。
   - 再读真实代码与必要的 Unity 资产结构，确认启动场景、主流程入口、UI 打开方式、主玩法入口、关键实体生成链路。
   - 先找现有测试：测试程序集、`Assets/Tests`、`asmdef`、Editor 桥接器、已有分组（如 `P0/P1/P2`）。
   - 不要没看现状就重写测试基础设施。

2. 优先复用已有测试与现有分组。
   - 如果项目已经有 PlayMode 集成测试，先读测试代码，确认：覆盖目标、分组方式、启动辅助方法、反射工具、等待工具、结果文件位置。
   - 先跑已有用例验证现状，再决定补测试、修测试还是修项目代码。
   - 若已有分层分组，优先沿用；常见分法：
     - `P0`：启动、菜单、主流程、死亡回退等核心闭环
     - `P1`：设置、资源流程、Procedure/FSM 关键分支
     - `P2`：补强覆盖，如数据映射、输入边界、特殊行为

3. 若项目还没有 PlayMode 集成测试，按最小闭环先落 P0。
   - 先保证可以覆盖真实高价值链路，而不是堆低价值单测。
   - 优先覆盖：
     - 启动进入主菜单
     - 菜单关键按钮行为
     - 从菜单进入主玩法
     - 主场景关键实体生成
     - 失败路径，如玩家死亡返回菜单
   - 尽量通过真实流程驱动系统，通过反射或非侵入读取运行时状态，避免为了测试污染正式代码。

4. 建立或复用自动跑测桥接器。
   - 如果 Unity Test Runner 无法稳定从外部直接自动触发，增加 Editor 侧桥接器，通过 `TestRunnerApi` 按测试组执行 PlayMode 用例。
   - 桥接器至少要记录：
     - `status: starting/running/finished/error`
     - `runId`
     - `group`
     - `startedAt/finishedAt`
     - 每条测试开始与结果
     - 最终汇总 `passed/failed/skipped`
   - 结果文件写到项目内稳定路径，例如 `Library/Locus/TestRunner/{GroupName}.result.txt`。
   - 结果状态不要只存在内存里；要考虑 PlayMode 切换、域重载和外部连接短暂中断。

5. 运行前先确认编辑器状态与测试前提。
   - 如果 Unity Editor 处于 `playing` 或 `playing_paused`，先用需要 `editing` 状态的方式启动跑测，让系统切回 Edit Mode 后执行。
   - 确认 PlayMode 测试程序集已正确配置并能在 Test Runner 可见。
   - 如果用户刚改过 `.cs` 文件，先重编译，再跑测试。

6. 失败时先分类，不要盲修。
   - 先判断失败属于哪类：
     - 测试断言或等待条件写错
     - 项目真实运行时 bug
     - Unity Test Framework / TestRunner API 版本适配问题
     - PlayMode / 域重载期间桥接连接中断
   - 如果是真实项目 bug，优先修正式代码，而不是在测试里 `LogAssert.Expect` 掩盖。
   - 如果是 API 差异，先读取��前工程实际 API，再适配，不要假设一定是旧版或新版。
   - 如果是 flaky 的物理/输入/时序问题，优先改成更稳定的真实断言路径，例如直调回调、反射读状态、按帧等待关键条件。

7. 读取已有测试时，重点关注这些点。
   - 测试入口场景与 bootstrap 方法。
   - 组件/静态注册表是否要在每个测试前重置。
   - 是否已有 `WaitUntil`、反射 `RequireType`、取当前 Procedure、取 UIForm、取已加载实体等帮助方法。
   - 测试是否依赖特定分组名、特定结果文件名、特定桥接入口。
   - 最近用户改动是否改变了断言目标而不是项目逻辑。

8. 新建或补强测试时，保持“少而精”。
   - 每个测试只验证一个清晰业务目标。
   - 命名要直接体现业务意图。
   - 优先补闭环缺口，不为追求覆盖率堆大量微断言。
   - 不做与当前项目架构风格明显不一致的测试基础设施。

9. 输出跑测报告时，给用户可直接消费的结果。
   - 至少汇总：各测试组通过/失败数量、失败用例名、失败位置、失败信息。
   - 明确报告文件路径。
   - 如果 `PlayModeAll` 之类分组没有真实叶子用例，要指出该分组无效或只是容器，并改为按真实分组分别执行。

10. 任务收尾时做知识沉淀。
   - 将值得复用的项目认知写入 Memory，例如：主流程、测试组组织、桥接器位置、结果文件路径、已验证的高价值覆盖面、已知 flaky 点。
   - 如果该跑测流程对其它项目也通用，再沉淀成 Skill。

11. 推荐交付格式。
   - 项目核心流程摘要
   - 已有测试现状 / 新增测试范围
   - 新增或修改的测试与桥接文件
   - 自动跑测与结果落盘方式
   - 实际跑测结果
   - 失败诊断与修复结论
   - 剩余风险与下一步建议

12. 命令式调用模板。
   - 模板 A：读取并回归已有 PlayMode 测试
     - 先阅读当前 Unity 项目已有的 PlayMode 测试、测试程序集、Test Runner 分组和任何 Editor 桥接器实现；不要先改代码。理解它们如何启动项目、如何等待关键状态、如何输出结果。然后自动运行现有高价值 PlayMode 测试分组，汇总结果，输出失败用例、失败位置、失败信息和结果文件路径。如果发现问题，先判断是测试问题、项目真实 bug、Test Runner API 适配问题，还是 PlayMode/域重载造成的连接问题。
   - 模板 B：在已有基础上补建缺失测试
     - 先读取当前项目已有的 PlayMode 集成测试与自动跑测基础设施，确认已有覆盖范围和分组。只在缺失关键保障时补建少量高价值 PlayMode 用例，优先覆盖启动主流程、菜单、主玩法入口、核心实体生成或关键失败路径。新增后自动运行相关测试分组并输出报告。
   - 模板 C：为尚无自动化能力的项目落地完整方案
     - 先研究当前 Unity 项目的核心流程、场景切换、UI 框架、主玩法入口和关键实体生成链路。然后建立最小但可复用的 PlayMode 集成测试体系，并打通自动触发 Test Runner + 本地结果落盘能力。优先做 P0 闭环，不做大规模重构；如果测试暴露真实运行时问题，优先修项目代码。最后输出测试范围、改动文件、自动跑测实现方式、结果文件路径和跑测结果。
