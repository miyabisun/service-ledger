<script lang="ts">
  import Header from "./lib/Header.svelte";
  import { initRouter, router } from "./lib/router.svelte";
  import Edit from "./pages/Edit.svelte";
  import Home from "./pages/Home.svelte";

  $effect(() => initRouter());
</script>

<svelte:head>
  <title>サービス台帳</title>
</svelte:head>

<Header />

<main>
  {#if router.index === 1}
    {#key router.params.id}
      <Edit id={router.params.id} />
    {/key}
  {:else}
    <Home />
  {/if}
</main>
