# Security

Report vulnerabilities through [GitHub private vulnerability reporting](https://github.com/priyankark/rusty-robot/security/advisories/new).
Include affected versions, OS, reproduction, and impact. Avoid publishing exploit
instructions before maintainers have had an opportunity to respond. Only the
latest 0.1 release receives fixes; no response-time guarantee is offered.

This is an in-process native automation library, not an authorization layer or
sandbox. The embedding application owns authentication, user consent, allowable
actions, and capture retention. Do not expose desktop methods directly to an
unauthenticated network endpoint. Native permissions remain enforced by the OS.

Dependency updates are monitored by Dependabot and a scheduled RustSec audit.
Report security-relevant native buffer, file-decoding, or event-cleanup failures.
