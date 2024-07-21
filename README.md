### How to use

```sh
cargo run
```

(All required features should be in `Cargo.toml` already)

### Overview
1. When the app starts, it reads the gLTF file (as specified in `assets/default.terrain.bin`) and generates meshlets & colliders for it
2. It then spawns the terrain and a player controller for roaming around
