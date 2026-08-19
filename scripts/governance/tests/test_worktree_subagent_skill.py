"""wms-worktree-subagent 执行模式契约。"""

import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
SKILL = ROOT / ".agents/skills/wms-worktree-subagent/SKILL.md"
TEMPLATE = ROOT / ".agents/skills/wms-worktree-subagent/references/subagent-task-template.md"
CLOSEOUT = ROOT / ".agents/skills/wms-worktree-subagent/references/closeout.md"


def test_skill_defines_safe_execution_modes_and_parallel_policy():
    content = SKILL.read_text(encoding="utf-8")

    for mode in ("write-worktree", "read-only-worktree", "read-only-current-diff"):
        assert mode in content
    assert "-m <model>" in content
    assert "默认最多 3 个" in content
    assert "禁止静默降级" in content
    assert "每个子代理使用唯一 slug 和输出文件" in content
    assert "记录任务、模式、模型、范围、依赖、进程状态和退出码" in content
    assert "等待本批全部进程退出" in content


def test_task_template_records_mode_model_and_visible_snapshot():
    content = TEMPLATE.read_text(encoding="utf-8")

    assert "执行模式：<write-worktree | read-only-worktree | read-only-current-diff>" in content
    assert "模型：<model>" in content
    assert "可见快照" in content
    assert "read-only-current-diff" in content


def test_read_only_modes_have_distinct_output_and_closeout_contracts():
    skill = SKILL.read_text(encoding="utf-8")
    template = TEMPLATE.read_text(encoding="utf-8")
    closeout = CLOSEOUT.read_text(encoding="utf-8")

    assert "`read-only-worktree` 输出下一轮切片" in skill
    assert "`read-only-current-diff` 输出按严重度排序的发现" in skill
    assert "只输出下一轮切片、允许文件、停止条件、验证命令和技能缺口" not in skill
    assert "只读模式输出契约" in template
    assert "重新核对 `git status --short`" in closeout
    assert "不进入合并或分支清理矩阵" in closeout
    assert "汇总全部输出文件" in closeout
    assert "汇总唯一输出文件" not in closeout
    for required in ("读写范围", "依赖", "输出文件归属"):
        assert required in closeout
    assert "`read-only-current-diff` 不适用本节" in closeout


def test_current_diff_review_has_a_mode_specific_final_report():
    closeout = CLOSEOUT.read_text(encoding="utf-8")
    match = re.search(
        r"## `read-only-current-diff` 最终汇报\n(?P<body>[\s\S]*?)(?=\n## |\Z)",
        closeout,
    )

    assert match, "当前差异只读模式必须有独立最终汇报契约"
    section = match.group("body")
    for required in ("模式和模型", "审查范围", "进程退出码", "发现", "git status --short"):
        assert required in section
    for irrelevant in ("issue-agent", "tmux", "worktree 列表", "agent 分支"):
        assert irrelevant not in section


def test_read_only_modes_cannot_claim_module_completion_or_submit():
    skill = SKILL.read_text(encoding="utf-8")
    template = TEMPLATE.read_text(encoding="utf-8")

    assert "只读模式统一写 `审查完成` 或 `审查阻断`" in template
    assert "只读模式本身不触发提交" in skill
    assert not re.search(r"只读模式[^\n]*`本切片可合并`", template)
    output_contract = template.split("只读模式输出契约：", 1)[1]
    assert "本切片可合并" not in output_contract
    assert "是否可合并" not in output_contract
    assert "P0-P3" not in output_contract
    assert "P0-P2" in output_contract
    assert "完整性边界（仅 `write-worktree`）" in template
