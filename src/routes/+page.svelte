<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import {
    Card,
    CardContent,
    CardDescription,
    CardHeader,
    CardTitle,
  } from "$lib/components/ui/card/index.js";

  let name = $state("");
  let greetMsg = $state("");

  async function greet(event: Event) {
    event.preventDefault();
    greetMsg = await invoke("greet", { name });
  }
</script>

<div class="flex min-h-screen items-center justify-center bg-background p-4">
  <Card class="w-full max-w-md">
    <CardHeader class="text-center">
      <div class="flex justify-center gap-4 mb-4">
        <a href="https://vite.dev" target="_blank">
          <img
            src="/vite.svg"
            class="h-14 w-14 transition-all hover:drop-shadow-[0_0_2em_#747bff]"
            alt="Vite Logo"
          />
        </a>
        <a href="https://tauri.app" target="_blank">
          <img
            src="/tauri.svg"
            class="h-14 w-14 transition-all hover:drop-shadow-[0_0_2em_#24c8db]"
            alt="Tauri Logo"
          />
        </a>
        <a href="https://svelte.dev" target="_blank">
          <img
            src="/svelte.svg"
            class="h-14 w-14 transition-all hover:drop-shadow-[0_0_2em_#ff3e00]"
            alt="SvelteKit Logo"
          />
        </a>
      </div>
      <CardTitle class="text-2xl">Welcome to Tauri + Svelte</CardTitle>
      <CardDescription>
        Click on the Tauri, Vite, and SvelteKit logos to learn more.
      </CardDescription>
    </CardHeader>
    <CardContent>
      <form onsubmit={greet} class="flex gap-2">
        <Input
          id="greet-input"
          placeholder="Enter a name..."
          bind:value={name}
          class="flex-1"
        />
        <Button type="submit">Greet</Button>
      </form>
      {#if greetMsg}
        <p class="mt-4 text-center text-sm text-muted-foreground">
          {greetMsg}
        </p>
      {/if}
    </CardContent>
  </Card>
</div>
