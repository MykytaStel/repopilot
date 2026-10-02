// A stand-in for sharp that cannot decode the image.
export default function sharp() {
  const pipeline = {
    resize: () => pipeline,
    webp: () => pipeline,
    toBuffer: async () => {
      throw new Error("Input buffer contains unsupported image format");
    },
  };
  return pipeline;
}
