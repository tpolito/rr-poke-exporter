<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";

  interface Pokemon {
    nickname: string;
    species: string;
    level: number;
    item: string | null;
    nature: string;
    ability: string;
    moves: string[];
    display_text: string;
  }

  let party: Pokemon[] = $state([]);
  let error = $state("");
  let loading = $state(false);
  let copied = $state(false);
  let currentPath = $state("");

  async function loadSav(path: string) {
    loading = true;
    error = "";
    try {
      party = await invoke<Pokemon[]>("parse_sav_file", { path });
      currentPath = path;
    } catch (e) {
      error = String(e);
      party = [];
    } finally {
      loading = false;
    }
  }

  async function pickFile() {
    const path = await open({
      filters: [{ name: "Save File", extensions: ["sav"] }],
      multiple: false,
      directory: false,
    });
    if (path) {
      await loadSav(path);
    }
  }

  async function copyAll() {
    if (!currentPath || loading) return;

    loading = true;
    error = "";
    try {
      // Re-load from disk every time you copy, so the clipboard always reflects
      // the latest state of the save file without re-opening the file dialog.
      const latestParty = await invoke<Pokemon[]>("parse_sav_file", {
        path: currentPath,
      });
      party = latestParty;

      const text = latestParty.map((p) => p.display_text).join("\n\n");
      await navigator.clipboard.writeText(text);

      copied = true;
      setTimeout(() => (copied = false), 2000);
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  onMount(async () => {
    try {
      const saved = await invoke<string | null>("get_saved_path");
      if (saved) {
        await loadSav(saved);
      }
    } catch (_) {
      // No saved path, that's fine
    }
  });
</script>

<main>
  <h1>RR Poke Exporter</h1>

  <div class="controls">
    <button onclick={pickFile} disabled={loading}>
      {loading ? "Loading..." : "Select .sav File"}
    </button>
    {#if party.length > 0}
      <button
        onclick={copyAll}
        class="copy-btn"
        disabled={loading || !currentPath}
      >
        {loading ? "Refreshing..." : copied ? "Copied!" : "Copy All"}
      </button>
    {/if}
  </div>

  {#if currentPath}
    <p class="path">{currentPath}</p>
  {/if}

  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#if party.length > 0}
    <div class="party">
      {#each party as mon}
        <div class="card">
          <pre>{mon.display_text}</pre>
        </div>
      {/each}
    </div>
  {/if}
</main>

<style>
  :root {
    font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
    font-size: 16px;
    line-height: 1.5;
    color: #f6f6f6;
    background-color: #1a1a2e;
  }

  main {
    max-width: 800px;
    margin: 0 auto;
    padding: 2rem;
  }

  h1 {
    text-align: center;
    color: #e94560;
    margin-bottom: 1.5rem;
  }

  .controls {
    display: flex;
    justify-content: center;
    gap: 1rem;
    margin-bottom: 1rem;
  }

  button {
    padding: 0.6rem 1.4rem;
    font-size: 1rem;
    font-weight: 600;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    background-color: #e94560;
    color: #fff;
    transition: background-color 0.2s;
  }

  button:hover:not(:disabled) {
    background-color: #c73650;
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .copy-btn {
    background-color: #0f3460;
  }

  .copy-btn:hover {
    background-color: #16213e;
  }

  .path {
    text-align: center;
    font-size: 0.8rem;
    color: #888;
    word-break: break-all;
    margin-bottom: 1rem;
  }

  .error {
    text-align: center;
    color: #e94560;
    background: #2a1a2e;
    padding: 0.8rem;
    border-radius: 8px;
    margin-bottom: 1rem;
  }

  .party {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 1rem;
  }

  .card {
    background: #16213e;
    border: 1px solid #0f3460;
    border-radius: 8px;
    padding: 1rem;
  }

  .card pre {
    margin: 0;
    white-space: pre-wrap;
    font-family: "Courier New", Courier, monospace;
    font-size: 0.9rem;
    line-height: 1.6;
  }
</style>
