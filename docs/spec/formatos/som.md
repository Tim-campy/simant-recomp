*Leia em português: [som.md](../../pt-BR/spec/formatos/som.md)*

# Sound and music: files and formats

Where SimAnt for Windows keeps its sounds and music, in what format, and along which path the executable plays them. When each one plays is in [regras/som.md](../regras/som.md).

## Summary

| Content | Where | Format | Used by |
|---|---|---|---|
| Sound effects | `SOUND` database, type **5**, ids 0–56 | 8-bit samples compressed as 4-bit deltas ("DAC instrument") | `_myBeginSound` with MMSYSTEM (`waveOutWrite`) or Sound Blaster (`vocPlayMemUnFormat`) |
| Sound effects (Mac version) | `SOUND` database, type **51**, 34 ids between 1 and 49 | Macintosh `snd ` resource (format 1 or 2) | `_myBeginSound` only with `DSOUND.DLL` (Disney Sound Source) |
| Music (standard Windows) | `SOUND\*.MID` files in the game folder (29 files) | standard MIDI (SMF format 1), Windows 3.1 dual arrangement | `_myBeginSong` via MCI (`mciSendCommand`) |
| Music headers | `SOUND` database, type **18**, 33 ids (10001–20011) | 18 bytes, 9 big-endian words | `_myBeginSong` (all paths) |
| Music (Sound Blaster) | `SOUND` database, type **20**, 30 ids | standard MIDI, a single arrangement (channels 2–4 and 10) | `_myBeginSong` via `SNDBLST.DLL` (`musPlayMemMidi`) |
| Music (PC speaker) | `SOUND` database, type **50**, 28 ids | standard MIDI reduced to one voice | `_myBeginSong` via `SOUND.DRV` (`SetVoiceNote`) |

**Source:** verified fact with `tools/db_list.py` and object dumps (counts, sizes, `MThd` signatures); playback paths seen in the code (`GR_MODULE`, segment `1008`), addresses below.

## Audio output chosen by the original

`_snd_Install` (`1008:793a`), called once at startup right after `_db_SetDataBase("sound")`, tests the libraries in this order (fact seen in the code; DLL and function names are DGROUP strings, offsets `0xafa`–`0xbcf`):

1. `MMSYSTEM.DLL` (searched for in the Windows folder, in `SYSTEM\` and in the game folder by `_IsDLLAvail` `1008:780e`). If `midiOutGetNumDevs()` > 0: **multimedia mode** (`[0x8d0a]` = 1). Music through MCI, effects through `waveOut*`.
2. Otherwise, `SNDBLST.DLL` with `sbcGetDLLVersion` and `sbcGetCardName`: **Sound Blaster mode** (`[0x8d0a]` = 0). Music through `musOpenDevice`/`musPlayMemMidi`, effects through `vocOpenDevice`/`vocPlayMemUnFormat`.
3. Otherwise, `MMSYSTEM.DLL` again, now only with `waveOutGetNumDevs()` > 0: effects through `waveOut*`, music through the PC speaker (`[0x8d0c]` = 1).
4. With nothing: music through the Windows 3.0 `SOUND.DRV` (imported by ordinal: 1 `OpenSound`, 2 `CloseSound`, 3 `SetVoiceQueueSize`, 4 `SetVoiceNote`, 9 `StartSound`) and effects turned off.

In addition, with MMSYSTEM and without `waveOut` [hypothesis: the exact condition was not followed], it tries `DSOUND.DLL` (`GetDSoundVersion` ≥ 2) and, if present, the effects use the type 51 objects through `PlaySound`.

On an ordinary Windows 3.1 installation with a sound card (including the reference one, with a Sound Blaster 16 in DOSBox-X), the path is **1**: MIDI through MCI from `SOUND\*.MID` and type 5 effects through `waveOut`. **The port reproduces this path.** Types 20 and 51 remain as alternative data; type 50 is not used.

### `SIMANT.CFG` does not choose the output

`_ReadConfig` (`1008:08f4`) stores the "Sound Mode" digit in `_musicDevice`, but no function reads `_musicDevice` in the pseudocode: all three references are writes (`_IBMInitStuff` `1008:0220` sets −1 or a digit; `_ReadConfig` sets the digit from the file). `_snd_Install` does not consult the variable, and `_MusicInit` (`1008:827c`) only writes "MUSIC INIT!" to the debug log. Conclusion: on Windows, "Sound Mode" has no effect (seen in the code; an indirect read that Ghidra did not resolve is not ruled out). See [simant-cfg.md](simant-cfg.md).

## Type 5: sound effect ("DAC instrument")

57 objects, ids 0–56, all with flags 8 (no LZSS compression). Size from 67 to 6464 bytes.

| Offset | Size | Field |
|---|---|---|
| 0 | 16 bytes | delta table: 16 **signed** values (8 bits), indexed by a nibble |
| 16 | N − 16 bytes | data: each byte holds two nibbles, first the **high** one, then the low one |

Decoding (fact seen in the code, three identical copies: `_UncompressDACInstrument` `1008:768c`, `_MciOutWave` `1008:959c` and the `sndPlaySound` path of `_myBeginSound`):

```
sample = 0x80
for each data byte b:
    sample = (sample + table[b >> 4])  mod 256 ; emit sample
    sample = (sample + table[b & 0xF]) mod 256 ; emit sample
```

- Result: **unsigned 8-bit** PCM (0x80 = silence), mono, **2 × (N − 16)** samples.
- The addition is 8-bit with overflow (`char` arithmetic), without saturation.
- **Rate:** the `WAVEFORMAT` built for `waveOutOpen` has `nSamplesPerSec` = `nAvgBytesPerSec` = **0x1000 = 4096 Hz**, `nChannels` = 1, `nBlockAlign` = 1, `wBitsPerSample` = 8 (fact seen in the disassembly: `mov word [bx+4], 0x1000` at `1008:9d7e`). The Sound Blaster path also passes 0x1000 to `vocPlayMemUnFormat` (`1008:9b01`) and the `sndPlaySound` path builds a RIFF header with the same values.
- **Actual rate on the Sound Blaster 16 (measured on 2026-10-19):** `sb16snd.drv` plays these bytes at **5000 Hz** (the card's minimum), without resampling, and only in **whole blocks of 504 samples**: the rest of the last block is lost and an effect with fewer than 504 samples (15, 17, 18, 19, 28, 29, 40, 47 and others) makes no sound. Measured in a session played in DOSBox-X and in a controlled test with a 4096 Hz WAV through the same driver (at 5000 Hz the same effect comes out whole); see [regras/som.md](../regras/som.md), "In-game effects, recorded in a session played by the user". The port follows the SB16: `taxa_efeitos` = 5000 and `bloco_efeitos` = 504 in `data/som.json` (4096 and 0 give what the code asks for).
- Note: comparing with the Mac versions (type 51), the sample count of several sounds matches a rate of ~5563 Hz (e.g. id 41 has 12 896 samples here and 13 184 in the `snd ` at 5563.6 Hz; id 15 has 102 samples versus 205 at 11 127 Hz). In other words, the Windows code asks for the effects about 26 % slower and lower-pitched than the Mac; on the SB16 (5000 Hz) they are ~10 % slower. The port follows the measured SB16 (5000 Hz) by default; the rate is in `data/som.json`.

## Type 51: sound effect in Mac format

34 objects (ids 1–49 with gaps), with and without LZSS. They are Macintosh `snd ` resources (verified fact from the dump; Apple's format, *Inside Macintosh: Sound*):

- format 1: `0001` (format), `0001` (one synthesizer), `0005` (sampled synth) + 4 initialization bytes, `0001` command, `8051` (`bufferCmd` with the offset bit), parameters `0000` and `00000014` → sound header at offset 20;
- format 2 (only id 23): `0002`, `0000` (references), `0001` command, `8050`... → header at offset 14;
- sound header (big-endian): pointer (0), number of samples (u32), rate in 16.16 fixed point (e.g. `15bba2e8` = 5563.6 Hz, `2b7745d2` = 11 127 Hz, `56ee8ba3` = 22 254.5 Hz), loop start and end, encoding (0), base note (`3c`), followed by the unsigned 8-bit samples.

Only the `DSOUND.DLL` variant uses these objects. The port does not play them [open: offer as a "Mac sounds" option].

## Type 18: music header

33 objects of 18 bytes = 9 **big-endian** words (data from the Mac; `_FlipWord` in `_myBeginSong`).

| Word | Field | Evidence |
|---|---|---|
| 0 | MIDI object id (types 20/50) | fact seen in the code: read in `_myBeginSong` and passed to `_db_LoadObject(x, 0x14)` / `(x, 0x32)`; the existing values match the ids of types 20 and 50 |
| 1 | 4 in the long themes (10001–10010), 1 in the others | [meaning unknown] |
| 2 | always 0 | — |
| 3 | **transposition in semitones** (0, 6 or 12) | fact seen in the code: passed to `musTransposeNote` in Sound Blaster mode; the log uses the text `"myBeginSong(shift=%d)..."` |
| 4, 5 | pairs (8,3), (5,5), (6,4), (4,2) | [meaning unknown] |
| 6 | 0x4410, 0x410C, 0x400C, 0x4010, 0x4008 | [meaning unknown] |
| 7, 8 | 0 | — |

A song is requested by the **header id**. Several headers point to the same MIDI with different transpositions: 11014 = 11013 + 6 semitones, 11015 = 11013 + 12, 11017 = 11016 + 12. There is no header for 20006 (`jolly5`), 11008–11012 or 11019.

If the header does not exist, `_myBeginSong` returns without playing anything, in any mode (fact seen in the code: null `_db_LoadObject(id, 0x12, 1)` → return).

## Music through MCI: `SOUND\*.MID` and the name table

In multimedia mode, `_myBeginSong(id)` looks up `id` in a table of 31 (name, id) pairs terminated by −1 and plays the file `<executable folder>sound\<name>.mid` (format `"%ssound\%s.mid"`, DGROUP `0xe3c`) with `mciSendCommand` (open with the *MIDI Mapper*, `MCI_SEEK` to the start, `MCI_PLAY` with notification). The id is **not** used to choose the type 20 object nor the header's transposition: in MCI mode word 3 is ignored.

The table lives in DGROUP: far pointers to the names at `[0x8d2a]` (31 × 4 bytes) and the ids at `[0x8da6]` (31 words + −1). In the executable file this area is zeroed; the values below were read from the memory of the running game (verified fact in the RAM dump `re_trabalho/qemu_ram_quick2.bin`, QEMU bench; the names are the DGROUP strings `0xd05`–`0xdee`):

| Id | File | | Id | File |
|---|---|---|---|---|
| 10001 | gamethme | | 11004 | newcoln2 |
| 10003 | antthme1 | | 11005 | newbody |
| 10004 | antthme2 | | 11006 | qn4day |
| 10005 | antthme3 | | 11007 | badnews |
| 10006 | hmanawar | | 11013 | call2arm |
| 10007 | hmanthme | | 11014 | call2arm |
| 10008 | victory | | 11016 | victqndd |
| 10010 | defeat | | 11017 | victqndd |
| 20000 | happy1 | | 11018 | attack |
| 20001 | happy2 | | 11020 | blackq |
| 20002–20006 | jolly1–jolly5 | | 11021 | redq |
| 20007 | omnious1 | | 20008–20011 | sad1–sad4 |

Consequences (fact seen in the code, to be confirmed by listening to the original):

- 11015 has a header but is not in the table: it **does not play** in MCI mode (`_myBeginSong` returns when the lookup reaches the −1). Likewise 11002 and 11003.
- 20006 (`jolly5`) is in the table but has no header: it also **does not play**. The file `JOLLY5.MID` exists unused.
- 11014 and 11017 play the same file as 11013 and 11016, **without** the transposition.

### Format of the `.MID` files

Standard MIDI (SMF), format 1, division 240 ticks per quarter note, with tempo (`FF 51`), SMPTE (`FF 54`) and time signature (`FF 58`) meta-events. Verified fact with a simple analyzer (note count per channel):

- All of them follow the **Windows 3.1 MIDI authoring guidelines** (dual arrangement): the same music appears twice, in the "extended" arrangement (channels 1–10, percussion on 10) and in the "base" arrangement (channels 13–16, percussion on 16), with the same notes and the same programs. Example: `GAMETHME.MID` has 346 notes on channel 3 and 346 on 13, 456 on 10 and 456 on 16.
- The Windows 3.1 *MIDI Mapper* plays only one of the arrangements, depending on the synthesizer configuration. Playing all channels doubles every note. **The port plays only channels 1–10** (list in `data/som.json`, `canais_midi`).
- The programs are General MIDI (e.g. 34 bass, 3 electric piano, 95 pad), and channel 10 receives program 126 [hypothesis: a leftover from the conversion; in GM, a program on the percussion channel selects the kit].

### Type 20 (Sound Blaster) and type 50 (PC speaker)

Also standard MIDI (`MThd` signature), with LZSS in some of them. They are not copies of the `.MID` files (no hash matches): type 20 has a single arrangement, generally on channels 2–4 with percussion on 10 (e.g. 10001 with 182/346/98 notes on channels 2/3/4 and 597 on 10); type 50 is a reduction to one voice (10001: only channel 3), for `SetVoiceNote`. In Sound Blaster mode, `_myBeginSong` loads the type 20 object with the id from header word 0, applies the transposition (`musTransposeNote`) and plays it with `musPlayMemMidi`; in PC speaker mode it interprets type 50 inside `_myBeginSong` (`"Unknown command"`, `"Meta Event"`, `"Program Change"`...).

## Port

- **Reading:** `engine/src/formats/som.rs` (type 5 → PCM, type 18 → `CabecalhoMusica`, WAV writing) and `engine/src/formats/midi.rs` (SMF → event list with time in seconds, channel filter and transposition). Tests in `engine/tests/som_reais.rs`, against the user's files.
- **Playback:** `simant/src/som/` (SDL2 + synthesizer), see [regras/som.md](../regras/som.md) and `docs/decisoes.md`.
- **Mods:** an effect can be replaced with `objetos/5/<id>.bin` (same DAC format); a song with `arquivos/sound/<name>.mid`; the id → file table, the effects' rate and driver blocks, the volumes, the channels and the synthesizer are in `data/som.json`; the FM bank in `data/opl_banco.json`.

### MIDI Mapper of the reference installation

`SYSTEM.INI` connects MIDI to `sb16fm.drv` (OPL3) through `midimap.drv`, and `MIDIMAP.CFG` is in the "SB16 Ext Synth" setup: channels 1–10 → "Creative Music Synthesizer", with no patch map or key map; 11–16 muted. Details and evidence in [regras/som.md](../regras/som.md), "The original's audio in DOSBox-X". This confirms the choice `canais_midi` = 1–10.

### FM synthesizer (port)

`simant/src/som/fm.rs` is the port's own FM synthesizer in the style of the OPL2/OPL3 (an original implementation based on the chip's documented behavior, with no code from existing emulators):

- 18 two-operator voices (those of the OPL3); no 4-operator mode and no rhythm mode. It generates at 49 716 Hz (the chip's rate) and resamples linearly to the output.
- Per operator, the fields of registers 0x20–0xE0: AM, VIB, EG-TYP (`sustenta`), KSR, MULT (½, 1–10, 10, 12, 12, 15, 15), KSL (0, 3, 1.5 and 6 dB/octave, with the chip's base table by F-number), TL (0.75 dB), AR/DR/SL/RR and the 8 OPL3 waveforms. Envelopes with the timings from the YM3812 datasheet (attack 0→100 % of 2826 ms at rate 1, decay 0→96 dB of 39 281 ms, halving every 4 steps of effective rate); exponential attack, linear decay in dB; without EG-TYP, the release starts when SL is reached. Modulator feedback by the average of the last two outputs; the carrier receives ±4 cycles of modulation with the modulator at maximum; FM or additive connection; tremolo LFO (3.7 Hz, 4.8 or 1 dB) and vibrato (6.1 Hz, 14 or 7 cents). One channel at maximum gives 1/8 of full scale (4096/32768, like the chip's output) times the `ganho`.
- The port's own MIDI driver: note → frequency (block and F-number for KSL/KSR), pitch bend ±2 semitones, volume (CC 7), expression (CC 11), pan (CC 10, continuous; the original output was mono), sustain pedal (CC 64), CC 120/121/123; note attenuation = −`curva_volume_db` × log10(velocity × volume × expression), on the carrier (and on the modulator, if additive); voice stealing: the quietest released voice, otherwise the oldest.
- Percussion channel: timbre per note (`percussao`), played at `nota_fixa`; a note without a timbre = silence. Notes 59, 67, 70, 77 and 80 of the GM map and 85–95 outside it make no sound in the original (measured in the recordings, regras/som.md, "Percussion") and have no timbre in the port's bank.
- A program change takes effect from the next event on: in the `.MID` files where the first note comes before the Program Change at the same instant (`sad2`, `omnious1`, `happy2`, `jolly*`, `redq`, `sad3`, `blackq`, `newcoln2`; the `attack` channel has no change at all), that note plays with the program the channel already had; in a recording made right after boot it is 0 (piano), which the port also does (the channel starts at 0 for each song). [hypothesis] in the game, the original's driver keeps the program from the previous song; the port does not.

**Bank (`data/opl_banco.json`):** the port's own, generated by `tools/gerar_banco_opl.py` (one model per GM family with variations; the programs the songs use — 0, 3, 19, 21, 25, 26, 31, 33, 34, 35, 49, 57, 59, 69, 95 — and the percussion tuned against the 28 recordings of the original, with `tools/ajustar_timbre_fm.py` and `tools/som_dosbox.py perfil`/`percussao`, on 2026-10-19: harmonic profiles, envelopes, per-program level (`NIVEL_TL`) and velocity curve; regras/som.md, "FM bank adjustment against the 28 songs"). Keys: `melodicos` (128 timbres, index = GM program), `percussao` (note 35–81 → timbre), `curva_volume_db` (25: measured on the original; it was 40), `ganho` (1.65: calibrated by the measured relative volume; it was 2.25 with the previous bank), `tremolo_profundo`, `vibrato_profundo`. Timbre: `nome`, `modulador`, `portadora` (operators: `mult`, `tl`, `ar`, `dr`, `sl`, `rr`, `sustenta`, `ksl`, `ksr`, `onda`, `am`, `vib`), `realimentacao`, `aditivo`, `transposicao`, `nota_fixa`. Alternative: `fm.banco_op2` in `data/som.json` points to a `.op2` bank (DMX "#OPL_II#" format: 175 instruments of 36 bytes, 128 melodic plus notes 35–81; only the first voice is used) supplied by the user, read at run time; the port ships none.
