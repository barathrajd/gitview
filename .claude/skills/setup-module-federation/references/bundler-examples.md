# MFE Integrations with Bundlers — Examples

Verified from module-federation.io. Each shows installing the integration, the config, and registering the plugin. Config fields are the same across bundlers (see Configuration Overview); only the package and registration differ.

---

## Vite (and Vite+)
Package: `@module-federation/vite`. **Caveat:** every config option is supported *except* `dev`. Build target must be ≥ `chrome89` (or add `vite-plugin-top-level-await` for lower).

```sh
pnpm add @module-federation/vite --save
```

```ts
// module-federation.config.ts
import { createModuleFederationConfig } from '@module-federation/vite';

export default createModuleFederationConfig({
  name: 'vite_provider',
  manifest: true,
  remotes: {
    // object form (ESM remote)
    esm_remote: { type: 'module', name: 'esm_remote', entry: 'https://.../remoteEntry.js' },
    // string form
    var_remote: 'var_remote@https://.../remoteEntry.js',
  },
  exposes: { './button': './src/components/button' },
  shared: {
    react: { singleton: true },
    'react/': { singleton: true },
  },
});
```

```ts
// vite.config.ts  (or inside your Vite+ config)
import { federation } from '@module-federation/vite';
import mfConfig from './module-federation.config';

export default {
  server: { origin: 'http://localhost:2000', port: 2000 },
  base: 'http://localhost:2000',
  plugins: [federation(mfConfig)],
  build: { target: 'chrome89' },
};
```

---

## Rsbuild / Rstack
Package: `@module-federation/rsbuild-plugin`. In Rstack, this applies through the Rsbuild layer (`define.app()` / the underlying `rsbuild.config.ts`). For runtime capabilities also install `@module-federation/enhanced`.

```sh
pnpm add @module-federation/rsbuild-plugin --save-dev
```

```ts
// rsbuild.config.ts
import { pluginModuleFederation } from '@module-federation/rsbuild-plugin';
import { defineConfig } from '@rsbuild/core';
import { pluginReact } from '@rsbuild/plugin-react';

export default defineConfig({
  server: { port: 2000 },
  plugins: [
    pluginReact(),
    pluginModuleFederation({
      name: 'federation_consumer',
      remotes: { remote1: 'remote1@http://localhost:2001/mf-manifest.json' },
      shared: ['react', 'react-dom'],
    }),
  ],
});
```

**SSR:** pass a second options arg — `target: 'web' | 'node' | 'dual'` (`dual` builds browser + Node; `node` + `environment` targets one Rsbuild environment). `ssr: true` is deprecated — use `target: 'dual'`.

---

## Rspack
Package: `@module-federation/enhanced` (import from `/rspack`).

```sh
pnpm add @module-federation/enhanced
```

```ts
// module-federation.config.ts
import { createModuleFederationConfig } from '@module-federation/enhanced/rspack';

export default createModuleFederationConfig({
  name: 'host',
  remotes: { provider: 'provider@http://localhost:2004/mf-manifest.json' },
  exposes: { './Button': './src/components/Button.tsx' },
  shared: { react: { singleton: true }, 'react-dom': { singleton: true } },
});
```

```ts
// rspack.config.ts
import { ModuleFederationPlugin } from '@module-federation/enhanced/rspack';
import mfConfig from './module-federation.config';

export default defineConfig({
  plugins: [new ModuleFederationPlugin(mfConfig)],
});
```

---

## Webpack
Package: `@module-federation/enhanced` (import from `/webpack`). Config is a plain object.

```sh
pnpm add @module-federation/enhanced
```

```js
// module-federation.config.js
module.exports = {
  name: 'host',
  remotes: { provider: 'provider@http://localhost:2004/mf-manifest.json' },
  exposes: { './Button': './src/components/Button.tsx' },
  shared: { react: { singleton: true }, 'react-dom': { singleton: true } },
};
```

```js
// webpack.config.js
const { ModuleFederationPlugin } = require('@module-federation/enhanced/webpack');
const mfConfig = require('./module-federation.config');

module.exports = {
  devServer: { port: 2000 },
  output: { publicPath: 'http://localhost:2000/' },
  plugins: [new ModuleFederationPlugin(mfConfig)],
};
```

---

## Cross-bundler notes
- **Remote entry:** prefer `mf-manifest.json` entries over `remoteEntry.js` — they add type hints, resource preloading, and DevTools support.
- **Shared singletons:** always `singleton: true` for `react` / `react-dom` (and router/state libs) to avoid duplicate instances.
- **Config fields are shared** across all bundlers — `name`, `filename`, `remotes`, `exposes`, `shared`, `manifest`, `dts`, `shareStrategy`, `runtimePlugins`. Only the import path and plugin registration differ.
- **Other hosts:** Next.js → `@module-federation/nextjs-mf`; Modern.js → `@module-federation/modern-js-v3`; Angular → `@angular-architects/module-federation`; Metro/RN → Metro MF. Check their guide pages for host-specific wiring.
