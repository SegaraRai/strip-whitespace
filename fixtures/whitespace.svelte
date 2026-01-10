<script>
  const a = "A";
  const b = "B";
</script>

<div>
  <pre>
    Leading spaces should stay.
  </pre>

  <p>
    {a}
    {b}
  </p>

  <p>Inline: {a} {b}</p>

  <p>
    Mixed text and expressions:
    hello {a}
    world {b}
  </p>
</div>
