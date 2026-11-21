# Bringing a new PdfCraft release into READORA

READORA's history starts fresh (it doesn't carry PdfCraft's commits), so an update applies the
changes between two PdfCraft releases as a patch. `packaging/readora/UPSTREAM` records the release
READORA is on now.

```sh
# 1. PdfCraft's changes from the release READORA is on to the new one (here v0.22.0)
git clone https://github.com/storytold/pdfcraft ../pdfcraft-upstream
git -C ../pdfcraft-upstream diff v0.21.1 v0.22.0 -- . ':!docs/brand' ':!.github' > /tmp/pdfcraft.patch

# 2. Apply them; --3way leaves conflict markers where READORA changed the same lines
git apply --3way /tmp/pdfcraft.patch

# 3. Rename the "PdfCraft" strings the new release added, and check nothing is left
python3 packaging/readora/rebrand_strings.py
python3 packaging/readora/rebrand_strings.py --check

# 4. Record the new base (packaging/readora/UPSTREAM: tag and commit), bump nothing else, then
cargo check --locked -p pdfcraft -p pdfcraft-cli
```

The version in `Cargo.toml` comes from PdfCraft. A push to `main` with a new version makes a new
release, which READORA's built-in updater (Help ▸ Check for updates) then offers.

Places READORA changed by hand (not by the script), where merges conflict most often:

- `crates/engine/src/links.rs` and `commands.rs` (READORA links, no Discord/ArtCraft entries),
  `crates/ui-egui/src/menus.rs` (Help menu), `chrome.rs` (no Discord button), `home.rs` (welcome,
  no community card), `widgets.rs` (`readora_mark`), `dialogs.rs` (About)
- `crates/ui-egui/src/lib.rs` (`UPDATE_APP`: READORA's own releases) and
  `crates/ui-egui/src/platform/updates.rs` (`RELEASES_PAGE`)
- `apps/pdfcraft/build.rs` (exe name, company, copyright), `apps/pdfcraft-cli/src/main.rs`
- `assets/app-icon/*` (READORA icon: regenerate with `packaging/readora/make_logo_assets.py`)
- `README.md`, `NOTICE`, `LICENSE-MIT`
- `packaging/readora/` is READORA's copy of `packaging/windows/` with the names and GUIDs changed:
  bring packaging changes over by hand.
- Keep PdfCraft's ArtCraft marks (`docs/brand/`) and workflows (`.github/workflows/` other than
  `readora-windows.yml`) out.
