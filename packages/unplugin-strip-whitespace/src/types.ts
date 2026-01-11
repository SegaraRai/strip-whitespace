/**
 * Supported languages for whitespace stripping.
 */
export type Language = "astro" | "svelte";

/**
 * Kinds of bundlers where plugin reordering is supported.
 *
 * We don't support webpack and rspack reordering for now, since there seems no way to get plugin names there.
 */
export type ReorderableBundlerKind =
  | "esbuild"
  | "farm"
  | "rolldown"
  | "rollup"
  | "unloader"
  | "vite";

/**
 * A matcher for plugin names, used to identify plugins for reordering.
 */
export type PluginNameMatcher = RegExp | ((name: string) => boolean);

/**
 * Options for the strip-whitespace plugin.
 */
export interface StripWhitespaceOptions {
  /**
   * A list of languages or a function to select which languages to process.
   *
   * If a function is provided, it receives the file ID and content, and should return a language or null to skip.
   * If an array is provided, only files with matching languages will be processed.
   *
   * By default, the plugin processes `.astro` and `.svelte` files excluding those in `node_modules` or with query parameters.
   *
   * @default defaultLanguageSelector
   */
  readonly selectLanguage?:
    | readonly Language[]
    | ((id: string, content: string) => Language | false)
    | undefined;

  /**
   * If true, preserves "section breaks" by skipping gaps that contain an empty line.
   *
   * @default false
   */
  readonly preserveBlankLines?:
    | boolean
    | ((lang: Language, id: string, content: string) => boolean)
    | undefined;

  /**
   * A regular expression to match plugins before which the strip-whitespace plugin should be moved.
   * If set to `false`, the plugin will not be moved.
   * Default is to move before plugins that handle Astro or Svelte files.
   *
   * Availability and behavior of plugin reordering differs across bundlers; in some cases this option may be ignored.
   *
   * When providing an object, keys are bundler kinds and values are matchers specific to that bundler.
   * If a bundler is not specified in the object, the plugin will not be moved for that bundler. (i.e. equivalent to `false`)
   *
   * @default /^astro:build|^vite-plugin-svelte$/
   */
  readonly movePluginBefore?:
    | PluginNameMatcher
    | false
    | Partial<
        Record<ReorderableBundlerKind, PluginNameMatcher | false | undefined>
      >
    | undefined;

  /**
   * If true, skips whitespace stripping if an error occurs during processing.
   * If false, errors will be thrown.
   *
   * @default false
   */
  readonly skipOnError?: boolean | undefined;
}
