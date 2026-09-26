import { MENU_MARGIN, placeMenu } from "../placeMenu";

describe("placeMenu", () => {
  it("opens at the cursor when there is room", () => {
    expect(placeMenu(100, 100, 200, 150, 1200, 800)).toEqual({
      left: 100,
      top: 100,
      maxHeight: 800 - 100 - MENU_MARGIN,
      maxWidth: 1200 - 100 - MENU_MARGIN,
    });
  });

  it("flips to the left of the cursor near the right edge", () => {
    expect(placeMenu(1100, 100, 200, 150, 1200, 800)).toEqual({
      left: 900,
      top: 100,
      maxHeight: 800 - 100 - MENU_MARGIN,
      maxWidth: 1200 - 900 - MENU_MARGIN,
    });
  });

  it("slides up near the bottom edge", () => {
    expect(placeMenu(100, 750, 200, 150, 1200, 800)).toEqual({
      left: 100,
      top: 800 - 150 - MENU_MARGIN,
      maxHeight: 150,
      maxWidth: 1200 - 100 - MENU_MARGIN,
    });
  });

  it("never leaves the window in a tiny viewport", () => {
    expect(placeMenu(50, 50, 400, 400, 300, 300)).toEqual({
      left: MENU_MARGIN,
      top: MENU_MARGIN,
      maxHeight: 300 - 2 * MENU_MARGIN,
      maxWidth: 300 - 2 * MENU_MARGIN,
    });
  });
});
