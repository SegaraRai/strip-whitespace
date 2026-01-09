<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  name?: string;
}>();

const displayName = computed(() => props.name ?? "Vue");
</script>

<template>
  <p>
    Hello
    <span>
      {{ name ?? "Vue" }}
    </span>!
  </p>

  <p>
    Vue has own whitespace handling so Astro's strip-whitespace feature should
    not affect it.
  </p>

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
</template>
