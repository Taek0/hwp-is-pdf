# Third-party materials

Original engine code is licensed under the root MIT LICENSE, copyright 2026 Taek0.
That grant does not replace the rights or notices of third-party materials.

- `vendor/legacy-chart`: MIT, copyright 2025-2026 Edward Kim. Preserve its
  `LICENSE` and `UPSTREAM.md`, including the documented local changes.
- Registry dependencies: versions and checksums are fixed by `Cargo.lock`.
  Their own Cargo metadata and license/copyright/notice files govern those crates.
  The Docker build collects the downloaded crates' notices under
  `/usr/share/licenses/hwp-engine/dependencies/`. These are copied notices,
  not a declaration that every dependency has the engine's MIT license.
- The Rust toolchain's installed license/copyright files are also collected when
  available. Debian's package notices remain in the runtime base image.
- Fonts are separately supplied assets. The root MIT license does not license
  Windows, Hancom, Hanyang, Human or other fonts. Keep the authorized font ZIP
  separate from source and public images. Font embedding/subsetting in a PDF
  does not itself grant font-file redistribution or server-use rights.
- Sample documents, reference specifications, viewer-produced PDFs and other
  supplied third-party test/reference materials retain their original rights.
  Inclusion for regression testing is not a new public redistribution license.
  Expected PDFs are regression evidence, not independent fidelity certification.

The backend handoff includes a dependency inventory and copied notices separately
from the engine source and the private font ZIP. No new terms are imposed on the
existing third-party licenses.
