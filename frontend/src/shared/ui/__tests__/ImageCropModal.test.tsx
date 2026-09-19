import { fireEvent, render } from "@testing-library/react";

import "../../../i18n";
import ImageCropModal from "../ImageCropModal";

describe("ImageCropModal", () => {
  beforeAll(() => {
    URL.createObjectURL = jest.fn(() => "blob:preview");
    URL.revokeObjectURL = jest.fn();
  });

  it("keeps escape to itself so the screen underneath stays open", () => {
    const underneath = jest.fn();
    const onCancel = jest.fn();
    window.addEventListener("keydown", underneath);
    render(
      <ImageCropModal
        file={new File(["x"], "a.png", { type: "image/png" })}
        aspect={1}
        shape="circle"
        title="Crop"
        onCancel={onCancel}
        onConfirm={jest.fn()}
      />,
    );

    fireEvent.keyDown(document.body, { key: "Escape" });

    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(underneath).not.toHaveBeenCalled();
    window.removeEventListener("keydown", underneath);
  });
});
