# fast-compress

Compressore ed estrattore CLI scritto in Rust per archiviare in modo lossless file (binari e testuali) e intere alberature di cartelle all'interno di un unico file archivio `.cmpr`.

## Caratteristiche principali

- **Supporto completo a file binari e testo:** opera direttamente sui byte grezzi (`u8`), senza richiedere codifica UTF-8.
- **Archiviazione ricorsiva di cartelle:** preserva gerarchia delle sottocartelle, directory vuote e permessi Unix (`0o777`).
- **Algoritmi di compressione lossless:**
  - `auto` *(predefinito)*: usa **LZ77** con fallback automatico a `store` se il file non è comprimibile.
  - `lz77`: codifica stile LZ4 con tabella hash in tempo $O(n)$ e supporto a match sovrapposti.
  - `rle`: Run-Length Encoding byte-level stile PackBits.
  - `store`: memorizzazione senza compressione.
- **Integrità e sicurezza:**
  - Checksum **CRC-32 (IEEE 802.3)** verificato su ogni file estratto.
  - Sanitizzazione dei percorsi relativi in estrazione per prevenire *path traversal* (`../`, path assoluti, `\`, NUL byte).
  - Prevenzione dell'auto-inclusione se l'archivio di destinazione si trova dentro la cartella sorgente (i symlink vengono saltati con avviso).

## Compilazione

```bash
cargo build --release
```

L'eseguibile compilato si trova in `target/release/fast-compress`.

## Utilizzo

```bash
# Comprimere un file o una cartella (destinazione predefinita: archive.cmpr)
fast-compress -p ./percorso/input -c

# Comprimere specificando destinazione e algoritmo
fast-compress -p ./mia_cartella -c -d backup.cmpr -a lz77

# Elencare il contenuto di un archivio
fast-compress -p backup.cmpr -l

# Estrarre un archivio (destinazione predefinita: extracted/)
fast-compress -p backup.cmpr -e -d ./output
```

### Opzioni CLI

| Flag | Forma estesa | Descrizione | Predefinito |
| :--- | :--- | :--- | :--- |
| `-p` | `--path <PATH>` | Percorso sorgente da comprimere o archivio `.cmpr` da leggere | *(obbligatorio)* |
| `-c` | `--compress` | Comprime file o cartella in un archivio `.cmpr` | `false` |
| `-e` | `--extract` | Estrae gli elementi dall'archivio `.cmpr` | `false` |
| `-l` | `--list` | Mostra l'elenco degli elementi presenti nell'archivio | `false` |
| `-d` | `--destination <DEST>` | File archivio in uscita (`-c`) o cartella di estrazione (`-e`) | `archive.cmpr` / `extracted` |
| `-a` | `--algorithm <ALGO>` | Algoritmo: `auto`, `lz77`, `rle`, `store` | `auto` |

*(Nota: i flag legacy `--exctract` e `--destionation` restano supportati come alias.)*

## Formato dell'archivio `.cmpr`

Ogni archivio inizia con l'header magic a 8 byte `FCMPR\x01\x00\x00`, seguito da una sequenza di entry:

- **Header entry:** `entry_kind (1B: 0=dir, 1=file)` | `mode (u32 LE)` | `path_len (u32 LE)` | `path (UTF-8 '/')`
- **Solo per i file:** `method (1B: 0=store, 1=rle, 2=lz77)` | `original_size (u64 LE)` | `compressed_size (u64 LE)` | `crc32 (u32 LE)` | `data`

## Struttura del progetto

```text
src/
├── main.rs                    # Entry point e gestione errori CLI
├── cli/cli.rs                 # Parsing argomenti (clap) e dispatch comandi
├── compressor/compressor.rs   # Orchestrazione compressione e riepilogo
├── decompressor/decompressor.rs # Estrazione e listing archivi (-e, -l)
├── lz77/lz77.rs               # Codec LZ77 con hash table
├── rle/rle.rs                 # Codec RLE byte-level (PackBits-style)
└── libs/
    ├── archive.rs             # Costanti formato .cmpr, CRC-32, validazione path
    ├── read_file.rs           # Lettura binaria e permessi file
    ├── read_folder.rs         # Attraversamento ricorsivo cartelle
    ├── compress_file.rs       # Compressione singolo file
    ├── compress_folder.rs     # Compressione ricorsiva cartella
    ├── create_archive.rs      # Writer bufferizzato archivio .cmpr
    ├── push_to_archive.rs     # Serializzazione entry nell'archivio
    ├── open_archive.rs        # Reader streaming e validazione header
    └── decompress_file.rs     # Verifica CRC-32, decompressione ed estrazione
```
