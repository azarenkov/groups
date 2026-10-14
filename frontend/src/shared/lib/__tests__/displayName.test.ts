import i18n from "../../../i18n";
import { displayUserName, isDeletedUserName } from "../displayName";

beforeAll(async () => {
  await i18n.changeLanguage("ru");
});

afterAll(async () => {
  await i18n.changeLanguage("en");
});

it("shows a deleted account as a deleted user", () => {
  expect(isDeletedUserName("deleted_ff03b10930414a8883030f8800d4ffc2")).toBe(true);
  expect(displayUserName("deleted_ff03b10930414a8883030f8800d4ffc2")).toBe("Удалённый пользователь");
});

it("keeps ordinary names", () => {
  expect(isDeletedUserName("deleted_bob")).toBe(false);
  expect(displayUserName("Alexey")).toBe("Alexey");
});
