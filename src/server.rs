mod pb;
use std::sync::Arc;

use anyhow::{Error, Result};
use dashmap::DashMap;

use futures::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_util::codec::LengthDelimitedCodec;
use tracing::info;

use pb::{request::*, *};
#[derive(Debug)]
struct ServerState {
    store: DashMap<String, Vec<u8>>,
}

impl ServerState {
    fn new() -> Self {
        ServerState {
            store: DashMap::new(),
        }
    }
}

impl Default for ServerState {
    fn default() -> Self {
        ServerState::new()
    }
}
#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();
    let number_of_yaks = 3;
    // this creates a new event, outside of any spans.
    info!(number_of_yaks, "xx");

    let state = Arc::new(ServerState::new());
    let addr = "0.0.0.0:8888";
    let listener = TcpListener::bind(addr).await?;
    info!("Listening on {}", addr);
    // 在主线程中执行 accept
    loop {
        let (stream, addr) = listener.accept().await?;
        info!("Accepted connection from {}", addr);

        let shared = state.clone();
        // 生成一个task 处理连接
        // 我们要怎么传递消息,TCP 并不知道消息有多长
        tokio::spawn(async move {
            #[allow(unused_doc_comments)]
            /// 使用2字节长度字段将提供的 `stream` 包装为 `LengthDelimitedCodec`，
            /// 使该流能够以带有长度前缀的帧进行发送和接收。
            /// 这样可以安全高效地在流上传递消息并检测消息边界。
            let mut stream = LengthDelimitedCodec::builder()
                .length_field_length(2)
                .new_framed(stream);
            // steam.next 实现了读取长度字段的帧
            // 持续读取客户端发送过来的每一帧数据
            while let Some(Ok(buf)) = stream.next().await {
                // 尝试将收到的字节缓冲区反序列化为 Request 消息
                let msg: Request = buf.try_into()?;
                info!("Got a command {:?}", msg);

                // 根据请求中的 command 字段进行匹配处理
                let response = match msg.command {
                    // 处理 Get 命令
                    Some(Command::Get(RequestGet { key })) => match shared.store.get(&key) {
                        // 如果 key 存在，返回对应的值
                        Some(v) => Response::new(key, v.value().to_vec()),
                        // 如果 key 不存在，返回 not_found 响应
                        None => Response::not_found(key),
                    },
                    // 处理 Put 命令
                    Some(Command::Put(RequestPut { key, value })) => {
                        // 将 key 和 value 插入到共享的存储中
                        shared.store.insert(key.clone(), value.clone());
                        // 返回插入成功的响应
                        Response::new(key, value)
                    }
                    // 未知命令，暂未实现
                    None => unimplemented!("No command"),
                };

                // 将响应序列化后发送回客户端
                stream.send(response.into()).await?;
            }
            // 任务正常结束，返回 Ok
            Ok::<(), Error>(())
        });
    }
}
