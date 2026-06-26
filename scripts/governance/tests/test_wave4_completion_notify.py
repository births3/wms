"""Wave 4 完成通知治理测试。"""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


def test_notify_wave4_completion_payload_shape():
    """企微 webhook payload 必须是文本消息，不包含 URL/key。"""
    import notify_wave4_completion as notify

    payload = json.loads(
        notify.build_qy_wechat_payload("Wave 4 complete").decode("utf-8")
    )

    assert payload == {
        "msgtype": "text",
        "text": {
            "content": "Wave 4 complete",
        },
    }


def test_notify_wave4_completion_skips_when_gate_fails(monkeypatch):
    """Wave 4 strict gate 未通过时，hook 目标必须不发送 webhook。"""
    import notify_wave4_completion as notify

    monkeypatch.setattr(
        notify,
        "run_completion_check",
        lambda: notify.CompletionCheck(ok=False, output="阻塞缺口: 1"),
    )

    def fail_if_called(*args, **kwargs):
        raise AssertionError("webhook must not be sent when Wave 4 is incomplete")

    monkeypatch.setattr(notify, "post_qy_wechat_webhook", fail_if_called)

    assert notify.main([]) == 0


def test_notify_wave4_completion_requires_env_when_gate_passes(monkeypatch):
    """Wave 4 已完成但缺少环境变量时，应失败而不是伪装已通知。"""
    import notify_wave4_completion as notify

    monkeypatch.setattr(
        notify,
        "run_completion_check",
        lambda: notify.CompletionCheck(ok=True, output="ok"),
    )
    monkeypatch.delenv(notify.DEFAULT_WEBHOOK_ENV, raising=False)

    assert notify.main([]) == 1


def test_notify_wave4_completion_posts_when_gate_passes(monkeypatch):
    """Wave 4 strict gate 通过且 env 存在时，才发送企微 webhook。"""
    import notify_wave4_completion as notify

    sent = {}

    monkeypatch.setattr(
        notify,
        "run_completion_check",
        lambda: notify.CompletionCheck(ok=True, output="ok"),
    )
    monkeypatch.setenv(notify.DEFAULT_WEBHOOK_ENV, "https://qyapi.weixin.qq.test/send")

    def fake_post(webhook_url, content, *, timeout_seconds):
        sent["webhook_url"] = webhook_url
        sent["content"] = content
        sent["timeout_seconds"] = timeout_seconds
        return 200, '{"errcode":0,"errmsg":"ok"}'

    monkeypatch.setattr(notify, "post_qy_wechat_webhook", fake_post)

    assert notify.main(["--message", "done", "--timeout-seconds", "1"]) == 0
    assert sent == {
        "webhook_url": "https://qyapi.weixin.qq.test/send",
        "content": "done",
        "timeout_seconds": 1.0,
    }
