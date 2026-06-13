// A short router whose branch density (branch constructs x 1000 / LOC) sits
// above the complexity threshold, even though the file itself is small. This
// is a file-level metric, distinct from per-function cognitive complexity.
function route(method, path) {
  if (method === "GET") {
    return handleGet(path);
  } else if (method === "POST") {
    return handlePost(path);
  } else if (method === "PUT") {
    return handlePut(path);
  } else if (method === "DELETE") {
    return handleDelete(path);
  }
  return notFound(path);
}
