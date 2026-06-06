import type { ClassValue } from "clsx";
import { clsx } from "clsx";
import { twMerge } from "tailwind-merge";

/** Merge conditional class lists and de-conflict Tailwind utilities (shadcn-vue `cn`). */
// Canonical variadic clsx wrapper; a rest parameter can't satisfy prefer-readonly-parameter-types,
// and a non-variadic signature would break the standard cn('a', cond && 'b', props.class) usage.
// oxlint-disable-next-line typescript/prefer-readonly-parameter-types
export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}
