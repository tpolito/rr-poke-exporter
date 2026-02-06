import { parseArgs } from "util";
import { readFileSync } from "fs";

// -- Gen 3 character encoding --
// GBA Pokemon games use a custom encoding, not ASCII
const charMap = new Map<number, string>();

function charMapStore(offset: number, symbols: string) {
  for (let i = 0; i < symbols.length; i++) {
    charMap.set(offset + i, symbols[i]);
  }
}

charMapStore(0x00, " ");
charMapStore(0xa1, "0123456789");
charMapStore(0xab, "!?.-");
charMapStore(0xb5, "♂♀");
charMapStore(0xbb, "ABCDEFGHIJKLMNOPQRSTUVWXYZ");
charMapStore(0xd5, "abcdefghijklmnopqrstuvwxyz");

const STRING_TERM = 0xff;

// -- Binary helpers --

function toAscii(bytes: Uint8Array): string {
  let result = "";
  for (const b of bytes) {
    if (b === STRING_TERM) return result;
    result += charMap.get(b) ?? `[0x${b.toString(16).padStart(2, "0")}]`;
  }
  return result;
}

function u16(data: Uint8Array, off: number): number {
  return data[off] | (data[off + 1] << 8);
}

function u32(data: Uint8Array, off: number): number {
  return (
    (data[off] | (data[off + 1] << 8) | (data[off + 2] << 16) | (data[off + 3] << 24)) >>> 0
  );
}

function hexDump(data: Uint8Array, offset: number, length: number): string {
  const bytes = Array.from(data.slice(offset, offset + length));
  return bytes.map((b) => b.toString(16).padStart(2, "0")).join(" ");
}

// -- Gen 3 save structure --
// The save is split into two slots of 14 sections (4KB each).
// Sections can be in any order — the footer identifies which is which.

const SECTION_SIZE = 0x1000;
const SECTION_COUNT = 14;
const SLOT_SIZE = SECTION_SIZE * SECTION_COUNT;
const SIGNATURE = 0x08012025;

interface Section {
  id: number;
  saveIndex: number;
  data: Uint8Array;
}

function parseSaveSlot(raw: Uint8Array, slotOffset: number): Section[] {
  const sections: Section[] = [];
  for (let i = 0; i < SECTION_COUNT; i++) {
    const start = slotOffset + i * SECTION_SIZE;
    const data = raw.slice(start, start + SECTION_SIZE);
    sections.push({
      id: u16(data, 0xff4),
      saveIndex: u32(data, 0xffc),
      data,
    });
  }
  return sections;
}

function getActiveSlot(raw: Uint8Array): Section[] {
  const a = parseSaveSlot(raw, 0);
  const b = parseSaveSlot(raw, SLOT_SIZE);
  const idxA = a[0].saveIndex;
  const idxB = b[0].saveIndex;
  console.log(`Slot A save index: ${idxA}, Slot B save index: ${idxB}`);
  console.log(`Using slot ${idxA >= idxB ? "A" : "B"}\n`);
  return idxA >= idxB ? a : b;
}

function section(sections: Section[], id: number): Uint8Array {
  const s = sections.find((s) => s.id === id);
  if (!s) throw new Error(`Section ${id} not found`);
  return s.data;
}

// -- Pokemon data decryption --
// The 48-byte data block (bytes 32-79) is XOR'd with (personality ^ otId).
// It contains 4 substructures of 12 bytes each, whose order depends on personality % 24.

// prettier-ignore
const SUB_ORDER = [
  [0,1,2,3],[0,1,3,2],[0,2,1,3],[0,2,3,1],[0,3,1,2],[0,3,2,1],
  [1,0,2,3],[1,0,3,2],[1,2,0,3],[1,2,3,0],[1,3,0,2],[1,3,2,0],
  [2,0,1,3],[2,0,3,1],[2,1,0,3],[2,1,3,0],[2,3,0,1],[2,3,1,0],
  [3,0,1,2],[3,0,2,1],[3,1,0,2],[3,1,2,0],[3,2,0,1],[3,2,1,0],
];

function decryptPokemon(raw: Uint8Array): Uint8Array {
  const pokemon = new Uint8Array(raw);
  const key = (u32(pokemon, 0) ^ u32(pokemon, 4)) >>> 0;
  for (let i = 32; i < 80; i += 4) {
    const dec = (u32(pokemon, i) ^ key) >>> 0;
    pokemon[i] = dec & 0xff;
    pokemon[i + 1] = (dec >> 8) & 0xff;
    pokemon[i + 2] = (dec >> 16) & 0xff;
    pokemon[i + 3] = (dec >> 24) & 0xff;
  }
  return pokemon;
}

// subId: 0=Growth, 1=Attacks, 2=EVs, 3=Misc
function substructure(pokemon: Uint8Array, subId: number): Uint8Array {
  const order = SUB_ORDER[u32(pokemon, 0) % 24];
  const pos = order.indexOf(subId);
  const off = 32 + pos * 12;
  return pokemon.slice(off, off + 12);
}

// -- Main --

const { values } = parseArgs({
  args: Bun.argv.slice(2),
  options: { path: { type: "string" } },
});

if (!values.path) {
  console.log("Usage: bun run parse --path <path-to-sav-file>");
  process.exit(1);
}

const raw = new Uint8Array(readFileSync(values.path));
console.log(`Loaded ${values.path} (${raw.length} bytes)\n`);

const sections = getActiveSlot(raw);

// Verify section signatures
console.log("=== Sections ===");
for (const s of sections) {
  const sig = u32(s.data, 0xff8);
  console.log(`  Section ${String(s.id).padStart(2)}: signature ${sig === SIGNATURE ? "OK" : "BAD"}`);
}
console.log();

// Section 0: Trainer info
const sec0 = section(sections, 0);
console.log("=== Trainer ===");
console.log(`  Name:       ${toAscii(sec0.slice(0x00, 0x07))}`);
console.log(`  Gender:     ${sec0[0x08] === 0 ? "Boy" : "Girl"}`);
console.log(`  Trainer ID: ${u16(sec0, 0x0a)}`);
console.log(`  Secret ID:  ${u16(sec0, 0x0c)}`);
console.log(`  Name bytes: ${hexDump(sec0, 0x00, 8)}`);
console.log();

// Section 1: Party data (FireRed offsets)
const sec1 = section(sections, 1);
const partySize = u32(sec1, 0x0034);

console.log(`=== Party (${partySize} Pokemon) ===`);
for (let i = 0; i < partySize && i < 6; i++) {
  const off = 0x0038 + i * 100;
  const pkmn = decryptPokemon(sec1.slice(off, off + 100));

  const nickname = toAscii(pkmn.slice(8, 18));
  const otName = toAscii(pkmn.slice(20, 27));
  const level = pkmn[84];
  const hp = u16(pkmn, 86);
  const maxHp = u16(pkmn, 88);

  const growth = substructure(pkmn, 0);
  const speciesId = u16(growth, 0);
  const exp = u32(growth, 4);

  const attacks = substructure(pkmn, 1);
  const moves = [u16(attacks, 0), u16(attacks, 2), u16(attacks, 4), u16(attacks, 6)].filter(
    (m) => m !== 0,
  );

  const evBlock = substructure(pkmn, 2);

  console.log(`  --- ${nickname} (Species #${speciesId}) ---`);
  console.log(`    OT: ${otName} | Level: ${level} | HP: ${hp}/${maxHp}`);
  console.log(
    `    Stats: ATK=${u16(pkmn, 90)} DEF=${u16(pkmn, 92)} SPD=${u16(pkmn, 94)} SPATK=${u16(pkmn, 96)} SPDEF=${u16(pkmn, 98)}`,
  );
  console.log(`    EXP: ${exp} | Move IDs: [${moves.join(", ")}]`);
  console.log(
    `    EVs: HP=${evBlock[0]} ATK=${evBlock[1]} DEF=${evBlock[2]} SPD=${evBlock[3]} SPATK=${evBlock[4]} SPDEF=${evBlock[5]}`,
  );
  console.log(`    Personality: ${u32(pkmn, 0)}`);
  console.log();
}
