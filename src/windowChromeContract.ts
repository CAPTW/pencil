type WindowDragTarget = {
  closest: (selector: string) => unknown;
};

type WindowDragEvent = {
  button: number;
  target: unknown;
};

type WindowDragApi = {
  startDragging: () => Promise<void>;
};

const INTERACTIVE_WINDOW_CHROME_SELECTOR =
  "button, input, select, textarea, a, [role='button'], [data-window-no-drag]";

function isWindowDragTarget(target: unknown): target is WindowDragTarget {
  return (
    typeof target === "object" &&
    target !== null &&
    "closest" in target &&
    typeof target.closest === "function"
  );
}

export async function startWindowDrag(
  event: WindowDragEvent,
  windowApi: WindowDragApi,
): Promise<boolean> {
  if (
    event.button !== 0 ||
    !isWindowDragTarget(event.target) ||
    event.target.closest(INTERACTIVE_WINDOW_CHROME_SELECTOR) !== null
  ) {
    return false;
  }

  await windowApi.startDragging();
  return true;
}
