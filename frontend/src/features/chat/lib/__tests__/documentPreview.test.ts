import {
  PREVIEW_SOURCE_MAX_BYTES,
  TEXT_PREVIEW_MAX_BYTES,
  documentKind,
  isPreviewable,
  parseCsv,
  prettyJson,
} from "../documentPreview";

describe("documentKind", () => {
  it("recognises documents by type or by extension", () => {
    expect(documentKind("application/pdf", "a.pdf")).toBe("pdf");
    expect(documentKind("application/octet-stream", "Scan.PDF")).toBe("pdf");
    expect(documentKind("application/vnd.openxmlformats-officedocument.presentationml.presentation", "x")).toBe("office");
    expect(documentKind("application/octet-stream", "budget.xlsx")).toBe("office");
    expect(documentKind("text/csv; charset=utf-8", "data")).toBe("csv");
    expect(documentKind("application/json", "a")).toBe("json");
    expect(documentKind("text/markdown", "README")).toBe("text");
    expect(documentKind("application/octet-stream", "main.rs")).toBe("text");
    expect(documentKind("application/zip", "a.zip")).toBeNull();
    expect(documentKind("application/octet-stream", ".docx")).toBeNull();
  });

  it("limits previews by size", () => {
    expect(isPreviewable("application/pdf", "a.pdf", PREVIEW_SOURCE_MAX_BYTES)).toBe(true);
    expect(isPreviewable("application/pdf", "a.pdf", PREVIEW_SOURCE_MAX_BYTES + 1)).toBe(false);
    expect(isPreviewable("text/plain", "a.txt", TEXT_PREVIEW_MAX_BYTES + 1)).toBe(false);
    expect(isPreviewable("image/png", "a.png", 1)).toBe(false);
  });
});

describe("parseCsv", () => {
  it("handles quotes, embedded delimiters and line breaks", () => {
    const { rows, truncated } = parseCsv('name,note\r\n"Smith, J","said ""hi""\nthen left"\nAnn,ok', 10);
    expect(rows).toEqual([
      ["name", "note"],
      ["Smith, J", 'said "hi"\nthen left'],
      ["Ann", "ok"],
    ]);
    expect(truncated).toBe(false);
  });

  it("detects semicolon-separated files and truncates long ones", () => {
    expect(parseCsv("a;b\n1;2\n", 10).rows).toEqual([["a", "b"], ["1", "2"]]);
    const { rows, truncated } = parseCsv("x\n1\n2\n3\n4\n", 2);
    expect(rows).toHaveLength(2);
    expect(truncated).toBe(true);
  });
});

describe("prettyJson", () => {
  it("indents valid JSON and leaves anything else untouched", () => {
    expect(prettyJson('{"a":[1,2]}')).toBe('{\n  "a": [\n    1,\n    2\n  ]\n}');
    expect(prettyJson("{oops")).toBe("{oops");
  });
});
