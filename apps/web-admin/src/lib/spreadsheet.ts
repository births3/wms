export async function readSpreadsheetRows(file: File): Promise<string[][]> {
  const extension = file.name.toLowerCase().split(".").pop();
  if (extension === "csv") return parseCsv(await file.text());
  if (extension === "xls") return parseHtmlSpreadsheet(await file.text());
  if (extension === "xlsx") return parseXlsx(await file.arrayBuffer());
  throw new Error("仅支持 .xlsx、.xls 或 .csv 文件");
}

export function parseCsv(text: string): string[][] {
  const rows: string[][] = [];
  let row: string[] = [];
  let cell = "";
  let quoted = false;
  for (let index = 0; index < text.length; index += 1) {
    const char = text[index];
    const next = text[index + 1];
    if (char === '"' && quoted && next === '"') {
      cell += '"';
      index += 1;
    } else if (char === '"') {
      quoted = !quoted;
    } else if (char === "," && !quoted) {
      row.push(cell.trim());
      cell = "";
    } else if ((char === "\n" || char === "\r") && !quoted) {
      if (char === "\r" && next === "\n") index += 1;
      row.push(cell.trim());
      if (row.some(Boolean)) rows.push(row);
      row = [];
      cell = "";
    } else {
      cell += char;
    }
  }
  if (cell || row.length) {
    row.push(cell.trim());
    if (row.some(Boolean)) rows.push(row);
  }
  return rows;
}

function parseHtmlSpreadsheet(text: string): string[][] {
  const document = new DOMParser().parseFromString(text, "text/html");
  const table = document.querySelector("table");
  if (!table) throw new Error("XLS 文件中没有可读取的表格");
  return Array.from(table.rows).map((row) => Array.from(row.cells).map((cell) => cell.textContent?.trim() ?? ""));
}

async function parseXlsx(buffer: ArrayBuffer): Promise<string[][]> {
  const bytes = new Uint8Array(buffer);
  const entries = readZipEntries(bytes);
  const sharedStrings = entries.has("xl/sharedStrings.xml")
    ? readSharedStrings(await readZipEntry(bytes, entries.get("xl/sharedStrings.xml")!))
    : [];
  const sheetName = entries.has("xl/worksheets/sheet1.xml") ? "xl/worksheets/sheet1.xml" : findFirstWorksheet(entries);
  if (!sheetName) throw new Error("XLSX 文件中没有工作表");
  return readWorksheet(await readZipEntry(bytes, entries.get(sheetName)!), sharedStrings);
}

function readSharedStrings(xml: string): string[] {
  const document = parseXml(xml);
  return Array.from(document.querySelectorAll("si")).map((item) => item.textContent?.trim() ?? "");
}

function readWorksheet(xml: string, sharedStrings: string[]): string[][] {
  const document = parseXml(xml);
  const rows: string[][] = [];
  for (const row of Array.from(document.querySelectorAll("row"))) {
    const cells = Array.from(row.querySelectorAll("c"));
    const values: string[] = [];
    for (const cell of cells) {
      const reference = cell.getAttribute("r") ?? "";
      const column = columnNumber(reference);
      const type = cell.getAttribute("t");
      const raw = type === "inlineStr"
        ? cell.querySelector("is")?.textContent ?? ""
        : cell.querySelector("v")?.textContent ?? "";
      values[column] = type === "s" ? sharedStrings[Number(raw)] ?? "" : raw;
    }
    while (values.length && values.at(-1) === undefined) values.pop();
    rows.push(values.map((value) => value ?? ""));
  }
  return rows.filter((row) => row.some(Boolean));
}

function findFirstWorksheet(entries: Map<string, ZipEntry>): string | null {
  return Array.from(entries.keys()).find((name) => /^xl\/worksheets\/sheet\d+\.xml$/.test(name)) ?? null;
}

function parseXml(xml: string): XMLDocument {
  const document = new DOMParser().parseFromString(xml, "application/xml");
  if (document.querySelector("parsererror")) throw new Error("XLSX XML 解析失败");
  return document;
}

type ZipEntry = { method: number; compressedSize: number; localOffset: number };

function readZipEntries(bytes: Uint8Array): Map<string, ZipEntry> {
  const end = findSignature(bytes, 0x06054b50, Math.max(0, bytes.length - 22));
  if (end < 0) throw new Error("不是有效的 XLSX 压缩包");
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const count = view.getUint16(end + 10, true);
  const directoryOffset = view.getUint32(end + 16, true);
  const entries = new Map<string, ZipEntry>();
  let offset = directoryOffset;
  const decoder = new TextDecoder();
  for (let index = 0; index < count; index += 1) {
    if (view.getUint32(offset, true) !== 0x02014b50) throw new Error("XLSX 目录损坏");
    const nameLength = view.getUint16(offset + 28, true);
    const extraLength = view.getUint16(offset + 30, true);
    const commentLength = view.getUint16(offset + 32, true);
    entries.set(decoder.decode(bytes.slice(offset + 46, offset + 46 + nameLength)), {
      method: view.getUint16(offset + 10, true),
      compressedSize: view.getUint32(offset + 20, true),
      localOffset: view.getUint32(offset + 42, true),
    });
    offset += 46 + nameLength + extraLength + commentLength;
  }
  return entries;
}

async function readZipEntry(bytes: Uint8Array, entry: ZipEntry): Promise<string> {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const offset = entry.localOffset;
  if (view.getUint32(offset, true) !== 0x04034b50) throw new Error("XLSX 文件项损坏");
  const nameLength = view.getUint16(offset + 26, true);
  const extraLength = view.getUint16(offset + 28, true);
  const start = offset + 30 + nameLength + extraLength;
  const compressed = bytes.slice(start, start + entry.compressedSize);
  let content = compressed;
  if (entry.method === 8) {
    if (!("DecompressionStream" in globalThis)) throw new Error("当前浏览器不支持 XLSX 解压");
    const stream = new Blob([compressed]).stream().pipeThrough(new DecompressionStream("deflate-raw"));
    content = new Uint8Array(await new Response(stream).arrayBuffer());
  } else if (entry.method !== 0) {
    throw new Error("XLSX 使用了当前浏览器不支持的压缩方式");
  }
  return new TextDecoder().decode(content);
}

function columnNumber(reference: string): number {
  const letters = reference.match(/^[A-Z]+/i)?.[0].toUpperCase() ?? "A";
  return Array.from(letters).reduce((value, letter) => value * 26 + letter.charCodeAt(0) - 64, 0) - 1;
}

function findSignature(bytes: Uint8Array, signature: number, from: number): number {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  for (let offset = from; offset >= 0; offset -= 1) {
    if (view.getUint32(offset, true) === signature) return offset;
  }
  return -1;
}
