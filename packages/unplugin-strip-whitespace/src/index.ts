import { initWasmOnce, stripWhitespace } from "#wasm";
import {
  createUnplugin,
  type UnpluginFactory,
  type UnpluginOptions,
} from "unplugin";
import { UNPLUGIN_NAME } from "./internal/consts";
import type { Language, StripWhitespaceOptions } from "./types";

export type { Language, StripWhitespaceOptions } from "./types";

export function defaultLanguageSelector(
  id: string,
  _content: string,
): Language | null {
  if (
    id.includes("?") ||
    id.includes("node_modules/") ||
    id.includes("node_modules\\")
  ) {
    return null;
  }

  if (id.endsWith(".astro")) {
    return "astro";
  }

  if (id.endsWith(".svelte")) {
    return "svelte";
  }

  return null;
}

function createLanguageSelectorFromArray(languages: readonly Language[]) {
  return (id: string, content: string): Language | null => {
    const lang = defaultLanguageSelector(id, content);
    if (lang && languages.includes(lang)) {
      return lang;
    }
    return null;
  };
}

export const unpluginFactory: UnpluginFactory<
  StripWhitespaceOptions | undefined
> = (options) => {
  initWasmOnce();

  const {
    selectLanguage: rawSelectLanguage = defaultLanguageSelector,
    preserveBlankLines: rawPreserveBlankLines = false,
    viteMovePluginBefore = /^astro:build|^vite-plugin-svelte$/,
    skipOnError = false,
  } = options ?? {};

  const selectLanguage = Array.isArray(rawSelectLanguage)
    ? createLanguageSelectorFromArray(rawSelectLanguage)
    : (rawSelectLanguage as Exclude<typeof rawSelectLanguage, readonly any[]>);

  return {
    name: UNPLUGIN_NAME,
    transform: {
      // Apply default filter only if using default language selector
      filter:
        selectLanguage === defaultLanguageSelector
          ? {
              id: {
                include: [/\.astro$/, /\.svelte$/],
                exclude: [/\?/],
              },
            }
          : undefined,
      async handler(code, id) {
        const language = selectLanguage(id, code);
        if (!language) {
          return null;
        }

        const preserveBlankLines =
          typeof rawPreserveBlankLines === "function"
            ? rawPreserveBlankLines(language, id, code)
            : rawPreserveBlankLines;

        try {
          const result = stripWhitespace(code, id, language, {
            preserveBlankLines,
          });
          if (result.code === code) {
            // No changes
            return null;
          }

          return result;
        } catch (err) {
          if (skipOnError) {
            this.warn(`failed to process ${id}, skipping. Error: ${err}`);
            return null;
          }

          this.error(`failed to process ${id}. Error: ${err}`);
          throw err;
        }
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

        if (thisPluginIndex < pluginIndex) {
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
