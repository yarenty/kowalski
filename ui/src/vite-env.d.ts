/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** Optional full API origin for production (no trailing slash). Dev: leave unset to use Vite `/api` proxy. */
  readonly VITE_API_BASE?: string;
}

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<object, object, unknown>;
  export default component;
}
