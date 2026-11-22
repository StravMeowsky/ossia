# Native engine resources

`ENGINE.lock` is a versioned JSON lock shared by the Rust engine and the Node development installer. Runtime code does not download, unpack or repair resources. LilyPond and its converters are fixed at 2.26.0.

## Release targets

| Target | Preparation | Validation |
|---|---|---|
| darwin-arm64 | Official tar.gz; `pnpm engine:install` on Apple Silicon | Integrity, converters, SVG/PDF/MIDI, Chinese/space relocation, cache and diagnostics |
| mingw-x86_64 | Official ZIP; `node scripts/engine.mjs install --target=mingw-x86_64 --download` | Archive download/hash, extraction and inventory; Rust Windows target static check |

Run each preparation command from the repository, or call the script by absolute path from another cwd. The target flag selects a resource bundle, not emulation. Foreign target preparation never executes its tools. `node scripts/engine.mjs verify --target=<target>` checks the selected tree; only the current host target runs the smoke test.

The installer verifies the archive before extraction, prepares in a sibling staging directory, inventories and verifies the result, then replaces an existing development tree. A failed download/preparation preserves the previous tree. An explicit `OSSIA_LILYPOND_ROOT` is authoritative for `verify`/`path`, with no fallback. Rust receives its root explicitly.

The complete upstream tree is retained: Python 3.14.3, Guile modules, Ghostscript, standard includes, music/text fonts, libraries and licenses. Converters run as `libexec/python3.14 libexec/musicxml2ly` (or `midi2ly`) on Unix; Windows uses `bin/python.exe bin/musicxml2ly.py` (or `midi2ly.py`). No shell wrapper or user Python is required. The archive checksums were calculated from the actual downloaded official archives, not guessed from filenames.

`resources.json` records the build/target/archive identity and every immutable regular file's size, digest and executable flag. Installer preparation preserves archive permissions and repairs Guile `.go` mtimes in the writable staging tree. Bytecode caches are excluded and runtime bytecode generation is disabled. Finish preparation before signing; verification does not change installed files. Resource inventories are local integrity metadata, not a substitute for package signatures.

Sources: [official downloads](https://lilypond.org/download.html), [v2.26.0 native packaging](https://github.com/lilypond/lilypond/tree/v2.26.0/release/binaries), [upstream build requirements](https://lilypond.org/doc/v2.26/Documentation/contributor/requirements-for-compiling-lilypond).
