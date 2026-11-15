<!--
  A card image from the local cache (`cardimg://`). Until the image loads, or when it
  cannot (the protocol is not served before ticket I1), it is a neutral card-shaped
  placeholder.
-->
<script lang="ts">
  import type { ImageSize, Passcode } from '../lib/bindings';
  import { cardImageUrl } from '../lib/card-image';

  interface Props {
    passcode: Passcode;
    size: ImageSize;
    alt: string;
  }

  let { passcode, size, alt }: Props = $props();

  const src = $derived(cardImageUrl(passcode, size));
  let failedSrc = $state<string | null>(null);
</script>

{#if failedSrc === src}
  <div class="card-placeholder" role="img" aria-label={alt}></div>
{:else}
  <img
    class="card-image"
    {src}
    {alt}
    draggable="false"
    onerror={() => {
      failedSrc = src;
    }}
  />
{/if}

<style>
  .card-image,
  .card-placeholder {
    display: block;
    width: 100%;
    aspect-ratio: 59 / 86;
    border-radius: 3% / 2%;
  }

  .card-image {
    object-fit: cover;
    background: var(--color-surface-raised);
  }

  .card-placeholder {
    background:
      linear-gradient(135deg, transparent 0 46%, var(--color-border) 46% 54%, transparent 54%),
      var(--color-surface-raised);
    border: 1px solid var(--color-border);
  }
</style>
