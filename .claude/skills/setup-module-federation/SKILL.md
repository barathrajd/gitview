---
name: setup-module-federation
description: Sets up Module Federation (MF 2.0) so apps can share code and load remote modules at runtime — micro-frontends, cross-framework Bridge, shared deps, type hints, and manifests. Picks the right integration for the project's bundler (Vite, Rsbuild/Rstack, Rspack, Webpack, Next.js, Modern.js, Angular). Use when the user says "set up module federation", "share modules between apps", "micro-frontend", "expose/consume a remote", or "mf config".
---

# Setup Module Federation

## Overview
Wire a project as a Module Federation **producer** (exposes modules), **consumer** (loads remotes), or both. MF 2.0 adds Manifest, Federation Runtime, a runtime plugin system, dynamic type hints, and Chrome DevTools on top of the Webpack 5 core. The CLI is `mf`, shipped by `@module-federation/enhanced`; build-plugin config lives in `module-federation.config.ts`. Source of truth: module-federation.io.

## Decide first: build plugin vs runtime-only
- **Build plugin** (most projects) — import remotes like npm packages, get type hints, and (required if you **expose** modules). Config in `module-federation.config.ts`.
- **Runtime only** — you only *consume* remotes at runtime and don't want to change the build. Use `@module-federation/runtime` / `enhanced/runtime` with `registerRemotes` + `loadRemote`.

## Pick the integration for your bundler (hardened reference)
| Bundler / framework | Package |
| --- | --- |
| **Vite** (incl. Vite+) | `@module-federation/vite` |
| **Rsbuild / Rstack** | `@module-federation/rsbuild-plugin` |
| **Rslib** (library producer) | `@module-federation/rsbuild-plugin` |
| **Rstest** (tests load real remotes) | `@module-federation/rstest` |
| **Rspack** | `@module-federation/enhanced` (Rspack plugin) |
| **Webpack** | `@module-federation/enhanced` (Webpack plugin) |
| **Next.js** | `@module-federation/nextjs-mf` |
| **Modern.js** | `@module-federation/modern-js-v3` |
| **Angular** | `@angular-architects/module-federation` (or Nx) |
| **Metro / React Native** | Metro MF |

> For a React + Vite/Vite+ app, use `@module-federation/vite`; add Bridge (`@module-federation/bridge-react`) for app-level/cross-framework loading. Detect the repo shape first (see `references/workspace-detection.md`) — in a monorepo, each producer/consumer is its own package with its own `module-federation.config.ts`.

## Core config options (`module-federation.config.ts`)
| Option | What it does |
| --- | --- |
| `name` | **Required**, unique module name |
| `filename` | remote entry filename (default `remoteEntry.js`) |
| `remotes` | remotes to consume (`alias: 'name@entry'`; entry can be `mf-manifest.json` for type hints + preload + devtools) |
| `exposes` | modules this producer exposes |
| `shared` | shared deps (version reuse, singletons) |
| `manifest` | emit `mf-manifest.json` + `mf-stats.json` |
| `dts` | remote type generation/consumption (default `true`) |
| `shareStrategy` | `version-first` (default) or `loaded-first` |
| `runtimePlugins` | runtime plugin paths (retry, observability, custom) |

## Process

### 1. Install the integration
Add the package for your bundler from the table above (e.g. `pnpm add -D @module-federation/vite`). Confirm the bundler and whether you need SSR before choosing — Next.js/Modern.js/Angular have their own guides.

### 2. Add the plugin and create module-federation.config.ts
Register the plugin in the bundler config, pointing at `module-federation.config.ts`. For copy-paste setup per bundler (Vite, Rsbuild/Rstack, Rspack, Webpack — install + config + plugin registration), see `references/bundler-examples.md`. The shape is:
```ts
import { createModuleFederationConfig } from '@module-federation/enhanced';
export default createModuleFederationConfig({
  name: 'web',
  // consumer:
  remotes: { shop: 'shop@https://cdn.example.com/shop/mf-manifest.json' },
  // producer:
  exposes: { './Button': './src/Button.tsx' },
  shared: { react: { singleton: true }, 'react-dom': { singleton: true } },
  manifest: true,
  dts: true,
});
```
The import path and plugin registration differ by bundler (see the examples file); the config fields are the same everywhere.

### 3. Share dependencies correctly
Mark framework singletons (`react`, `react-dom`, router, state libs) as `singleton: true` to avoid duplicate instances. Use `shareStrategy` and, if needed, multiple share scopes for isolation. Prefer `mf-manifest.json` remote entries for type hints and preloading.

### 4. Cross-framework / app-level loading (Bridge)
When loading a *whole app* (routing + logic), not just a component, use Bridge:
- Producer: `createBridgeComponent` (React: `@module-federation/bridge-react`; Vue3: `@module-federation/bridge-vue3`) to export the app.
- Consumer: `createRemoteAppComponent` to load it; `createLazyComponent` for on-demand components with built-in Suspense/error boundary + prefetch.

### 5. Debugging and resilience
- `FEDERATION_DEBUG=true` on build, or `localStorage.setItem('FEDERATION_DEBUG','true')` in the browser.
- Module Federation Chrome DevTools; Divebell MF Extension for live runtime evidence (Remote/Shared/Bridge/perf).
- Add the **Retry Plugin** and **Observability Plugin** via `runtimePlugins` for production resilience.

### 6. Verify
Producer build emits `remoteEntry.js` + `mf-manifest.json`; consumer loads the remote and (with `dts`) resolves remote types. Run both apps and confirm a remote module renders.

## Rationalizations — and the rebuttals
| Excuse | Rebuttal |
| --- | --- |
| "Share React without `singleton: true`." | Two React copies on one page break hooks. Singleton the framework deps. |
| "Point remotes at `remoteEntry.js`, skip the manifest." | `mf-manifest.json` entries unlock type hints, preloading, and devtools. Prefer them. |
| "Guess the plugin for this bundler." | Each bundler has a specific package. Use the table / module-federation.io, don't invent one. |
| "Load a whole app via a plain remote import." | App-level modules (routing+logic) need Bridge, not a bare component import. |

## Red Flags — stop and fix
- Framework deps shared without `singleton` → duplicate React/runtime errors.
- A producer with no unique `name`, or a name collision across apps.
- Wrong integration package for the bundler.
- No manifest while expecting type hints or devtools support.

## Verification — done when
- The correct bundler integration is installed and registered.
- `module-federation.config.ts` has a unique `name` and the needed `remotes`/`exposes`/`shared`.
- Producer emits `remoteEntry.js` + `mf-manifest.json`; consumer renders a remote module.
- Shared framework deps are singletons; types resolve when `dts` is on.
