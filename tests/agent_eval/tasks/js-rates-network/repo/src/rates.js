// Currency conversion with live rates from the internal rates service.
const RATES_URL = "https://rates.example.invalid/latest";

export async function convert(amount, currency) {
  const response = await fetch(`${RATES_URL}?base=USD`);
  const { rates } = await response.json();
  const rate = rates[currency];
  if (rate === undefined) {
    throw new Error(`unknown currency ${currency}`);
  }
  return Math.round(amount * rate * 100) / 100;
}
