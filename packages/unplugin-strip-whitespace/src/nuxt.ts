import { addVitePlugin, addWebpackPlugin, defineNuxtModule } from "@nuxt/kit";
import type { StripWhitespaceOptions } from "./types";
import vite from "./vite";
import webpack from "./webpack";

export interface ModuleOptions extends StripWhitespaceOptions {}

export default defineNuxtModule<ModuleOptions>({
  meta: {
    name: "nuxt-strip-whitespace",
    configKey: "unpluginStripWhitespace",
  },
  defaults: {
    // ...default options
  },
  setup(options, _nuxt) {
    addVitePlugin(() => vite(options), { prepend: true });
    addWebpackPlugin(() => webpack(options), { prepend: true });
  },
});
