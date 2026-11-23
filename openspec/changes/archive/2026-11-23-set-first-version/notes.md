# Notes

## 2026-11-23

The owner chose 0.1.0 as the first version once the Cube Core comparison was in
place. Implemented with Claude Code (Anthropic). No spec delta
(`skip_specs: true`): behaviour is unchanged and the determinism spec already
states that fingerprints cover the kernel version.

Done: workspace version 0.1.0, `Cargo.lock` for the six members, `CHANGELOG.md`.
Fingerprints change for every compilation by design. No golden holds a
fingerprint, so none changed.

Written while #80 (joined measures) was still open; the changelog describes
main with #80 merged. If #80 does not merge, drop the changelog lines about
third-cube joins and measures of a joined cube.

Open: no tag, image or release is published, by the owner's decision.
