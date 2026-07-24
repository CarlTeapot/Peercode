/** Subset of Monaco's IModelContentChange: the replaced span in PRE-edit
 *  offsets plus the inserted text. */
export interface ModelChange {
  rangeOffset: number;
  rangeLength: number;
  text: string;
}

/** An edit against the POST-edit document that undoes one ModelChange:
 *  replace [start, end) with `text` (the originally deleted characters). */
export interface InverseEdit {
  start: number;
  end: number;
  text: string;
}

/**
 * Compute exact inverse edits for a Monaco content-change event, given the
 * pre-edit document text (`shadow`). All of an event's changes are expressed
 * against the same pre-edit document, so a change's post-edit start offset is
 * its pre-edit offset shifted by the accumulated length delta of the changes
 * before it. Returned edits are in ascending `start` order and never overlap
 * (Monaco batch edits never overlap).
 */
export function computeInverseEdits(
  shadow: string,
  changes: ModelChange[],
): InverseEdit[] {
  const ascending = [...changes].sort((a, b) => a.rangeOffset - b.rangeOffset);
  const edits: InverseEdit[] = [];
  let delta = 0;
  for (const c of ascending) {
    const start = c.rangeOffset + delta;
    edits.push({
      start,
      end: start + c.text.length,
      text: shadow.slice(c.rangeOffset, c.rangeOffset + c.rangeLength),
    });
    delta += c.text.length - c.rangeLength;
  }
  return edits;
}
