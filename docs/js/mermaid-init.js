// Mermaid 初始化（兼容 mkdocs-material 是否启用 instant navigation）
(function () {
  function initMermaid() {
    if (typeof mermaid === "undefined") {
      return;
    }
    var scheme = document.body.getAttribute("data-md-color-scheme") || "default";
    mermaid.initialize({
      startOnLoad: false,
      theme: scheme === "slate" ? "dark" : "default",
      flowchart: { useMaxWidth: true, htmlLabels: true, curve: "basis" },
      stateDiagram: { useMaxWidth: true },
      securityLevel: "loose",
    });
    // 处理 pymdownx.superfences 输出的 <pre class="mermaid"><code>...</code></pre>
    // 把内部 <code> 的内容提到 <pre> 直接子节点（mermaid.run 才能识别）
    document.querySelectorAll("pre.mermaid > code").forEach(function (codeEl) {
      var pre = codeEl.parentNode;
      pre.textContent = codeEl.textContent;
    });
    mermaid.run({ querySelector: "pre.mermaid", suppressErrors: false }).catch(function (err) {
      console.error("[mermaid] render error:", err);
    });
  }

  // 1. material instant navigation（如启用）
  if (typeof document$ !== "undefined" && document$.subscribe) {
    document$.subscribe(function () {
      initMermaid();
    });
  } else {
    // 2. 普通页面加载
    if (document.readyState === "loading") {
      document.addEventListener("DOMContentLoaded", initMermaid);
    } else {
      initMermaid();
    }
  }
})();
