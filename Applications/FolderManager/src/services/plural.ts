// Counted nouns for the status messages, so every "1 file" / "2 files" reads the same way.
export const plural = (n: number, word: string): string => `${n} ${word}${n === 1 ? "" : "s"}`;
