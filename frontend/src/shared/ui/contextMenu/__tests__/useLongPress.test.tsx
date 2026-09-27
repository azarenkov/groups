import { act, fireEvent, render, screen } from "@testing-library/react";

import { useLongPress } from "../useLongPress";

const Row = ({ onLongPress, onLinkClick }: { onLongPress: () => boolean; onLinkClick: () => void }) => {
  const longPress = useLongPress(onLongPress);
  return (
    <div data-testid="row" {...longPress}>
      <button type="button" onClick={onLinkClick}>
        link
      </button>
    </div>
  );
};

describe("useLongPress", () => {
  beforeEach(() => jest.useFakeTimers());
  afterEach(() => jest.useRealTimers());

  it("swallows the click that follows a long press", () => {
    const onLongPress = jest.fn(() => true);
    const onLinkClick = jest.fn();
    render(<Row onLongPress={onLongPress} onLinkClick={onLinkClick} />);
    const link = screen.getByRole("button", { name: "link" });
    fireEvent.touchStart(link, { touches: [{ clientX: 5, clientY: 5 }] });
    act(() => {
      jest.advanceTimersByTime(600);
    });
    fireEvent.touchEnd(link);
    fireEvent.click(link);
    expect(onLongPress).toHaveBeenCalledTimes(1);
    expect(onLinkClick).not.toHaveBeenCalled();
    fireEvent.click(link);
    expect(onLinkClick).toHaveBeenCalledTimes(1);
  });

  it("keeps a short tap as a normal click", () => {
    const onLongPress = jest.fn(() => true);
    const onLinkClick = jest.fn();
    render(<Row onLongPress={onLongPress} onLinkClick={onLinkClick} />);
    const link = screen.getByRole("button", { name: "link" });
    fireEvent.touchStart(link, { touches: [{ clientX: 5, clientY: 5 }] });
    act(() => {
      jest.advanceTimersByTime(100);
    });
    fireEvent.touchEnd(link);
    fireEvent.click(link);
    expect(onLongPress).not.toHaveBeenCalled();
    expect(onLinkClick).toHaveBeenCalledTimes(1);
  });

  it("cancels the pending long press when a second finger lands", () => {
    const onLongPress = jest.fn(() => true);
    render(<Row onLongPress={onLongPress} onLinkClick={jest.fn()} />);
    const row = screen.getByTestId("row");
    fireEvent.touchStart(row, { touches: [{ clientX: 5, clientY: 5 }] });
    fireEvent.touchStart(row, {
      touches: [
        { clientX: 5, clientY: 5 },
        { clientX: 40, clientY: 40 },
      ],
    });
    act(() => {
      jest.advanceTimersByTime(600);
    });
    expect(onLongPress).not.toHaveBeenCalled();
  });

  it("cancels the pending long press when a second finger lands anywhere", () => {
    const onLongPress = jest.fn(() => true);
    render(<Row onLongPress={onLongPress} onLinkClick={jest.fn()} />);
    fireEvent.touchStart(screen.getByTestId("row"), { touches: [{ clientX: 5, clientY: 5 }] });
    fireEvent.touchStart(document.body, {
      touches: [
        { clientX: 5, clientY: 5 },
        { clientX: 300, clientY: 300 },
      ],
    });
    act(() => {
      jest.advanceTimersByTime(600);
    });
    expect(onLongPress).not.toHaveBeenCalled();
  });

  it("keeps the click when the long press opened nothing", () => {
    const onLongPress = jest.fn(() => false);
    const onLinkClick = jest.fn();
    render(<Row onLongPress={onLongPress} onLinkClick={onLinkClick} />);
    const link = screen.getByRole("button", { name: "link" });
    fireEvent.touchStart(link, { touches: [{ clientX: 5, clientY: 5 }] });
    act(() => {
      jest.advanceTimersByTime(600);
    });
    fireEvent.touchEnd(link);
    fireEvent.click(link);
    expect(onLongPress).toHaveBeenCalledTimes(1);
    expect(onLinkClick).toHaveBeenCalledTimes(1);
  });
});
