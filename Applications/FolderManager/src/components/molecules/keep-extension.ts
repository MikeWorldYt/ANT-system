// A file's name editor selects the name without its extension and gives the extension back if it is erased, so a new user cannot drop it by mistake.
type Field = HTMLInputElement | HTMLTextAreaElement;

export const extOf = (name: string): string => {
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(dot) : "";
};

export function keepExtension(field: Field, ext: string): void {
  if (!ext) return;
  field.addEventListener("input", () => {
    if (field.value.toLowerCase().endsWith(ext.toLowerCase())) return;
    const caret = Math.min(field.selectionStart ?? field.value.length, field.value.length);
    field.value += ext;
    field.setSelectionRange(caret, caret);
  });
}

export const selectStem = (field: Field, ext: string): void => field.setSelectionRange(0, field.value.length - (field.value.toLowerCase().endsWith(ext.toLowerCase()) ? ext.length : 0));

// A text area that grows to fit the whole name, as Explorer shows a long name being renamed.
export function growToFit(field: HTMLTextAreaElement): () => void {
  const fit = (): void => {
    field.style.height = "auto";
    field.style.height = `${field.scrollHeight}px`;
  };
  field.addEventListener("input", fit);
  return fit;
}
