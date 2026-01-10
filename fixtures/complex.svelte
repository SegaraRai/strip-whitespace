<script>
  const show = true;

  const groups = [
    { label: "One", children: ["a", "b"] },
    { label: "Two", children: ["c", "d"] },
  ];

  // Dummy HTML-looking strings in script (should not be treated as markup)
  const dummyHtmlTight = "<div><span>no-space</span><span>tight</span></div>";
  const dummyHtmlSpaced = "<div> <span> spaced </span> <span> out </span> </div>";
  const dummyHtmlTemplate = `
<section>
  <p>line 1</p><p>line 2</p>
  <div>  <span>  inner  </span> </div>
</section>
`;
  const dummyHtmlWithBraces = "<p>{notAnExpression}</p>";
  const dummyHtmlWithScript = "<script>const s = '</div>';</script>";

  const join = (a, b) => `${a}${b}`;
</script>

<div id="tight"><span>A</span><span>B</span><span>{1 + 2}</span></div>

<div id="spaced">
  <span> A </span>
  <span> B </span>
</div>

<div id="mixed">
  <span>before</span>{" "}<span>after</span>
  <span>{join("x", "y")}</span>
</div>

<div id="comments">
  <!-- HTML comment should be preserved as a node -->
  <span>left</span>{#if show}<span>mid</span>{/if}<span>right</span>
</div>

<p id="stringy">
  {"<span>this is a string, not a tag</span>"}
</p>

<ul id="nested">
  {#each groups as group}
    <li>
      <strong>{group.label}</strong>

      <ul>
        {#each group.children as child}
          <li>
            {#if show}
              <span class="child">{child}</span>
              <span class="sep">|</span>
              {#if child === "b"}
                <em>last</em>
              {:else}
                <b>mid</b>
              {/if}
            {:else}
              <span>hidden</span>
            {/if}
          </li>
        {/each}
      </ul>
    </li>
  {/each}
</ul>
