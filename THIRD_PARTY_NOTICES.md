# Third-party notices

- The firmware application declares GPL-2.0-or-later. See [LICENSE](LICENSE).
- The vendored RMK crates retain their upstream MIT or Apache-2.0 terms. The base is [RMK at 986017d](https://github.com/rmk-rs/rmk/tree/986017d901c7609a9a79167686524f014effa68a). See [LICENSE-MIT](rmk-ui-core/LICENSE-MIT) and [LICENSE-APACHE](rmk-ui-core/LICENSE-APACHE).
- The Luna keyboard pet artwork is by HellSingCoder. The OLED sprite and layer artwork in `firmware/src/assets.rs` was extracted from the [PandaKB QMK sources](https://github.com/PandaKBLab/QMK-firmware-for-keyboards/tree/887274498cfcbe5d531be4a9f612e5984780e8a2/sofle/sofle). See the [Luna QMK source](https://github.com/HellSingCoder/qmk_firmware/tree/master/keyboards/sofle/keymaps/helltm).
- The build downloads Nordic's precompiled SoftDevice Controller and MPSL via the Cargo dependencies. They have their own [Nordic license](licenses/NORDIC-5-Clause.txt) and [MPSL attribution](licenses/MPSL-ATTRIBUTION.txt). The source release excludes the linked UF2 images.
- The separately maintained [QMK WebHID editor](https://github.com/kolbenhans/qmk-webgui) is not bundled with this source release.
