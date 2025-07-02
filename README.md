# kv

description of the crate

## How to use it

```bash
$ cargo generate --git https://github.com/tyrchen/rust-lib-template
```

Have fun with this crate!

### pre-commit 挻多好处，检查提交前的问题，代码格式

```shell
pre-commit install
```

### 添加 cargo 需要的库文件

```shell
cargo add prost # 这个 poto 是protobuf 的库
cargo add tonic # grpc 的库
cargo add tonic-build # grpc 的库
cargo add tokio # tokio 的库
```

### 在 cargo.toml 文件中添加以下内容,增加 更多的可执行文件

```toml
[[bin]]
name = "server"
path = "src/server.rs"

[[bin]]
name = "client"
path = "src/client.rs"
```

### 运行

````shell
cargo run --bin server
cargo run --bin client
```

### 添加 dashmap
```bash
cargo add dashmap
```
## 服务端应用的基本组成部分
### 数据序列化:serde/protobuf/flatbuffer/capnp/etc.
### 传输协议:tcp/http/websocket/quic/etc.
### 安全协议:TLS/noise protocol/secio/etc.
### 应用协议: your own application logic
### 数据在各个部分之间的流传:共享内存，channel等




## License

This project is distributed under the terms of MIT.

See [LICENSE](LICENSE.md) for details.

Copyright 2025 Tyr Chen
````
