// Thumbnails for uploaded product photos: 320 px wide, WebP at quality 80.
// sharp is loaded on first use, so callers (and tests) can pass their own.
export async function thumbnail(input, load = () => import("sharp")) {
  const { default: sharp } = await load();
  return sharp(input).resize({ width: 320 }).webp({ quality: 80 }).toBuffer();
}

export function thumbnailName(file) {
  return `${file.replace(/\.[^.]+$/, "")}-320.webp`;
}
