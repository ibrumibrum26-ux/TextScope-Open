# TextScope deployment guidance

## Deployment model

TextScope is a static browser application. A Vite build produces files under
`apps/web/dist`; a static HTTPS host can serve those files without an API,
database, account system, or server-side text processor. Analysis remains in
the browser and Rust/WASM runtime.

No hosting provider, domain, CDN, backend, telemetry service, or remote
processing endpoint is selected by this repository.

## Build input

From a clean checkout, use the committed lockfiles:

```powershell
Set-Location apps/web
npm.cmd ci
npm.cmd run build
```

Only the contents of `apps/web/dist` belong in a static deployment. Do not
include source control metadata, `node_modules`, tests, reports, traces,
sourcemaps, local paths, or release-workspace files.

## HTTPS and headers

Use HTTPS. Clipboard capabilities require a secure context, while localhost and
loopback remain useful for development. Review the final host against a
provider-neutral baseline:

- a same-origin Content Security Policy with no `object-src` or inline
  execution and only the narrowly required WebAssembly allowance;
- `X-Content-Type-Options: nosniff`;
- `Referrer-Policy: no-referrer` or stricter;
- a least-privilege `Permissions-Policy` for clipboard;
- explicit same-origin/CORS rules, with no wildcard access to app or user data;
- a valid certificate and no mixed content.

These are recommendations, not provider-specific configuration.

## Post-deployment validation

Before advertising a deployment, validate the exact origin in Microsoft Edge
Stable:

1. the app loads without console errors or failed requests;
2. Paste Text, Rich Clipboard, TXT, Markdown, HTML, DOCX, and PDF paths work;
3. Scan, Reveal, Clean, the transformation manifest, independent rescan, and
   the Verification Report work for representative local inputs;
4. clipboard access occurs only after the user action;
5. network inspection shows no remote input fetch, result transmission,
   analytics, telemetry, or remote logging;
6. the 10 MiB guard and unsupported-format errors remain controlled and
   visible.

Any future publication or deployment requires a separate human approval.
