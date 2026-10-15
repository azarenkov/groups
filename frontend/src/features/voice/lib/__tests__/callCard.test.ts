import i18n from "../../../../i18n";
import { callCardLabel, canCallBack, countsAsUnread } from "../callCard";

beforeAll(async () => {
  await i18n.changeLanguage("en");
});

it("counts only missed calls and ordinary messages as unread", () => {
  expect(countsAsUnread(undefined)).toBe(true);
  expect(countsAsUnread("missed")).toBe(true);
  expect(countsAsUnread("declined")).toBe(false);
  expect(countsAsUnread("completed")).toBe(false);
});

it("offers to call back only after a missed call", () => {
  expect(canCallBack("missed")).toBe(true);
  expect(canCallBack("declined")).toBe(false);
  expect(canCallBack(null)).toBe(false);
});

it("labels each call card by its outcome and side", () => {
  expect(callCardLabel("missed", null, false)).toBe("Missed call");
  expect(callCardLabel("missed", null, true)).toBe("No answer");
  expect(callCardLabel("declined", null, false)).toBe("Call declined");
  expect(callCardLabel("completed", 65, false)).toMatch(/1/);
});
