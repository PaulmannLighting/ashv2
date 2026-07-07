# TODOs

- [ ] Merge `Tasks` into `Handle` and only return `Handle` from `Actor::spawn()`.
- [x] Reduce API complexity of `Handle::send()` by returning a `Result<(), AshError>` from a single await point.
