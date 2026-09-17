import {
  MAX_REPORT_DETAILS,
  countReportDetails,
  detailsLengthAfterEdit,
} from "../reasons";

describe("report details length", () => {
  it("counts code points, not utf-16 units", () => {
    expect(countReportDetails("🙂🙂🙂")).toBe(3);
    expect(countReportDetails("abc")).toBe(3);
  });

  it("charges only what typing adds", () => {
    expect(detailsLengthAfterEdit("abc", 3, 3, "d")).toBe(4);
    expect(detailsLengthAfterEdit("abc", 3, 3, "🙂")).toBe(4);
  });

  it("gives back what the replaced selection held", () => {
    expect(detailsLengthAfterEdit("abcdef", 0, 3, "x")).toBe(4);
    expect(detailsLengthAfterEdit("🙂🙂🙂", 0, 4, "x")).toBe(2);
  });

  it("lets a full field be rewritten in place", () => {
    const full = "a".repeat(MAX_REPORT_DETAILS);
    expect(detailsLengthAfterEdit(full, 0, full.length, "fresh start")).toBe(11);
    expect(detailsLengthAfterEdit(full, 5, 6, "z")).toBe(MAX_REPORT_DETAILS);
    expect(detailsLengthAfterEdit(full, 5, 5, "z")).toBe(MAX_REPORT_DETAILS + 1);
  });
});
