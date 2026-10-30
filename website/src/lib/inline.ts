/** Splits release-note text on backticks, so `gitview --claude` renders as code. */
export const inlineCode = (text: string) =>
  text.split("`").map((part, index) => ({ text: part, code: index % 2 === 1 }));
