use std::{io::Write, pin::Pin, sync::Arc};

use bytes::{BufMut, BytesMut};
use prost::Message;
use ring::rand::{SecureRandom, SystemRandom};

use crate::{
    HandshakeRequest, LogPoint, Pong, Statistics,
    protocol::{
        Algorithm,
        close::Close,
        data::{DataAck, DataFrame},
        frame::Frame,
        handshake::{HandshakeInfo, HandshakeResponse},
        heartbeat::Ping,
        pb::{self},
    },
};

use super::{
    buf::Buffer,
    crypto::CryptoCore,
    signature::{self, Signature},
};

pub fn build_handshake_request(
    signature: &Signature,
    ecdh_public_key: &[u8],
    log_points: Vec<LogPoint>,
    info: HandshakeInfo,
) -> anyhow::Result<Buffer> {
    let rand = SystemRandom::new();
    let mut salt = [0; 4];
    rand.fill(&mut salt)
        .map_err(|_| anyhow::anyhow!("generate salt error"))?;
    let hash = signature::calculate_hash(signature.get_public_key(), &salt);

    let mut buf = BytesMut::new();
    buf.put(&salt[..]);
    buf.put(&hash[..]);
    buf.put(ecdh_public_key);
    let signature = signature.sign(&buf);

    let request = pb::HandshakeRequest {
        signature,
        public_key_salt: salt.to_vec(),
        public_key_hash: hash.to_vec(),
        public_key_data: ecdh_public_key.to_vec(),
        log_points: log_points.into_iter().map(|l| l.into()).collect(),
        ip: info.ip,
        cluster: info.cluster,
        version: info.version,
        os_name: info.os_name,
        os_version: info.os_version,
        os_arch: info.os_arch,
        cpu_name: info.cpu_name,
        cpu_vendor: info.cpu_vendor,
        cpu_frequency: info.cpu_frequency,
        cpu_nums: info.cpu_nums,
    };

    let mut buf = Buffer::new(128);

    buf.write_all(&request.encode_to_vec())?;
    let len = buf.len();
    prepend_type_and_len(&mut buf, Frame::HANDLESHAKE_REQ, len as u32);
    Ok(buf)
}

///
/// Parse handshake
///
pub async fn parse_handshake_request<F>(
    data: &[u8],
    signature: F,
) -> anyhow::Result<(HandshakeRequest, Arc<Signature>)>
where
    F: Fn(&str) -> Pin<Box<dyn Future<Output = anyhow::Result<Arc<Signature>>> + Send>>,
{
    let r = pb::HandshakeRequest::decode(data)?;
    let signature = signature(&r.cluster).await?;

    let mut buf = BytesMut::new();
    buf.put(&r.public_key_salt[..]);
    buf.put(&r.public_key_hash[..]);
    buf.put(&r.public_key_data[..]);

    let verify = signature.verify(&r.public_key_salt, &r.public_key_hash, &r.signature, &buf)?;
    if !verify {
        return Err(anyhow::anyhow!("Signature error."));
    }

    let result = r.try_into()?;
    Ok((result, signature))
}

pub fn build_handshake_response(
    signature: &Signature,
    algorithm: &Algorithm,
    ecdh_public_key: &[u8],
) -> anyhow::Result<Buffer> {
    let rand = SystemRandom::new();
    let mut salt = [0; 4];
    rand.fill(&mut salt)
        .map_err(|_| anyhow::anyhow!("generate salt error"))?;
    let hash = signature::calculate_hash(signature.get_public_key(), &salt);

    let mut buf = BytesMut::new();
    buf.put(&salt[..]);
    buf.put(&hash[..]);
    buf.put(ecdh_public_key);
    buf.put_i32(algorithm.code());
    let signature = signature.sign(&buf);

    let response = pb::HandshakeResponse {
        signature,
        public_key_salt: salt.to_vec(),
        public_key_hash: hash.to_vec(),
        public_key_data: ecdh_public_key.to_vec(),
        algorithm: algorithm.code(),
    };

    let mut buf = Buffer::new(128);

    buf.write_all(&response.encode_to_vec())?;
    let len = buf.len();
    prepend_type_and_len(&mut buf, Frame::HANDLESHAKE_RSP, len as u32);
    Ok(buf)
}

pub fn parse_handshake_response(
    data: &[u8],
    signature: &Signature,
) -> anyhow::Result<HandshakeResponse> {
    let r = pb::HandshakeResponse::decode(data)?;

    let mut buf = BytesMut::new();
    buf.put(&r.public_key_salt[..]);
    buf.put(&r.public_key_hash[..]);
    buf.put(&r.public_key_data[..]);
    buf.put_i32(r.algorithm);

    let verify = signature.verify(&r.public_key_salt, &r.public_key_hash, &r.signature, &buf)?;
    if !verify {
        return Err(anyhow::anyhow!("Signature error."));
    }

    r.try_into()
}

pub fn build_ping(
    buffer: &mut Buffer,
    core: &CryptoCore,
    ts: i64,
    log_points: Vec<LogPoint>,
    statistics: Statistics,
) -> anyhow::Result<()> {
    buffer.clear();
    let ping = pb::Ping {
        ts,
        log_points: log_points.into_iter().map(|l| l.into()).collect(),
        uptime: statistics.uptime,
        cpu: statistics.cpu,
        cpu_current_process: statistics.cpu_current_process,
        cpu_load_one: statistics.cpu_load_one,
        cpu_load_five: statistics.cpu_load_five,
        cpu_load_fifteen: statistics.cpu_load_fifteen,
        mem_used: statistics.mem_used,
        mem_free: statistics.mem_free,
        swap_used: statistics.swap_used,
        swap_free: statistics.swap_free,
        disk_used: statistics.disk_used,
        disk_free: statistics.disk_free,
        io_read: statistics.io_read,
        io_written: statistics.io_written,
        net_send: statistics.net_send,
        net_received: statistics.net_received,
        tls: statistics.tls,
        pv: statistics.pv,
        rt: statistics.rt,
        error: statistics.error,
        ext_info: statistics.ext_info,
    };
    buffer.write_all(&ping.encode_to_vec())?;
    core.encrypt(buffer);
    let len = buffer.len();
    prepend_type_and_len(buffer, Frame::HEARTBEAT_PING, len as u32);
    Ok(())
}

pub fn parse_ping(buffer: &mut Buffer, core: &CryptoCore) -> anyhow::Result<Ping> {
    core.decrypt(buffer)?;
    let data = pb::Ping::decode(buffer.as_ref())?;
    data.try_into()
}

pub fn build_pong(buffer: &mut Buffer, core: &CryptoCore, ts: i64) -> anyhow::Result<()> {
    buffer.clear();
    let pong = pb::Pong { ts };
    buffer.write_all(&pong.encode_to_vec())?;
    core.encrypt(buffer);
    let len = buffer.len();
    prepend_type_and_len(buffer, Frame::HEARTBEAT_PONG, len as u32);
    Ok(())
}

pub fn parse_pong(buffer: &mut Buffer, core: &CryptoCore) -> anyhow::Result<Pong> {
    core.decrypt(buffer)?;
    let data = pb::Pong::decode(buffer.as_ref())?;
    Ok(Pong { ts: data.ts })
}

pub fn build_data(buffer: &mut Buffer, data: DataFrame, core: &CryptoCore) -> anyhow::Result<()> {
    let data: pb::Data = data.into();
    buffer.clear();
    let _ = buffer.write(&data.encode_to_vec())?;
    core.encrypt(buffer);
    let len = buffer.len();
    prepend_type_and_len(buffer, Frame::DATA, len as u32);
    Ok(())
}

pub fn parse_data(buffer: &mut Buffer, core: &CryptoCore) -> anyhow::Result<DataFrame> {
    core.decrypt(buffer)?;
    let data = pb::Data::decode(buffer.as_ref())?;
    Ok(data.into())
}

pub fn build_ack(buffer: &mut Buffer, data: DataAck, core: &CryptoCore) -> anyhow::Result<()> {
    let data: pb::Ack = data.into();
    buffer.clear();
    let _ = buffer.write(&data.encode_to_vec())?;
    core.encrypt(buffer);
    let len = buffer.len();
    prepend_type_and_len(buffer, Frame::ACK, len as u32);
    Ok(())
}

pub fn parse_ack(buffer: &mut Buffer, core: &CryptoCore) -> anyhow::Result<DataAck> {
    core.decrypt(buffer)?;
    let data = pb::Ack::decode(buffer.as_ref())?;
    Ok(DataAck {
        log_point: data.log_point.map(|l| l.into()),
    })
}

pub fn build_close(buffer: &mut Buffer, data: &Close, core: &CryptoCore) -> anyhow::Result<()> {
    let data: pb::Close = data.into();
    buffer.clear();
    let _ = buffer.write(&data.encode_to_vec())?;
    core.encrypt(buffer);
    let len = buffer.len();
    prepend_type_and_len(buffer, Frame::CLOSE, len as u32);
    Ok(())
}

fn prepend_type_and_len(buffer: &mut Buffer, data_type: u8, data_len: u32) {
    let mut data_len = data_len.to_be_bytes();
    data_len.reverse();
    for b in data_len {
        buffer.prepend_byte(b);
    }
    buffer.prepend_byte(data_type);
}
