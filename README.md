Building for WASM on macOS:

https://github.com/n0-computer/iroh/discussions/3200

```
PATH=(brew --prefix llvm)/bin:$PATH cargo build --target wasm32-unknown-unknown
```
