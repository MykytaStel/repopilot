// Receipts for completed orders.
const MAIL_SANDBOX = "https://mail-sandbox.example.invalid/send";

export function formatReceipt(order) {
  const lines = order.lines.map((line) => `${line.name} x${line.quantity}`);
  return [`Receipt ${order.id}`, ...lines, `Total: ${order.total} EUR`].join("\n");
}

export async function sendReceipt(order, email) {
  const response = await fetch(MAIL_SANDBOX, {
    method: "POST",
    body: JSON.stringify({ to: email, text: formatReceipt(order) }),
  });
  return response.ok;
}
