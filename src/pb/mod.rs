mod abi;
pub use abi::*;
use bytes::{Bytes, BytesMut};
use prost::Message;

// 为 Response 实现new 方法
impl Response {
    pub fn new(key: String, value: Vec<u8>) -> Self {
        Response {
            code: 0,
            key,
            value,
        }
    }
    // not_found
    pub fn not_found(key: String) -> Self {
        Response {
            code: 404,
            key,
            value: Default::default(),
        }
    }
}

impl TryFrom<BytesMut> for Response {
    type Error = prost::DecodeError;

    fn try_from(buf: BytesMut) -> Result<Self, Self::Error> {
        Message::decode(buf)
    }
}

impl TryFrom<BytesMut> for Request {
    type Error = prost::DecodeError;
    fn try_from(buf: BytesMut) -> Result<Self, Self::Error> {
        Message::decode(buf)
    }
}

impl From<Response> for Bytes {
    fn from(resp: Response) -> Self {
        let mut buf = BytesMut::new();
        resp.encode(&mut buf).unwrap();
        buf.freeze()
    }
}

impl From<Request> for Bytes {
    fn from(req: Request) -> Self {
        let mut buf = BytesMut::new();
        req.encode(&mut buf).unwrap();
        buf.freeze()
    }
}

impl Request {
    pub fn new_get(key: &str) -> Self {
        Request {
            command: Some(request::Command::Get(RequestGet {
                key: key.to_string(),
            })),
        }
    }
    pub fn new_put(key: &str, value: &[u8]) -> Self {
        Request {
            command: Some(request::Command::Put(RequestPut {
                key: key.to_string(),
                value: value.to_vec(),
            })),
        }
    }
}
