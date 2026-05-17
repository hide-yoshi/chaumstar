// Pure SPA: no SSR (we need WASM at runtime). Prerender the static shell only.
export const ssr = false;
export const prerender = true;
export const trailingSlash = 'always';
