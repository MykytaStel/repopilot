### Added

- **NestJS controller parameter taint analysis.** RepoPilot now treats request-bound controller parameters decorated with `@Body()`, `@Query()`, `@Param()`, `@Headers()`, `@Req()`, or `@Request()` as HTTP request sources for intra-procedural taint propagation. Response, dependency-injection, and custom decorators remain excluded, parameterized SQL stays quiet, and differential review-zoo fixtures pin the safe and unsafe boundaries.
