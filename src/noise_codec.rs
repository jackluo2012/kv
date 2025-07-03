use anyhow::Result;
use async_trait::async_trait;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures::{SinkExt, StreamExt};
use snow::{HandshakeState, TransportState};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{Decoder, Encoder, Framed};
use tracing::info;

pub const NOISE_PARAMS: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";
const HEADER_LEN: usize = 2;
const MAX_FRAME_SIZE: usize = 65535;
pub struct Builder {
    params: &'static str,
    initiator: bool,
}

enum NoiseState {
    Handshake(Box<HandshakeState>),
    Transport(TransportState),
    None,
}

impl NoiseState {
    fn write_message(&mut self, payload: &[u8], message: &mut [u8]) -> Result<usize, snow::Error> {
        match self {
            NoiseState::Handshake(s) => s.write_message(payload, message),
            NoiseState::Transport(s) => s.write_message(payload, message),
            NoiseState::None => unimplemented!(),
        }
    }

    fn read_message(&mut self, payload: &[u8], message: &mut [u8]) -> Result<usize, snow::Error> {
        match self {
            NoiseState::Handshake(s) => s.read_message(payload, message),
            NoiseState::Transport(s) => s.read_message(payload, message),
            NoiseState::None => unimplemented!(),
        }
    }
}

pub struct NoiseCodec {
    #[allow(dead_code)]
    builder: Builder,
    state: NoiseState,
}

impl NoiseCodec {
    pub fn builder(params: &'static str, initiator: bool) -> Builder {
        Builder::new(params, initiator)
    }

    pub fn switch_transport_mode(&mut self) -> Result<(), snow::Error> {
        self.state = match std::mem::replace(&mut self.state, NoiseState::None) {
            NoiseState::Handshake(s) => NoiseState::Transport(s.into_transport_mode()?),
            v => v,
        };

        Ok(())
    }
}

#[async_trait]
pub trait NoiseStream {
    async fn handshake(&mut self) -> Result<()>;
}

#[async_trait]
impl<S> NoiseStream for Framed<S, NoiseCodec>
where
    S: AsyncRead + AsyncWrite + Send + Sync + Unpin,
{
    /// 执行 Noise 协议的握手流程
    ///
    /// 该方法会根据当前端的角色（initiator 或 responder），
    /// 依次完成 Noise XX 协议的三次消息交换，协商出安全的会话密钥。
    /// 握手完成后，自动切换到传输模式（Transport Mode）。
    async fn handshake(self: &mut Self) -> Result<()> {
        match self.codec().builder.initiator {
            true => {
                // initiator（发起方）流程：
                // 第一步：发送 e（ephemeral 公钥）
                self.send(Bytes::from_static(&[])).await?;
                info!("-> e");

                // 第二步：接收 e, ee, s, es（对端 ephemeral、公钥交换、静态密钥等）
                let data = self.next().await.unwrap()?;
                info!("<- e, ee, s, es");

                // 第三步：发送 s, se（静态密钥和密钥交换）
                self.send(data.freeze()).await?;
                info!("-> s, se");
            }
            false => {
                // responder（响应方）流程：
                // 第一步：接收 e（对端 ephemeral 公钥）
                let data = self.next().await.unwrap()?;
                info!("<- e");

                // 第二步：发送 e, ee, s, es（本端 ephemeral、公钥交换、静态密钥等）
                self.send(data.freeze()).await?;
                info!("-> e, ee, s, es");

                // 第三步：接收 s, se（对端静态密钥和密钥交换）
                let _data = self.next().await.unwrap()?;
                info!("<- s, se");
            }
        }
        // 握手完成，切换到传输模式，后续数据将被加密传输
        self.codec_mut().switch_transport_mode()?;
        Ok(())
    }
}

impl Builder {
    /// 创建一个新的 Builder 实例
    ///
    /// # 参数
    /// - `params`: Noise 协议参数字符串
    /// - `initiator`: 是否为发起方（true 为发起方，false 为响应方）
    fn new(params: &'static str, initiator: bool) -> Self {
        Self { params, initiator }
    }

    /// 基于当前 Builder 构建一个 NoiseCodec 实例
    ///
    /// 该方法会根据 initiator 标志，创建握手状态的 Noise 协议对象，并封装为 NoiseCodec
    fn new_codec(self) -> Result<NoiseCodec> {
        // 创建 snow 的 Builder，用于生成密钥对和协议状态
        let builder = snow::Builder::new(self.params.parse()?);
        // 生成本地密钥对
        let keypair = builder.generate_keypair()?;
        // 设置本地私钥
        let builder = builder.local_private_key(&keypair.private);
        // 根据 initiator 标志，构建握手状态（发起方或响应方）
        let noise = match self.initiator {
            true => builder.build_initiator()?,
            false => builder.build_responder()?,
        };
        // 返回 NoiseCodec，初始状态为握手阶段
        Ok(NoiseCodec {
            builder: self,
            state: NoiseState::Handshake(Box::new(noise)),
        })
    }

    /// 基于当前 Builder 创建一个带有 NoiseCodec 的 Framed 流
    ///
    /// # 参数
    /// - `inner`: 实际的异步读写流（如 TCP 流）
    ///
    /// # 返回
    /// 返回一个 Framed<T, NoiseCodec>，用于加密/解密数据帧的异步流
    pub fn new_framed<T>(self, inner: T) -> Result<Framed<T, NoiseCodec>>
    where
        T: AsyncRead + AsyncWrite,
    {
        let codec = self.new_codec()?;
        Ok(Framed::new(inner, codec))
    }
}

// 为 NoiseCodec 实现 Encoder trait，用于加密和编码要发送的数据帧
impl Encoder<Bytes> for NoiseCodec {
    // 编码过程中可能出现的错误类型
    type Error = anyhow::Error;

    // encode 方法负责将明文数据加密后写入目标缓冲区
    fn encode(&mut self, item: Bytes, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // 创建一个临时缓冲区用于存放加密后的数据
        let mut buf = [0u8; MAX_FRAME_SIZE];
        // 获取待加密数据的长度
        let n = item.len();

        // 如果数据长度超过最大帧长度，返回错误
        if n > MAX_FRAME_SIZE {
            return Err(anyhow::anyhow!("Invalid Input".to_string()));
        }

        // 使用 Noise 协议状态加密数据，n 为加密后数据的实际长度
        let n = self.state.write_message(&item, &mut buf)?;

        // 预留空间：帧头（2字节）+ 加密后数据长度
        dst.reserve(HEADER_LEN + n);
        // 写入帧头（2字节，表示加密后数据长度）
        dst.put_uint(n as u64, HEADER_LEN);
        // 写入加密后的数据
        dst.put_slice(&buf[..n]);

        Ok(())
    }
}
// 为 NoiseCodec 实现 Decoder trait，用于解码收到的数据帧
impl Decoder for NoiseCodec {
    // 解码后的数据类型为 BytesMut
    type Item = BytesMut;
    // 错误类型为 anyhow::Error
    type Error = anyhow::Error;

    // 解码函数，每当有新数据到达时会被调用
    fn decode(&mut self, src: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // 创建一个临时缓冲区用于存放解密后的数据
        let mut buf = [0u8; MAX_FRAME_SIZE];

        // 如果缓冲区中的数据长度小于头部长度（2字节），说明数据还不完整，返回 None 等待更多数据
        if src.len() < HEADER_LEN {
            return Ok(None);
        }

        // 读取头部，获取数据帧的长度（前2字节），并将其转换为 usize
        let len = src.get_uint(HEADER_LEN) as usize;

        // 如果剩余的数据长度小于帧长度，说明数据还未接收完整，返回 None
        if src.len() < len {
            return Ok(None);
        }

        // 从缓冲区中取出完整的数据帧
        let payload = src.split_to(len);

        // 使用 Noise 协议状态解密数据帧，n 为解密后数据的实际长度
        let n = self.state.read_message(&payload, &mut buf)?;

        // 返回解密后的数据，封装为 BytesMut
        Ok(Some(BytesMut::from(&buf[..n])))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() -> Result<()> {
        let mut client = NoiseCodec::builder(NOISE_PARAMS, true).new_codec()?;
        let mut server = NoiseCodec::builder(NOISE_PARAMS, false).new_codec()?;

        let mut buf = BytesMut::new();

        // (client)
        // -> e
        client
            .encode(Bytes::from_static(b"hello"), &mut buf)
            .unwrap();

        let mut msg = buf.split_to(buf.len());
        // client sent msg out

        // (server)
        // <- e
        let msg = server.decode(&mut msg).unwrap().unwrap();
        // -> e, ee, s, es
        server.encode(msg.freeze(), &mut buf).unwrap();
        let mut msg = buf.split_to(buf.len());
        // server sent msg out

        // (client)
        // <- e, ee, s, es
        let msg = client.decode(&mut msg).unwrap().unwrap();
        // -> s, se
        client.encode(msg.freeze(), &mut buf).unwrap();
        let mut msg = buf.split_to(buf.len());
        // client sent msg out

        // (server)
        // <- s, se
        let msg = server.decode(&mut msg).unwrap().unwrap();
        assert_eq!(msg.freeze().as_ref(), b"hello");

        client.switch_transport_mode().unwrap();
        server.switch_transport_mode().unwrap();

        Ok(())
    }
}
