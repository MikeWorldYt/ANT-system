// Local time in the YY-MM-DD-HH-MM form the debug logs and backups are named with.
const two = (n: number) => String(n).padStart(2, "0");

export const localStamp = (d = new Date()): string =>
  [d.getFullYear() % 100, d.getMonth() + 1, d.getDate(), d.getHours(), d.getMinutes()].map(two).join("-");
