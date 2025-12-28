<script lang="ts">
  export let name = "Svelte";
</script>

<p>
  Hello <span>{name}</span>!
</p>

<p>
  Svelte has own whitespace handling so Astro's strip-whitespace feature should
  not affect it.
</p>

<!-- prettier-ignore -->
<p>
  Consecutive
    spaces
      and
        new
          lines
            should
              be
                preserved.
</p>
