import { defineConfig } from "astro/config";
import tailwindcss from "@tailwindcss/vite";

const defaultPagesDomain = process.env.GITVIEW_DEFAULT_PAGES_DOMAIN === "true";

export default defineConfig({
  site: defaultPagesDomain
    ? "https://barathrajd.github.io"
    : "https://gitview.barathrajdevarajan.dev",
  base: defaultPagesDomain ? "/gitview" : "/",
  output: "static",
  vite: {
    plugins: [tailwindcss()],
  },
});
