<!--
  Mount point of the dev demo's floating Demo button (see `demo/Demo.svelte`). In a
  production build it renders nothing and the demo module is not bundled.
-->
<script lang="ts">
  import type { CameraId } from '../lib/bindings';

  interface Props {
    /** The cameras on screen, top to bottom. */
    cameras: CameraId[];
  }

  let { cameras }: Props = $props();

  // `import.meta.env.DEV` is `false` in production, so the import is dropped there.
  const demo = import.meta.env.DEV ? import('./demo/Demo.svelte') : null;
</script>

{#if demo}
  {#await demo then { default: Demo }}
    <Demo {cameras} />
  {/await}
{/if}
