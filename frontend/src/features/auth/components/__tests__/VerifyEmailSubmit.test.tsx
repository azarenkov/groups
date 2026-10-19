import { act, fireEvent, render, screen } from "@testing-library/react";

import i18n from "../../../../i18n";
import { VerifyEmail } from "../steps/VerifyEmail";
import { authAPI } from "../../api/auth";

jest.mock("../../api/auth", () => ({
  authAPI: { confirmRegistration: jest.fn(), refresh: jest.fn(), requestVerificationCode: jest.fn() },
}));

const confirm = authAPI.confirmRegistration as jest.Mock;

beforeAll(async () => {
  await i18n.changeLanguage("en");
});

const typeCode = (code: string) => {
  const inputs = screen.getAllByRole("textbox");
  code.split("").forEach((digit, index) => {
    fireEvent.change(inputs[index], { target: { value: digit } });
  });
};

it("submits a rejected code once instead of retrying it in a loop", async () => {
  confirm.mockRejectedValue(new Error("bad code"));
  render(<VerifyEmail email="alice@rift.app" onVerified={jest.fn()} onBack={jest.fn()} />);
  await act(async () => {
    typeCode("123456");
  });
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 50));
  });
  expect(confirm).toHaveBeenCalledTimes(1);
  expect(confirm).toHaveBeenCalledWith("123456");
});
