import { fireEvent, render, screen } from "@testing-library/svelte";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import ErrorToast from "$lib/components/toast/ErrorToast.svelte";

describe("ErrorToast", () => {
  let onDismiss: () => void;

  beforeEach(() => {
    vi.useFakeTimers();
    onDismiss = vi.fn();
  });

  afterEach(() => {
    vi.runOnlyPendingTimers();
    vi.useRealTimers();
  });

  it("renders the message and type", () => {
    render(ErrorToast, {
      props: {
        message: "Permission denied",
        type: "error",
        duration: 5000,
        onDismiss,
      },
    });

    expect(screen.getByTestId("error-toast-message").textContent).toBe("Permission denied");
    expect(screen.getByTestId("error-toast").getAttribute("data-toast-type")).toBe("error");
  });

  it("auto-dismisses after the configured duration", () => {
    render(ErrorToast, {
      props: {
        message: "Network reset",
        type: "warning",
        duration: 1000,
        onDismiss,
      },
    });

    vi.advanceTimersByTime(999);
    expect(onDismiss).not.toHaveBeenCalled();

    vi.advanceTimersByTime(1);
    expect(onDismiss).toHaveBeenCalledTimes(1);
  });

  it("dismisses when the close button is clicked", async () => {
    render(ErrorToast, {
      props: {
        message: "Heads up",
        type: "info",
        duration: 5000,
        onDismiss,
      },
    });

    await fireEvent.click(screen.getByTestId("error-toast-dismiss"));

    expect(onDismiss).toHaveBeenCalledTimes(1);
  });
});
