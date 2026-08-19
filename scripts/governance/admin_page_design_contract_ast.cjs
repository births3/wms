#!/usr/bin/env node
const fs = require("fs");
const path = require("path");

function loadTypescript() {
  const candidates = [
    path.join(__dirname, "../../apps/web-admin/node_modules/typescript"),
    path.join(__dirname, "../../packages/ui/node_modules/typescript"),
    "typescript",
  ];
  for (const candidate of candidates) {
    try {
      return require(candidate);
    } catch (_error) {
      // try next candidate
    }
  }
  throw new Error("Cannot resolve TypeScript compiler API");
}

const ts = loadTypescript();
const WRITE_VERB_PATTERN = /新增|修改|保存|提交|停用|启用|取消|作废|发布|回滚|归档|迁移|切换|重发|下单|打印|审批|复核|交接|出库|拣货|校验/;

function parseArgs(argv) {
  const args = [...argv];
  let repoRoot = process.cwd();
  const repoIndex = args.indexOf("--repo-root");
  if (repoIndex !== -1) {
    repoRoot = args[repoIndex + 1];
    args.splice(repoIndex, 2);
  }
  return { repoRoot: path.resolve(repoRoot), files: args.map((file) => path.resolve(file)) };
}

function relativeFile(repoRoot, file) {
  return path.relative(repoRoot, file).split(path.sep).join("/");
}

function nameText(name) {
  if (!name) return "";
  if (ts.isIdentifier(name) || ts.isStringLiteral(name) || ts.isNumericLiteral(name)) return name.text;
  return name.getText();
}

function jsxTagName(node) {
  return node.tagName ? node.tagName.getText() : "";
}

function hasDialogAncestor(node) {
  let current = node.parent;
  while (current) {
    if (ts.isJsxElement(current)) {
      const tagName = jsxTagName(current.openingElement);
      if (/Dialog|AlertDialog/.test(tagName)) return true;
    }
    current = current.parent;
  }
  return false;
}

function stringValue(node) {
  if (!node) return "";
  if (ts.isStringLiteral(node) || ts.isNoSubstitutionTemplateLiteral(node)) return node.text;
  return "";
}

function jsxText(node) {
  if (!ts.isJsxElement(node)) return "";
  return node.children
    .filter((child) => ts.isJsxText(child))
    .map((child) => child.text.trim())
    .filter(Boolean)
    .join(" ");
}

function clickContextText(node) {
  if (ts.isJsxAttribute(node)) {
    const owner = node.parent.parent;
    const parts = [];
    for (const prop of node.parent.properties) {
      if (!ts.isJsxAttribute(prop)) continue;
      const key = nameText(prop.name);
      if (["label", "title", "aria-label"].includes(key)) parts.push(stringValue(prop.initializer));
    }
    if (ts.isJsxOpeningElement(owner) && ts.isJsxElement(owner.parent)) parts.push(jsxText(owner.parent));
    if (ts.isJsxElement(owner)) parts.push(jsxText(owner));
    return parts.join(" ");
  }

  if (ts.isPropertyAssignment(node) && ts.isObjectLiteralExpression(node.parent)) {
    const parts = [];
    for (const prop of node.parent.properties) {
      if (!ts.isPropertyAssignment(prop)) continue;
      const key = nameText(prop.name);
      if (["key", "label", "description"].includes(key)) parts.push(stringValue(prop.initializer));
    }
    return parts.join(" ");
  }
  return "";
}

function collectFunctionBodies(sourceFile) {
  const bodies = new Map();
  function visit(node) {
    if (ts.isFunctionDeclaration(node) && node.name && node.body) {
      bodies.set(node.name.text, node.body);
    }
    if (ts.isVariableDeclaration(node) && ts.isIdentifier(node.name) && node.initializer) {
      if (
        (ts.isArrowFunction(node.initializer) || ts.isFunctionExpression(node.initializer))
        && node.initializer.body
      ) {
        bodies.set(node.name.text, node.initializer.body);
      }
    }
    ts.forEachChild(node, visit);
  }
  visit(sourceFile);
  return bodies;
}

function isPropertyCall(call, objectName, methodNames) {
  if (!ts.isPropertyAccessExpression(call.expression)) return false;
  const methodName = call.expression.name.text;
  if (!methodNames.includes(methodName)) return false;
  if (!objectName) return true;
  return ts.isIdentifier(call.expression.expression) && call.expression.expression.text === objectName;
}

function isDialogStateCall(call, methodNames) {
  if (!ts.isPropertyAccessExpression(call.expression)) return false;
  if (!methodNames.includes(call.expression.name.text)) return false;
  const target = call.expression.expression;
  return ts.isIdentifier(target) && /Dialog$/.test(target.text);
}

function scanWriteAndConfirm(node, functionBodies, stack = new Set()) {
  let hasWrite = false;
  let hasConfirm = false;
  let hasDialogOpen = false;

  function merge(result) {
    hasWrite = hasWrite || result.hasWrite;
    hasConfirm = hasConfirm || result.hasConfirm;
    hasDialogOpen = hasDialogOpen || result.hasDialogOpen;
  }

  function visit(current) {
    if (ts.isCallExpression(current)) {
      if (isPropertyCall(current, null, ["mutate", "mutateAsync"])) hasWrite = true;
      if (isPropertyCall(current, "window", ["prompt", "print"])) hasWrite = true;
      if (isPropertyCall(current, "window", ["confirm"])) hasConfirm = true;
      if (
        isDialogStateCall(current, ["openWith"])
        || (
          isDialogStateCall(current, ["setOpen"])
          && current.arguments[0]?.kind === ts.SyntaxKind.TrueKeyword
        )
      ) {
        hasDialogOpen = true;
      }
      if (
        ts.isIdentifier(current.expression)
        && /^set[A-Z].*Open$/.test(current.expression.text)
        && current.arguments[0]?.kind === ts.SyntaxKind.TrueKeyword
      ) {
        hasDialogOpen = true;
      }

      if (ts.isIdentifier(current.expression)) {
        const name = current.expression.text;
        const body = functionBodies.get(name);
        if (body && !stack.has(name)) {
          const nextStack = new Set(stack);
          nextStack.add(name);
          merge(scanWriteAndConfirm(body, functionBodies, nextStack));
        }
      }
    }
    ts.forEachChild(current, visit);
  }

  if (ts.isIdentifier(node)) {
    const body = functionBodies.get(node.text);
    if (body && !stack.has(node.text)) {
      const nextStack = new Set(stack);
      nextStack.add(node.text);
      merge(scanWriteAndConfirm(body, functionBodies, nextStack));
      return { hasWrite, hasConfirm, hasDialogOpen };
    }
  }
  visit(node);
  return { hasWrite, hasConfirm, hasDialogOpen };
}

function clickExpression(node) {
  if (ts.isJsxAttribute(node)) {
    if (!node.initializer || !ts.isJsxExpression(node.initializer)) return null;
    return node.initializer.expression || null;
  }
  if (ts.isPropertyAssignment(node)) return node.initializer;
  return null;
}

function scanFile(repoRoot, file) {
  const text = fs.readFileSync(file, "utf8");
  const sourceFile = ts.createSourceFile(file, text, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
  const functionBodies = collectFunctionBodies(sourceFile);
  const issues = [];

  function maybeReportClick(node) {
    if (hasDialogAncestor(node)) return;
    const context = clickContextText(node);
    if (!WRITE_VERB_PATTERN.test(context)) return;
    const expression = clickExpression(node);
    if (!expression) return;
    const result = scanWriteAndConfirm(expression, functionBodies);
    if (result.hasWrite && !result.hasConfirm && !result.hasDialogOpen) {
      issues.push({
        file: relativeFile(repoRoot, file),
        kind: "direct_write_click",
        message: "写动作 onClick 不能直接或间接执行 mutate/浏览器动作，必须打开 Dialog/Confirm",
      });
    }
  }

  function visit(node) {
    if (ts.isJsxAttribute(node) && nameText(node.name) === "onClick") maybeReportClick(node);
    if (ts.isPropertyAssignment(node) && nameText(node.name) === "onClick") maybeReportClick(node);
    ts.forEachChild(node, visit);
  }

  visit(sourceFile);
  return issues;
}

function main() {
  const { repoRoot, files } = parseArgs(process.argv.slice(2));
  const issues = files.flatMap((file) => scanFile(repoRoot, file));
  process.stdout.write(JSON.stringify({ issues }, null, 2));
}

main();
