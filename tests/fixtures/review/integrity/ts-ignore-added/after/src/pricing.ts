export interface Price {
  amount: number;
  currency: string;
}

export function format(price: Price): string {
  // @ts-ignore amount can be a string from the legacy API
  return `${price.amount.toFixed(2)} ${price.currency}`;
}
