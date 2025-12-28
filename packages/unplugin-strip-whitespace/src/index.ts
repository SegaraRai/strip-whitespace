import { initWasmOnce, strip_whitespace, type StripConfig } from "#wasm";
import {
  createUnplugin,
  type UnpluginFactory,
  type UnpluginOptions,
} from "unplugin";
import { UNPLUGIN_NAME } from "./internal/consts";
import type { StripWhitespaceOptions } from "./types";

export type { StripWhitespaceOptions } from "./types";

export const unpluginFactory: UnpluginFactory<
  StripWhitespaceOptions | undefined
> = (options) => {
  initWasmOnce();

  const stripOptions: StripConfig = {
    preserve_blank_lines: options?.preserveBlankLines ?? false,
  };

  const viteMovePluginBefore = options?.viteMovePluginBefore ?? /^astro:build/;

  return {
    name: UNPLUGIN_NAME,
    transform: {
      filter: {
        id: {
          include: [/\.astro$/],
          exclude: [/\?/],
        },
      },
      async handler(code, id) {
        if (!id.endsWith(".astro") || id.includes("?")) {
          return null;
        }

        const result = strip_whitespace(code, id, stripOptions);
        if (result.code === code) {
          // No changes
          return null;
        }

        return result;
      },
    },
    vite: {
      configResolved(config) {
        if (viteMovePluginBefore === false) {
          return;
        }

        const pluginIndex = config.plugins.findIndex((plugin) =>
          viteMovePluginBefore.test(plugin.name),
        );
        if (pluginIndex === -1) {
          return;
        }

        const thisPluginIndex = config.plugins.findIndex(
          (plugin) => plugin.name === UNPLUGIN_NAME,
        );
        if (thisPluginIndex === -1) {
          return;
        }

        const [thisPlugin] = (
          config.plugins as (typeof config.plugins)[number][]
        ).splice(thisPluginIndex, 1);
        (config.plugins as (typeof config.plugins)[number][]).splice(
          pluginIndex,
          0,
          thisPlugin,
        );
      },
    },
  } satisfies UnpluginOptions;
};

export const unplugin = /* #__PURE__ */ createUnplugin(unpluginFactory);

export default unplugin;
