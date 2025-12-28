/**
 * Options for the strip-whitespace plugin.
 */
export interface StripWhitespaceOptions {
  /**
   * If true, preserves "section breaks" by skipping gaps that contain an empty line.
   */
  preserveBlankLines?: boolean;

  /**
   * A regular expression to match Vite plugins before which the strip-whitespace plugin should be moved.
   * If set to `false`, the plugin will not be moved.
   * Default is to move before plugins that handle HTML or Astro files.
   *
   * @default /^astro:build/
   */
  viteMovePluginBefore?: RegExp | false;
}
