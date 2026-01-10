<script>
  import Card from "./Card.svelte";

  const items = ["alpha", "beta", "gamma"];
</script>

<div 
 ><Card title="Demo"   

/><ul 
   >{#each items as item 
     }<li class="item">{item.toUpperCase()}</li 
   >{/each 
 }</ul
></div>
