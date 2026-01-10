export type Language = "astro" | "svelte";

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
  selectLanguage?:
    | readonly Language[]
    | ((id: string, content: string) => Language | null)
    | undefined;

  /**
   * If true, preserves "section breaks" by skipping gaps that contain an empty line.
   *
   * @default false
   */
  preserveBlankLines?:
    | boolean
    | ((lang: Language, id: string, content: string) => boolean)
    | undefined;

  /**
   * A regular expression to match Vite plugins before which the strip-whitespace plugin should be moved.
   * If set to `false`, the plugin will not be moved.
   * Default is to move before plugins that handle Astro files.
   *
   * @default /^astro:build|^vite-plugin-svelte$/
   */
  viteMovePluginBefore?: RegExp | false | undefined;

  /**
   * If true, skips whitespace stripping if an error occurs during processing.
   * If false, errors will be thrown.
   *
   * @default false
   */
  skipOnError?: boolean | undefined;
}
