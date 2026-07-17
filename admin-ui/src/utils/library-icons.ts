import type { TeamLibrary } from "../api";

const libraryIconSize = 256;

export class LibraryIconImageError extends Error {
  constructor() {
    super("Invalid library icon image");
    this.name = "LibraryIconImageError";
  }
}

export function pickLibraryIconFile() {
  return new Promise<File | null>((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = "image/*";
    input.onchange = () => resolve(input.files?.[0] ?? null);
    input.oncancel = () => resolve(null);
    input.click();
  });
}

export async function fileToLibraryIconWebpBytes(file: File) {
  if (!file.type.startsWith("image/")) {
    throw new LibraryIconImageError();
  }

  const imageUrl = URL.createObjectURL(file);
  try {
    const image = new Image();
    image.decoding = "async";
    const loadedImage = await new Promise<HTMLImageElement>((resolve, reject) => {
      image.onload = () => resolve(image);
      image.onerror = () => reject(new LibraryIconImageError());
      image.src = imageUrl;
    });

    if (!loadedImage.naturalWidth || !loadedImage.naturalHeight) {
      throw new LibraryIconImageError();
    }

    const canvas = document.createElement("canvas");
    canvas.width = libraryIconSize;
    canvas.height = libraryIconSize;
    const context = canvas.getContext("2d");
    if (!context) {
      throw new LibraryIconImageError();
    }

    const sourceSize = Math.min(loadedImage.naturalWidth, loadedImage.naturalHeight);
    const sourceX = (loadedImage.naturalWidth - sourceSize) / 2;
    const sourceY = (loadedImage.naturalHeight - sourceSize) / 2;
    context.clearRect(0, 0, libraryIconSize, libraryIconSize);
    context.drawImage(
      loadedImage,
      sourceX,
      sourceY,
      sourceSize,
      sourceSize,
      0,
      0,
      libraryIconSize,
      libraryIconSize,
    );

    const blob = await new Promise<Blob>((resolve, reject) => {
      canvas.toBlob(
        (result) => result ? resolve(result) : reject(new LibraryIconImageError()),
        "image/webp",
        0.88,
      );
    });
    return Array.from(new Uint8Array(await blob.arrayBuffer()));
  } finally {
    URL.revokeObjectURL(imageUrl);
  }
}

export function resolveLibraryIconUrl(library: TeamLibrary, token: string | null | undefined) {
  if (!library.iconUrl || !token) {
    return null;
  }

  const url = new URL(library.iconUrl, window.location.origin);
  url.searchParams.set("token", token);
  if (library.updatedAt) {
    url.searchParams.set("v", library.updatedAt);
  }
  return url.toString();
}
