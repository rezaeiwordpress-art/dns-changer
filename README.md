# DNS Changer — Complete Windows Desktop Application (Phase 2)

Native Windows 10 and Windows 11 desktop application for safely configuring, backing up, verifying, and restoring IPv4 DNS settings across network adapters, built with **Tauri 2**, **Rust**, **React**, **TypeScript**, **Vite**, and **Tailwind CSS**.

---

## Features

- **Dashboard**: Real-time backend-derived connection status (`Connected` / `Disconnected` / `Error`), active DNS summary, prominent **START** and **STOP / RESTORE** controls, full network adapter metrics, last operation telemetry, and original backup recovery panel.
- **DNS Providers**: All 8 built-in providers (Cloudflare, Google, Quad9, OpenDNS, AdGuard, CleanBrowsing Adult, CleanBrowsing Family, DNS.Watch) plus custom user providers. Selecting a provider updates preference only without auto-applying network changes.
- **Custom DNS Management**: Add, edit, and delete custom IPv4 DNS providers with strict frontend + Rust IPv4 validation, duplicate-name detection, delete confirmation dialogs, and built-in provider protection.
- **Network Adapters**: Real Windows adapter discovery (Wi-Fi, Ethernet, Virtual, Disabled, Disconnected) keyed by stable **Interface GUID**, with DHCP status, gateway, IPv4 addresses, MAC, link speed, and non-mutating selection.
- **Complete Persian (RTL) & English (LTR) Localization**: Defaults to Persian (`fa`) with full RTL layout, `Vazirmatn` typography, isolated LTR technical fields (`font-mono-tech`), and instant switching to English (`en`).
- **Dark, Light & System Themes**: Complete dual-theme surfaces with Tailwind v4 `@custom-variant dark` and blue/cyan accents.
- **Security & Reliability**: Restrictive Tauri CSP & capabilities (`core:default`), zero dynamic PowerShell interpolation (`-EncodedCommand` + environment variables), non-elevated read-only queries, hardened app-owned UAC `--elevated-dns-op` IPC (user/SYSTEM/Administrators ACLs, UUID-paired paths, symlink/reparse point rejection), strict IPv4-only `netsh` reset with pre/post IPv6 preservation checks, multi-signal DNS mode detection, and atomic SHA-256 verified backups.

---

## Project Structure

```text
dns-changer/
├── src/
│   ├── components/             # Sidebar, TopBar, ConfirmDialog, NotificationStack
│   ├── pages/                  # Dashboard, Providers, CustomDns, Adapters, Settings, About
│   ├── hooks/                  # useApplicationSettings, useNetworkAdapters, useDnsProviders,
│   │                           # useDnsOperations, useLocalization, useTheme, useNotifications
│   ├── services/               # tauriClient.ts (typed Tauri 2 IPC wrapper)
│   ├── localization/           # fa.ts (default Persian RTL), en.ts (English LTR), index.ts
│   ├── types/                  # dns.ts
│   ├── utils/                  # validation.ts
│   ├── App.tsx
│   └── index.css
│
├── src-tauri/
│   ├── src/
│   │   ├── main.rs             # Elevated UAC helper CLI dispatch + GUI entry
│   │   ├── lib.rs              # Tauri Builder, AppState, IPC command registration
│   │   ├── models.rs           # Shared domain models
│   │   ├── commands/           # Tauri IPC handlers (network, providers, dns, admin)
│   │   ├── network/            # PowerShell networking backend & JSON parser
│   │   ├── dns/                # DnsEngine apply/restore workflow, validation, providers
│   │   ├── backup/             # Atomic per-adapter backup store with SHA-256 checksums
│   │   ├── admin/              # Win32 token elevation & ShellExecuteExW UAC workflow
│   │   ├── storage/            # Atomic AppConfig & custom provider store
│   │   ├── errors/             # Structured ErrorCode & sanitized CommandError
│   │   └── logging/            # Daily rotating JSON logs
│   ├── capabilities/           # default.json (core:default)
│   ├── icons/                  # 32x32.png, 128x128.png, 128x128@2x.png, icon.ico
│   ├── Cargo.toml
│   └── tauri.conf.json         # Production Windows NSIS & WiX MSI bundle config
│
├── scripts/verify-frontend.ts  # Automated validation & localization parity test suite
└── package.json
```

---

## Development Instructions

### Prerequisites (Windows 10 / Windows 11 Host)
1. **Rust Toolchain** (`rustup` stable MSVC, `x86_64-pc-windows-msvc`).
2. **Node.js 20+** and `npm`.
3. **Visual Studio Build Tools** with *Desktop development with C++* workload.
4. **WebView2 Runtime** (pre-installed on Windows 11 and current Windows 10).
5. *(Optional for `.msi` builds)* WiX Toolset v3 (automatically managed by Tauri bundler on Windows if needed; NSIS is bundled by Tauri CLI).

### Run in Development Mode
```powershell
npm install
npm run tauri dev
```

### Run TypeScript, Frontend, and Rust Checks
```powershell
npm run lint
npm run build
npx tsx scripts/verify-frontend.ts
cd src-tauri
cargo test
```

---

## Production Windows Build Instructions

Build the standalone Windows executable and installers (NSIS `*-setup.exe` and WiX `*.msi`):

```powershell
npm install
npm run lint
npm run build
npm run tauri build
```

Output artifacts on Windows are generated under:
- Executable: `src-tauri/target/release/dns-changer.exe`
- NSIS Installer: `src-tauri/target/release/bundle/nsis/DNS Changer_1.0.0_x64-setup.exe`
- MSI Installer: `src-tauri/target/release/bundle/msi/DNS Changer_1.0.0_x64_en-US.msi`

---

## Testing Report

### Checks Executed in Current Linux Build Environment
- **TypeScript Compilation (`npm run lint` / `tsc --noEmit`)**: Passed.
- **Production Vite Bundle (`npm run build`)**: Passed.
- **Frontend Automated Verification (`npx tsx scripts/verify-frontend.ts`)**: Passed (IPv4 validation rules, 8 built-in providers, 100% Persian/English translation key & error code parity).

### Checks Requiring a Windows 10 / Windows 11 Host with Rust
- **Rust Backend Formatting, Type Check & Unit Tests (`cargo fmt --check`, `cargo check`, `cargo test`)**: Could not be executed in this Linux Node.js sandbox (no `rustc`/`cargo` toolchain installed). Includes unit tests for IPv4/GUID validation, built-in providers, config & custom provider CRUD, backup serialization/SHA-256 integrity, multi-adapter isolation, hardened elevated IPC path/UUID/symlink/schema validation, multi-signal DNS mode detection, IPv6 preservation & unexpected-change recovery, Automatic/Static apply & restore, restart recovery, permission denial, UAC cancellation, and verification failure rollback.
- **Windows Live Integration & Installer Verification (`npm run tauri dev`, `npm run tauri build`)**: Must be executed on Windows 10/11 to verify live `Get-NetAdapter` / `Set-DnsClientServerAddress` / `netsh interface ipv4`, UAC prompt flow, LocalAppData IPC ACLs & reparse-point checks, IPv6 non-interference, and NSIS/MSI installer packaging.

---

## Remaining Limitations
- **Windows-Only Networking**: Live adapter discovery and DNS changes require Windows 10 or Windows 11; non-Windows hosts return `UNSUPPORTED_PLATFORM`.
- **IPv4 Scope**: Phase 1 & Phase 2 target IPv4 DNS servers; IPv6 DNS settings remain untouched.
- **External Network Policy Overrides**: Active VPN clients, Group Policy / MDM restrictions, or Windows DNS-over-HTTPS bindings can override interface DNS settings and trigger verification failure (backups remain preserved).
- **Release Installer Signing**: Production distribution outside local testing should code-sign `dns-changer.exe` and the NSIS/MSI installers with a valid Authenticode certificate to prevent Windows SmartScreen warnings.
