"""H1 真实 Playwright 配置只能收集 H1 用例。"""

import re
from pathlib import Path


CONFIG = Path(__file__).parents[3] / "prototypes" / "playwright-web-admin-h1-real-config.ts"


def _test_match_pattern() -> re.Pattern[str]:
    text = CONFIG.read_text(encoding="utf-8")
    match = re.search(r"testMatch:\s*/([^/]+)/", text)
    assert match, "H1 Playwright config must declare testMatch"
    return re.compile(match.group(1))


def test_h1_real_config_excludes_unrelated_real_suites():
    pattern = _test_match_pattern()

    assert pattern.search("web-admin-h1-real.spec.ts")
    assert pattern.search("web-admin-h1-api-key-real.spec.ts")
    assert not pattern.search("web-admin-di-real.spec.ts")
    assert not pattern.search("web-admin-te-real.spec.ts")
