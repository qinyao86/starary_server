import { zhCore } from "./zh/core";
import { zhFeatures } from "./zh/features";
import { zhRuntime } from "./zh/runtime";

export const zh = {
  ...zhCore,
  ...zhFeatures,
  ...zhRuntime
} as const;