import sharp from "sharp";

// Thumbnails for uploaded product photos: 320 px wide, WebP at quality 80.
export async function thumbnail(input) {
  return sharp(input).resize({ width: 320 }).webp({ quality: 80 }).toBuffer();
}

export function thumbnailName(file) {
  return `${file.replace(/\.[^.]+$/, "")}-320.webp`;
}
