# Attribution

Rusty Robot is an independent Rust port of the RobotJS API and test contracts.
Ported tests and algorithms derive from [RobotJS](https://github.com/octalmage/robotjs),
Copyright (c) 2014 Jason Stallings, under MIT. The complete notice and permission
text are retained in [LICENSE.md](LICENSE.md).

The reference revision is `0af6073e1f1bcff865ffe52801c71647829cdc1c`.
Rusty Robot does not link or redistribute the RobotJS C/C++ addon, its bundled
libpng, uthash, snprintf, or random-number sources. PNG support uses the Rust
`png` crate. Cargo dependencies retain their individual upstream licenses.
The Target Practice Electron fixture is fetched only for opt-in desktop tests;
it retains its upstream license in the installed package.

The robot logo was generated for this project using OpenAI image generation.
The project makes the logo, documentation, skill, and original source available
under the same MIT terms, to the extent applicable. It does not claim affiliation
with RobotJS or the Rust project.
