export type DocumentKind = "pdf" | "office" | "csv" | "json" | "text";

export const PREVIEW_SOURCE_MAX_BYTES = 50 * 1024 * 1024;
export const TEXT_PREVIEW_MAX_BYTES = 2 * 1024 * 1024;
export const CSV_PREVIEW_MAX_ROWS = 1000;

const OFFICE_TYPES = new Set([
  "application/msword",
  "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
  "application/vnd.oasis.opendocument.text",
  "application/rtf",
  "text/rtf",
  "application/vnd.ms-powerpoint",
  "application/vnd.openxmlformats-officedocument.presentationml.presentation",
  "application/vnd.oasis.opendocument.presentation",
  "application/vnd.ms-excel",
  "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
  "application/vnd.oasis.opendocument.spreadsheet",
]);

const OFFICE_EXTENSIONS = new Set(["doc", "docx", "odt", "rtf", "ppt", "pptx", "odp", "xls", "xlsx", "ods"]);

const TEXT_EXTENSIONS = new Set([
  "txt", "md", "markdown", "log", "ini", "cfg", "conf", "env", "yaml", "yml", "toml", "xml", "sql",
  "js", "jsx", "ts", "tsx", "py", "rs", "go", "java", "kt", "c", "h", "cpp", "hpp", "cs", "rb", "php",
  "sh", "bash", "zsh", "css", "scss", "html", "htm", "swift", "dart", "lua", "r", "m", "pl", "tex",
]);

const baseType = (contentType: string) => contentType.split(";")[0].trim().toLowerCase();

const extensionOf = (fileName: string): string | null => {
  const dot = fileName.lastIndexOf(".");
  if (dot <= 0 || dot === fileName.length - 1) return null;
  return fileName.slice(dot + 1).toLowerCase();
};

export const documentKind = (contentType: string, fileName: string): DocumentKind | null => {
  const type = baseType(contentType);
  const ext = extensionOf(fileName);
  if (type === "application/pdf" || ext === "pdf") return "pdf";
  if (OFFICE_TYPES.has(type) || (ext !== null && OFFICE_EXTENSIONS.has(ext))) return "office";
  if (type === "text/csv" || ext === "csv") return "csv";
  if (type === "application/json" || ext === "json") return "json";
  if (type.startsWith("text/") || (ext !== null && TEXT_EXTENSIONS.has(ext))) return "text";
  return null;
};

export const isPreviewable = (contentType: string, fileName: string, sizeBytes: number): boolean => {
  const kind = documentKind(contentType, fileName);
  if (kind === null) return false;
  const limit = kind === "pdf" || kind === "office" ? PREVIEW_SOURCE_MAX_BYTES : TEXT_PREVIEW_MAX_BYTES;
  return sizeBytes <= limit;
};

export const parseCsv = (text: string, maxRows: number): { rows: string[][]; truncated: boolean } => {
  const rows: string[][] = [];
  let row: string[] = [];
  let field = "";
  let quoted = false;
  const delimiter = (text.split("\n", 1)[0].match(/;/g)?.length ?? 0) > (text.split("\n", 1)[0].match(/,/g)?.length ?? 0) ? ";" : ",";
  const pushRow = () => {
    row.push(field);
    rows.push(row);
    row = [];
    field = "";
  };
  for (let i = 0; i < text.length; i += 1) {
    const ch = text[i];
    if (quoted) {
      if (ch === '"' && text[i + 1] === '"') {
        field += '"';
        i += 1;
      } else if (ch === '"') {
        quoted = false;
      } else {
        field += ch;
      }
      continue;
    }
    if (ch === '"' && field === "") {
      quoted = true;
    } else if (ch === delimiter) {
      row.push(field);
      field = "";
    } else if (ch === "\n") {
      pushRow();
      if (rows.length > maxRows) return { rows: rows.slice(0, maxRows), truncated: true };
    } else if (ch !== "\r") {
      field += ch;
    }
  }
  if (field !== "" || row.length > 0) pushRow();
  return { rows, truncated: false };
};

export const prettyJson = (text: string): string => {
  try {
    return JSON.stringify(JSON.parse(text), null, 2);
  } catch {
    return text;
  }
};
