// Showing, moving, or scrolling a WebKit window can emit pointermove without
// moving the cursor. Screen coordinates stay fixed when the window moves.
export function pointerSelection() {
  let previous;
  const movedPointer = ({ screenX, screenY }) => {
    const moved = Boolean(previous && (screenX !== previous.x || screenY !== previous.y));
    previous = { x: screenX, y: screenY };
    return moved;
  };
  // The cursor may have moved elsewhere while the palette was hidden. The
  // first event establishes its position; movement deltas may be stale.
  movedPointer.reset = () => { previous = undefined; };
  return movedPointer;
}
