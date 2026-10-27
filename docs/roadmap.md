# Roadmap

The first release establishes the API contract, reproducible evidence, and a
public contribution workflow. It is not a declaration of production parity on
every desktop configuration.

1. Dogfood an opt-in Rust input adapter in AirCodum Agentum, retaining existing
   authentication, coordinate scaling, key normalization, and drag cancellation.
2. Validate Linux/X11 and Windows native event delivery in hosted desktop jobs;
   add hardware coverage for multiple monitors, DPI changes, and reconnects.
3. Improve small-region capture without weakening macOS color/Retina correctness;
   benchmark actual remote-desktop workloads before replacing capture backends.
4. Establish a stable Node binding/distribution strategy and first crates.io
   release after real consumers validate the API.
5. Evaluate Wayland portal integration as a separate backend with explicit
   permission and compositor support, rather than claiming X11 covers Wayland.

Performance claims will continue to publish slower cases and measurement limits.
