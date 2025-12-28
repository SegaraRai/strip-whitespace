import { unplugin, type StripWhitespaceOptions } from ".";

export default function astroStripWhitespace(
  options: StripWhitespaceOptions
): any {
  return {
    name: "astro-strip-whitespace",
    hooks: {
      "astro:config:setup": async (astro: any) => {
        astro.config.vite.plugins ||= [];
        astro.config.vite.plugins.push(unplugin.vite(options));
      },
    },
  };
}
