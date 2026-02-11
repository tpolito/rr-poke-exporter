# Radical Red Poke Exporter

A desktop app that reads Radical Red `.sav` files and exports your party Pokemon in Showdown-compatible text format. Load your save, see your team, and copy it straight into Pokemon Showdown.

## How It Works

1. Open the app and select your Radical Red `.sav` file
2. Your party of up to 6 Pokemon is parsed and displayed
3. Copy individual Pokemon or your full team in Showdown format

## Built With

- **Tauri 2** (Rust backend + SvelteKit frontend)
- Rust handles binary `.sav` parsing; the frontend displays the results
