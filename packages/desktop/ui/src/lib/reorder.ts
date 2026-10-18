/// Moves `ids[from]` so it lands at insertion index `to` (0..ids.length, counted
/// before the move). Returns null when the order would not change.
export function moveToInsertionIndex(
  ids: string[],
  from: number,
  to: number,
): string[] | null {
  const insertAt = from < to ? to - 1 : to;
  if (insertAt === from) {
    return null;
  }
  const next = [...ids];
  const [moved] = next.splice(from, 1);
  next.splice(insertAt, 0, moved);
  return next;
}
