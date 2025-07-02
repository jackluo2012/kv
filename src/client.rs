mod pb;
use pb::{Request, Response};
use tokio_util::codec::LengthDelimitedCodec;
use tracing::info;

use anyhow::Result;
use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let addr = "127.0.0.1:8888";
    let stream = TcpStream::connect(addr).await?;
    // 客户端和服务器端要保持同步
    let mut stream = LengthDelimitedCodec::builder()
        .length_field_length(2)
        .new_framed(stream);

    let msg = Request::new_put("hello", b"world");
    // 将请求序列化成字节缓冲区
    stream.send(msg.into()).await?;

    let msg = Request::new_get("hello1");
    stream.send(msg.into()).await?;

    // 接收响应
    while let Some(Ok(buf)) = stream.next().await {
        // 反序列化响应
        let resp = Response::try_from(buf)?;
        info!("Got a response {:?}", resp);
    }

    Ok(())
}
