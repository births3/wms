"""mkdocs hook：仅为功能方案说明页注入原始 Markdown，供"复制 Markdown"按钮使用。"""
import json

TARGET = "wms-scope-spec.md"
_store = {}


def on_page_markdown(markdown, page, config, files):
    if page.file.src_path.replace("\\", "/") == TARGET:
        # 去掉按钮行（含 no-print 标记），复制出的 Markdown 保持干净
        _store[TARGET] = "\n".join(
            ln for ln in markdown.splitlines() if "no-print" not in ln
        )
    return markdown


def on_post_page(output, page, config, **kwargs):
    if page.file.src_path.replace("\\", "/") != TARGET:
        return output
    payload = json.dumps(_store.get(TARGET, ""))
    return output.replace("</body>", f"<script>window.__PAGE_MD__={payload}</script></body>")
