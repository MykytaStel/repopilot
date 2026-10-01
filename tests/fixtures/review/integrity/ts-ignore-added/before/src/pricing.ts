export interface Price {
  amount: number;
  currency: string;
}

export function format(price: Price): string {
  return `${price.amount.toFixed(2)} ${price.currency}`;
}
