# Third-party license inventory

This inventory is derived from the committed `Cargo.lock` and
`apps/web/package-lock.json` for TextScope `0.1.0`. It is an engineering aid,
not legal advice. Transitive package metadata and upstream notices should be
rechecked before any redistribution.

## Direct Rust dependencies

| Package | License expression |
| --- | --- |
| `html5gum` | MIT |
| `lopdf` | MIT |
| `roxmltree` | MIT OR Apache-2.0 |
| `scraper` | ISC |
| `unicode-normalization` | MIT OR Apache-2.0 |
| `unicode-properties` | MIT OR Apache-2.0 |
| `unicode-script` | MIT OR Apache-2.0 |
| `unicode_names2` | (MIT OR Apache-2.0) AND Unicode-DFS-2016 |
| `zip` | MIT |
| `serde` | MIT OR Apache-2.0 |
| `serde_json` | MIT OR Apache-2.0 |
| `wasm-bindgen` | MIT OR Apache-2.0 |

The full Cargo graph contains additional transitive expressions, including
MPL-2.0, ISC, Zlib, Unicode-3.0/Unicode-DFS-2016, Unlicense/MIT, and other
permissive alternatives. In particular, transitive MPL-2.0 components and
Unicode data terms deserve a final notice review for the exact distribution.

## Direct JavaScript dependencies

| Package | Role | License |
| --- | --- | --- |
| `react`, `react-dom` | runtime UI | MIT |
| `@playwright/test` | browser tests | Apache-2.0 |
| `@types/react`, `@types/react-dom` | type declarations | MIT |
| `@vitejs/plugin-react` | build tooling | MIT |
| `typescript` | compiler | Apache-2.0 |
| `vite` | build/dev server | MIT |

The installed dependency metadata also includes BSD-3-Clause, ISC, MPL-2.0,
Apache-2.0, and MIT packages. A few nested `nanoid` metadata files do not
declare their own license; the parent package metadata should be checked when
the lockfile is refreshed.

## Notices and obligations

No standalone third-party `NOTICE` file was identified in the source tree
reviewed for this copy. The Apache License 2.0 requires preserving applicable
copyright, license, attribution, and upstream NOTICE text when a dependency
provides one; absence of a local NOTICE file is not a legal conclusion. Before
distribution, regenerate this inventory and inspect the exact dependency
archives for required notices, especially for MPL-2.0, Unicode, Zlib, ISC, and
Apache-2.0 components.
