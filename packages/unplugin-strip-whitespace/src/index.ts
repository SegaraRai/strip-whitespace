import { initWasmOnce, stripWhitespace } from "#wasm";
import {
  createUnplugin,
  type UnpluginFactory,
  type UnpluginOptions,
} from "unplugin";
import { UNPLUGIN_NAME } from "./internal/consts";
import type {
  Language,
  PluginNameMatcher,
  ReorderableBundlerKind,
  StripWhitespaceOptions,
} from "./types";

export type { Language, StripWhitespaceOptions } from "./types";

export function defaultLanguageSelector(
  id: string,
  _content: string,
): Language | false {
  if (
    id.includes("?") ||
    id.includes("node_modules/") ||
    id.includes("node_modules\\")
  ) {
    return false;
  }

  if (id.endsWith(".astro")) {
    return "astro";
  }

  if (id.endsWith(".svelte")) {
    return "svelte";
  }

  return false;
}

function createLanguageSelectorFromArray(languages: readonly Language[]) {
  return (id: string, content: string): Language | false => {
    const lang = defaultLanguageSelector(id, content);
    if (lang && languages.includes(lang)) {
      return lang;
    }
    return false;
  };
}

function resolvePluginMatcher(
  matcher: NonNullable<StripWhitespaceOptions["movePluginBefore"]>,
  bundler: ReorderableBundlerKind,
): PluginNameMatcher | false {
  if (typeof matcher === "object" && !(matcher instanceof RegExp)) {
    return matcher[bundler] ?? false;
  }
  return matcher;
}

function testPluginName(matcher: PluginNameMatcher, name: string): boolean {
  if (matcher instanceof RegExp) {
    return matcher.test(name);
  }

  return matcher(name);
}

function toArray(name: string | readonly string[] | null | undefined) {
  if (name == null) {
    return [];
  }
  return Array.isArray(name) ? name : [name];
}

function reorder<T>(
  array: T[],
  getName: (item: T) => string | readonly string[] | null | undefined,
  itemNameToMove: string,
  matcher: PluginNameMatcher,
): void {
  const itemIndex = array.findIndex((item) =>
    toArray(getName(item)).includes(itemNameToMove),
  );
  if (itemIndex === -1) {
    return;
  }

  const matchIndex = array
    .map((item) => toArray(getName(item)))
    .findIndex((names) => names.some((name) => testPluginName(matcher, name)));
  if (matchIndex === -1) {
    return;
  }

  if (itemIndex < matchIndex) {
    return;
  }

  const [item] = array.splice(itemIndex, 1);
  array.splice(matchIndex, 0, item);
}

export const unpluginFactory: UnpluginFactory<
  StripWhitespaceOptions | undefined
> = (options) => {
  initWasmOnce();

  const {
    selectLanguage: rawSelectLanguage = defaultLanguageSelector,
    preserveBlankLines: rawPreserveBlankLines = false,
    movePluginBefore: rawMovePluginBefore = /^astro:build|^vite-plugin-svelte$/,
    skipOnError = false,
  } = options ?? {};

  const selectLanguage = Array.isArray(rawSelectLanguage)
    ? createLanguageSelectorFromArray(rawSelectLanguage)
    : (rawSelectLanguage as Exclude<typeof rawSelectLanguage, readonly any[]>);

  return {
    name: UNPLUGIN_NAME,
    enforce: "pre",
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
    // Plugin reordering for various bundlers
    esbuild: {
      config(options) {
        const matcher = resolvePluginMatcher(rawMovePluginBefore, "esbuild");
        if (!matcher || !options.plugins) {
          return;
        }
        reorder(
          options.plugins,
          (plugin) => plugin.name,
          UNPLUGIN_NAME,
          matcher,
        );
      },
    },
    farm: {
      configResolved(config) {
        const matcher = resolvePluginMatcher(rawMovePluginBefore, "farm");
        if (!matcher || !config.jsPlugins) {
          return;
        }
        reorder(
          config.jsPlugins,
          (plugin) => plugin.name,
          UNPLUGIN_NAME,
          matcher,
        );
      },
    },
    unloader: {
      options(config) {
        const matcher = resolvePluginMatcher(rawMovePluginBefore, "unloader");
        if (!matcher || !config.plugins) {
          return;
        }
        reorder(
          config.plugins,
          (plugin) => plugin.name,
          UNPLUGIN_NAME,
          matcher,
        );
      },
    },
    rolldown: {
      options(config) {
        const matcher = resolvePluginMatcher(rawMovePluginBefore, "rolldown");
        if (!matcher || !config.plugins || !Array.isArray(config.plugins)) {
          return;
        }
        type PluginWithName = Exclude<
          (typeof config.plugins)[number],
          | Promise<any>
          | any[]
          | { _parallel: any }
          | false
          | void
          | null
          | undefined
        >;
        const flatten = (
          plugin: (typeof config.plugins)[number],
        ): PluginWithName[] => {
          if (!plugin || plugin instanceof Promise || "_parallel" in plugin) {
            return [];
          }
          if (Array.isArray(plugin)) {
            return plugin.flatMap(flatten);
          }
          return [plugin];
        };
        reorder(
          config.plugins,
          (plugin) => flatten(plugin).map((p) => p.name),
          UNPLUGIN_NAME,
          matcher,
        );
      },
    },
    rollup: {
      options(config) {
        const matcher = resolvePluginMatcher(rawMovePluginBefore, "rollup");
        if (!matcher || !config.plugins || !Array.isArray(config.plugins)) {
          return;
        }
        type PluginWithName = Exclude<
          (typeof config.plugins)[number],
          Promise<any> | any[] | false | void | null | undefined
        >;
        const flatten = (
          plugin: (typeof config.plugins)[number],
        ): PluginWithName[] => {
          if (!plugin || plugin instanceof Promise) {
            return [];
          }
          if (Array.isArray(plugin)) {
            return plugin.flatMap(flatten);
          }
          return [plugin];
        };
        reorder(
          config.plugins,
          (plugin) => flatten(plugin).map((p) => p.name),
          UNPLUGIN_NAME,
          matcher,
        );
      },
    },
    vite: {
      configResolved(config) {
        const matcher = resolvePluginMatcher(rawMovePluginBefore, "vite");
        if (!matcher) {
          return;
        }
        reorder(
          config.plugins as (typeof config.plugins)[number][],
          (plugin) => plugin.name,
          UNPLUGIN_NAME,
          matcher,
        );
      },
    },
    // We don't support webpack and rspack reordering for now, since there seems no way to get plugin names there
  } satisfies UnpluginOptions;
};

export const unplugin = /* #__PURE__ */ createUnplugin(unpluginFactory);

export default unplugin;
