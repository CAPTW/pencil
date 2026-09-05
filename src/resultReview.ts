export type ReviewOp = "equal" | "delete" | "insert";

export type ReviewSpan = Readonly<{
  op: ReviewOp;
  text: string;
}>;

export function tokenizeForReview(text: string): string[] {
  const tokens: string[] = [];
  let last = 0;
  for (let index = 0; index < text.length; ) {
    const character = text[index];
    if (character && /\s/.test(character)) {
      if (index > last) {
        tokens.push(text.slice(last, index));
      }
      tokens.push(character);
      index += 1;
      last = index;
    } else {
      index += 1;
    }
  }
  if (last < text.length) {
    tokens.push(text.slice(last));
  }
  return tokens;
}

export function reviewDiff(source: string, result: string): ReviewSpan[] {
  if (source === result) {
    return source ? [{ op: "equal", text: source }] : [];
  }
  const left = tokenizeForReview(source);
  const right = tokenizeForReview(result);
  if (left.length > 400 || right.length > 400) {
    const spans: ReviewSpan[] = [];
    if (source) spans.push({ op: "delete", text: source });
    if (result) spans.push({ op: "insert", text: result });
    return spans;
  }
  const table: number[][] = Array.from({ length: left.length + 1 }, () =>
    Array.from({ length: right.length + 1 }, () => 0),
  );
  for (let i = 0; i < left.length; i += 1) {
    for (let j = 0; j < right.length; j += 1) {
      table[i + 1][j + 1] =
        left[i] === right[j] ? table[i][j] + 1 : Math.max(table[i + 1][j], table[i][j + 1]);
    }
  }
  const ops: ReviewSpan[] = [];
  backtrack(table, left, right, left.length, right.length, ops);
  return mergeSpans(ops);
}

function backtrack(
  table: number[][],
  left: string[],
  right: string[],
  i: number,
  j: number,
  out: ReviewSpan[],
): void {
  if (i > 0 && j > 0 && left[i - 1] === right[j - 1]) {
    backtrack(table, left, right, i - 1, j - 1, out);
    out.push({ op: "equal", text: left[i - 1] });
    return;
  }
  if (j > 0 && (i === 0 || table[i][j - 1] >= table[i - 1][j])) {
    backtrack(table, left, right, i, j - 1, out);
    out.push({ op: "insert", text: right[j - 1] });
    return;
  }
  if (i > 0) {
    backtrack(table, left, right, i - 1, j, out);
    out.push({ op: "delete", text: left[i - 1] });
  }
}

function mergeSpans(spans: ReviewSpan[]): ReviewSpan[] {
  const merged: ReviewSpan[] = [];
  for (const span of spans) {
    const last = merged[merged.length - 1];
    if (last && last.op === span.op) {
      merged[merged.length - 1] = { op: last.op, text: last.text + span.text };
    } else {
      merged.push(span);
    }
  }
  return merged;
}

export function reviewChangeCount(spans: ReviewSpan[]): number {
  return spans.filter((span) => span.op !== "equal").length;
}
