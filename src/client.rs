mod noise_codec;
mod pb;

use noise_codec::{NOISE_PARAMS, NoiseCodec, NoiseStream};
use pb::{Request, Response};
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
    // 加密
    let mut stream = NoiseCodec::builder(NOISE_PARAMS, true).new_framed(stream)?;

    stream.handshake().await?;

    let msg = Request::new_put("hello", b"world");
    // 将请求序列化成字节缓冲区
    stream.send(msg.into()).await?;

    let msg = Request::new_get("hello");
    stream.send(msg.into()).await?;

    // 接收响应
    while let Some(Ok(buf)) = stream.next().await {
        // 反序列化响应
        let resp = Response::try_from(buf)?;
        info!("Got a response {:?}", resp);
    }

    Ok(())
}
