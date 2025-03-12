# How to Use

- Start a node:
```bash
cargo run -- --listen /ip4/0.0.0.0/tcp/8000
```

- Start another node and connect to the first:
```bash
cargo run -- --listen /ip4/0.0.0.0/tcp/8001 --peer /ip4/127.0.0.1/tcp/8000
```

- Create a transaction:
```bash
cargo run -- transaction Alice Bob 50.0
```

- Mine a block:
```bash
cargo run -- mine MinerAddress
```

- Check balance:
```bash
cargo run -- balance Alice
```