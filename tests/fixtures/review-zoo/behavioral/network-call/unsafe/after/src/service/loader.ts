export async function load() {
  const res = await fetch("https://example.com/data");
  return res.json();
}
