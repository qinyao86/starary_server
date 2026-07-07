import { enCore } from "./en/core";
import { enFeatures } from "./en/features";
import { enRuntime } from "./en/runtime";

export const en = {
  ...enCore,
  ...enFeatures,
  ...enRuntime
} as const;