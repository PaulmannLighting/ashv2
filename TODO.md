# TODOs

- [ ] Merge `Tasks` into `Proxy` and only return `Proxy` from `Actor::spawn()`.
- [ ] Reduce API complexity of `Proxy::send()` by returning a `Result<(), AshError>` from a single await point.