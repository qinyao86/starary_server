export async function prepareAvatarImage(file: File): Promise<File> {
  if (!file.type.startsWith("image/")) {
    throw new Error("Please choose an image file.");
  }

  const image = await loadImage(file);
  const sourceSize = Math.min(image.naturalWidth, image.naturalHeight);
  const canvas = document.createElement("canvas");
  canvas.width = 256;
  canvas.height = 256;
  const context = canvas.getContext("2d");
  if (!context) throw new Error("Your browser cannot process this image.");

  context.drawImage(
    image,
    (image.naturalWidth - sourceSize) / 2,
    (image.naturalHeight - sourceSize) / 2,
    sourceSize,
    sourceSize,
    0,
    0,
    256,
    256
  );
  const blob = await new Promise<Blob>((resolve, reject) => {
    canvas.toBlob((value) => {
      if (value) resolve(value);
      else reject(new Error("The image could not be converted to WebP."));
    }, "image/webp", 0.9);
  });
  return new File([blob], "avatar.webp", { type: "image/webp" });
}

function loadImage(file: File): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const url = URL.createObjectURL(file);
    const image = new Image();
    image.onload = () => {
      URL.revokeObjectURL(url);
      if (image.naturalWidth && image.naturalHeight) resolve(image);
      else reject(new Error("The selected image is empty."));
    };
    image.onerror = () => {
      URL.revokeObjectURL(url);
      reject(new Error("The selected image could not be read."));
    };
    image.src = url;
  });
}
