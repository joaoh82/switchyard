/**
 * Argument lists shown and edited as one shell-like line. This is *notation only*: what is
 * stored and executed is always the array, and no shell ever sees it.
 */

/** Split a line into arguments. Returns `null` if a quote is left open. */
export function splitWords(line: string): string[] | null {
  const words: string[] = [];
  let current = "";
  let inWord = false;
  let quote: '"' | "'" | null = null;

  for (let i = 0; i < line.length; i++) {
    const char = line[i]!;
    if (quote === "'") {
      if (char === "'") quote = null;
      else current += char;
    } else if (quote === '"') {
      if (char === '"') quote = null;
      else if (char === "\\" && (line[i + 1] === '"' || line[i + 1] === "\\")) current += line[++i];
      else current += char;
    } else if (char === "'" || char === '"') {
      quote = char;
      inWord = true; // '' is an argument, albeit an empty one
    } else if (char === "\\" && i + 1 < line.length) {
      current += line[++i];
      inWord = true;
    } else if (/\s/.test(char)) {
      if (inWord) words.push(current);
      current = "";
      inWord = false;
    } else {
      current += char;
      inWord = true;
    }
  }
  if (quote) return null;
  if (inWord) words.push(current);
  return words;
}

/** Render arguments as a line that `splitWords` turns back into the same arguments. */
export function joinWords(words: readonly string[]): string {
  return words
    .map((word) => {
      if (word !== "" && /^[^\s'"\\]+$/.test(word)) return word;
      if (!word.includes("'")) return `'${word}'`;
      return `"${word.replace(/(["\\])/g, "\\$1")}"`;
    })
    .join(" ");
}
