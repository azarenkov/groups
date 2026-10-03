import { render, screen } from "@testing-library/react";

import i18n from "../../../../../i18n";
import { SubscriptionSection } from "../SubscriptionSection";

describe("SubscriptionSection", () => {
  beforeAll(async () => {
    await i18n.changeLanguage("ru");
  });

  it("lists voice message and video note limits for every plan", () => {
    render(<SubscriptionSection />);
    expect(screen.getAllByText("Голосовые сообщения до 5 мин")).toHaveLength(1);
    expect(screen.getAllByText("Голосовые сообщения до 30 мин")).toHaveLength(2);
    expect(screen.getByText("Кружки 384×384")).toBeInTheDocument();
    expect(screen.getAllByText("Кружки в повышенном качестве 640×640")).toHaveLength(2);
  });

  it("lists sticker pack limits and no priority support", () => {
    render(<SubscriptionSection />);
    expect(screen.getByText("До 2 своих стикерпаков")).toBeInTheDocument();
    expect(screen.getByText("До 10 своих стикерпаков")).toBeInTheDocument();
    expect(screen.getByText("До 50 своих стикерпаков")).toBeInTheDocument();
    expect(screen.queryByText("Приоритетная поддержка")).toBeNull();
  });

  it("offers high noise suppression on the free plan", () => {
    render(<SubscriptionSection />);
    expect(screen.getByText("Шумоподавление: выключено, стандартное или высокое")).toBeInTheDocument();
    expect(screen.queryByText("Высокое шумоподавление")).toBeNull();
  });
});
