import { describe, expect, it } from "vitest";
import { joinWords, splitWords } from "./shellWords";

describe("shell-like argument notation", () => {
  it("splits on whitespace and honours quotes and escapes", () => {
    expect(splitWords("--model {model}")).toEqual(["--model", "{model}"]);
    expect(splitWords(`-c 'model_reasoning_effort="{effort}"'`)).toEqual([
      "-c",
      'model_reasoning_effort="{effort}"',
    ]);
    expect(splitWords(`--msg "say \\"hi\\" now"  tail`)).toEqual(["--msg", 'say "hi" now', "tail"]);
    expect(splitWords("a\\ b")).toEqual(["a b"]);
    expect(splitWords("  ")).toEqual([]);
    expect(splitWords("--flag ''")).toEqual(["--flag", ""]);
  });

  it("reports an open quote instead of guessing", () => {
    expect(splitWords(`--msg "unterminated`)).toBeNull();
    expect(splitWords("it's")).toBeNull();
  });

  it("round-trips whatever an argument contains", () => {
    const cases = [
      ["--model", "{model}"],
      ["-c", 'model_reasoning_effort="{effort}"'],
      ["it's", "both ' and \"", "back\\slash", "", "two words", "tab\there"],
      [],
    ];
    for (const words of cases) expect(splitWords(joinWords(words))).toEqual(words);
  });

  it("leaves simple arguments unquoted so the common case reads naturally", () => {
    expect(joinWords(["--resume", "{session_id}", "--fork-session"])).toBe(
      "--resume {session_id} --fork-session",
    );
  });
});
