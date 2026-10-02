import { slugify } from "./slug.js";

export function postUrl(post) {
  return `/posts/${slugify(post.title)}`;
}
