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
    We currently DON'T support stripping whitespace in Vue components.
  </p>

  <p>
    Vue
      does
        not
          preserve
            consecutive
              spaces
                in
                  text
                    nodes.
  </p>
</template>
