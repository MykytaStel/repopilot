// Deeply nested control flow: cognitive score 1+2+3+4+5+6 = 21, over the
// default threshold of 15. The nesting — not the line count — is the problem.
export function handle(items: number[]): void {
  if (items.length > 0) {
    for (const item of items) {
      if (item > 0) {
        while (item < 100) {
          if (item % 2 === 0) {
            if (item % 3 === 0) {
              process(item);
            }
          }
        }
      }
    }
  }
}

function process(value: number): void {
  console.log(value);
}
