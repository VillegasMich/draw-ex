<!--
  Mount point, in the empty camera area, of the stand-in table the dev demo draws when
  no camera is set up (see `demo/StandIn.svelte`). In a production build it renders
  nothing and the demo module is not bundled.
-->
<script lang="ts">
  // `import.meta.env.DEV` is `false` in production, so the import is dropped there.
  const standIn = import.meta.env.DEV ? import('./demo/StandIn.svelte') : null;
</script>

{#if standIn}
  {#await standIn then { default: StandIn }}
    <StandIn />
  {/await}
{/if}
