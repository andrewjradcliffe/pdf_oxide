# pdf_oxide platform subpackages

Each subdirectory is its own npm package containing the prebuilt `.node` binding
for a specific `(os, cpu[, libc])` triple. They are published alongside the main
`pdf-oxide` package and wired in via its `optionalDependencies`, so `npm install
pdf-oxide` automatically resolves only the subpackage matching the end user's
platform — no compilation, no toolchain, no `install` hook.

Do **not** edit the `.node` files in these directories by hand; they are
populated by the `build-nodejs` job in `.github/workflows/release.yml` and
published by the `publish-npm-platforms` job. Manual edits will be clobbered on
the next release.

The loader in `src/index.ts` `require`s these packages by name:

| directory            | package                        | os       | cpu     | libc    |
| -------------------- | ------------------------------ | -------- | ------- | ------- |
| `linux-x64-gnu/`     | `pdf_oxide-linux-x64-gnu`      | `linux`  | `x64`   | `glibc` |
| `linux-arm64-gnu/`   | `pdf_oxide-linux-arm64-gnu`    | `linux`  | `arm64` | `glibc` |
| `darwin-x64/`        | `pdf_oxide-darwin-x64`         | `darwin` | `x64`   | —       |
| `darwin-arm64/`      | `pdf_oxide-darwin-arm64`       | `darwin` | `arm64` | —       |
| `win32-x64-msvc/`    | `pdf_oxide-win32-x64-msvc`     | `win32`  | `x64`   | —       |
| `win32-arm64-msvc/`  | `pdf_oxide-win32-arm64-msvc`   | `win32`  | `arm64` | —       |
