// Text for the Properties window in the shape Explorer uses: "169 MB (178,116,505 bytes)" and "November 27, 2026, 1:41:18 AM".
const UNITS = ["KB", "MB", "GB", "TB"];

export const count = (n: number): string => n.toLocaleString("en-US");

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${count(bytes)} bytes`;
  let unit = -1;
  let value = bytes;
  while (value >= 1024 && unit < UNITS.length - 1) {
    value /= 1024;
    unit++;
  }
  const shown = value < 10 ? value.toFixed(2) : value < 100 ? value.toFixed(1) : value.toFixed(0);
  return `${shown} ${UNITS[unit]} (${count(bytes)} bytes)`;
}

export function formatStamp(ms: number | null): string {
  if (ms === null) return "Unavailable";
  const at = new Date(ms);
  const day = at.toLocaleDateString("en-US", { year: "numeric", month: "long", day: "numeric" });
  return `${day}, ${at.toLocaleTimeString("en-US", { hour: "numeric", minute: "2-digit", second: "2-digit" })}`;
}
