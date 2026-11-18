// VS Code's chords in the dictionary editor: Ctrl+K Ctrl+C wraps the selected lines in `%% %%`, or unwraps them when all already are; Ctrl+K Ctrl+U only unwraps.
const OPEN = "%% ";
const CLOSE = " %%";

const commented = (line: string): boolean => line.trimStart().startsWith("%%") && line.trimEnd().endsWith("%%") && line.trim().length >= 4;

function comment(line: string): string {
  if (!line.trim() || commented(line)) return line;
  const indent = line.length - line.trimStart().length;
  return `${line.slice(0, indent)}${OPEN}${line.slice(indent)}${CLOSE}`;
}

function uncomment(line: string): string {
  if (!commented(line)) return line;
  const indent = line.length - line.trimStart().length;
  const inner = line.trim().slice(2, -2);
  return `${line.slice(0, indent)}${inner.startsWith(" ") ? inner.slice(1) : inner}`.replace(/ $/, "");
}

function apply(text: HTMLTextAreaElement, mode: "toggle" | "uncomment"): void {
  const value = text.value;
  const from = value.lastIndexOf("\n", text.selectionStart - 1) + 1;
  // A selection that ends at the start of a line leaves that line alone, as in VS Code.
  const stop = text.selectionEnd > text.selectionStart && value[text.selectionEnd - 1] === "\n" ? text.selectionEnd - 1 : text.selectionEnd;
  const next = value.indexOf("\n", stop);
  const to = next === -1 ? value.length : next;
  const lines = value.slice(from, to).split("\n");
  const unwrap = mode === "uncomment" || lines.filter((l) => l.trim()).every(commented);
  const replaced = lines.map(unwrap ? uncomment : comment).join("\n");
  text.focus();
  text.setSelectionRange(from, to);
  // insertText keeps the change on the undo stack and fires the input event that saves it.
  if (!document.execCommand("insertText", false, replaced)) {
    text.setRangeText(replaced, from, to, "end");
    text.dispatchEvent(new Event("input"));
  }
  text.setSelectionRange(from, from + replaced.length);
}

export function installCommentKeys(text: HTMLTextAreaElement): void {
  let chord = false;
  text.addEventListener("keydown", (ev) => {
    if (!ev.ctrlKey || ev.altKey) {
      chord = false;
      return;
    }
    const key = ev.key.toLowerCase();
    if (key === "control") return;
    if (chord && (key === "c" || key === "u")) {
      ev.preventDefault();
      chord = false;
      apply(text, key === "c" ? "toggle" : "uncomment");
      return;
    }
    chord = key === "k";
    if (chord) ev.preventDefault();
  });
}
