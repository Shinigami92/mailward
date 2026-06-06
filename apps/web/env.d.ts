/// <reference types="vite/client" />
/// <reference types="unplugin-icons/types/vue" />

// Type-aware lint (tsgolint) does not model `.vue` SFCs the way vue-tsc/Volar does,
// so without this shim a `.vue` default import is `any` (tripping no-unsafe-*).
declare module "*.vue" {
  import type { DefineComponent } from "vue";

  const component: DefineComponent<Record<string, never>, Record<string, never>, unknown>;
  export default component;
}
