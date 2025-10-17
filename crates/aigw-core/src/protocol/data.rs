use prost::Message;

use super::pb;

#[derive(Clone, Debug, Copy, Hash, PartialEq, Eq)]
pub enum LogType {
    Site,
    Acme,
}

impl LogType {
    pub fn code(&self) -> u32 {
        match &self {
            LogType::Site => 1,
            LogType::Acme => 2,
        }
    }

    pub fn all_types() -> Vec<LogType> {
        let types = vec![LogType::Site, LogType::Acme];
        types
    }
}

impl TryFrom<u32> for LogType {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(LogType::Site),
            2 => Ok(LogType::Acme),
            _ => Err(anyhow::anyhow!("Unknow error.")),
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub enum LogAction {
    Add,
    Update,
    Delete,
}

impl LogAction {
    pub fn code(&self) -> u32 {
        match &self {
            LogAction::Add => 1,
            LogAction::Update => 2,
            LogAction::Delete => 3,
        }
    }
}

impl TryFrom<u32> for LogAction {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(LogAction::Add),
            2 => Ok(LogAction::Update),
            3 => Ok(LogAction::Delete),
            _ => Err(anyhow::anyhow!("Unknow error.")),
        }
    }
}

#[derive(Clone, Debug, Copy)]
pub struct LogPoint {
    pub log_id: u64,
    pub log_type: LogType,
}

impl From<LogPoint> for pb::LogPoint {
    fn from(val: LogPoint) -> Self {
        pb::LogPoint {
            log_id: val.log_id,
            log_type: val.log_type.code(),
        }
    }
}

impl From<pb::LogPoint> for LogPoint {
    fn from(val: pb::LogPoint) -> Self {
        LogPoint {
            log_id: val.log_id,
            log_type: val.log_type.try_into().unwrap(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ChangeLog {
    pub log_id: u64,
    pub log_type: LogType,
    pub log_action: LogAction,
    pub data_id: u64,
    pub data: Vec<u8>,
}

impl ChangeLog {
    pub fn to_vec(self) -> Vec<u8> {
        let pb: pb::ChangeLog = self.into();
        pb.encode_to_vec()
    }
}

impl TryFrom<&[u8]> for ChangeLog {
    type Error = anyhow::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let data = pb::ChangeLog::decode(value)?;
        Ok(data.into())
    }
}

impl From<ChangeLog> for pb::ChangeLog {
    fn from(val: ChangeLog) -> Self {
        Self {
            log_id: val.log_id,
            log_type: val.log_type.code(),
            log_action: val.log_action.code(),
            data_id: val.data_id,
            data: val.data,
        }
    }
}

impl From<pb::ChangeLog> for ChangeLog {
    fn from(val: pb::ChangeLog) -> Self {
        Self {
            log_id: val.log_id,
            log_type: val.log_type.try_into().unwrap(),
            log_action: val.log_action.try_into().unwrap(),
            data_id: val.data_id,
            data: val.data,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DataFrame {
    pub logs: Vec<ChangeLog>,
    pub log_point: Option<LogPoint>,
}

impl DataFrame {
    pub fn new(logs: Vec<ChangeLog>, log_point: LogPoint) -> anyhow::Result<Self> {
        Ok(Self {
            logs,
            log_point: Some(log_point),
        })
    }
}

impl From<DataFrame> for pb::Data {
    fn from(val: DataFrame) -> Self {
        Self {
            logs: val.logs.into_iter().map(|i| i.into()).collect(),
            log_point: val.log_point.map(|log| log.into()),
        }
    }
}

impl From<pb::Data> for DataFrame {
    fn from(val: pb::Data) -> Self {
        DataFrame {
            logs: val.logs.into_iter().map(|i| i.into()).collect(),
            log_point: val.log_point.map(|log| log.into()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DataAck {
    pub log_point: Option<LogPoint>,
}

impl DataAck {
    pub fn new(log_point: LogPoint) -> anyhow::Result<Self> {
        Ok(Self {
            log_point: Some(log_point),
        })
    }
}

impl From<DataAck> for pb::Ack {
    fn from(val: DataAck) -> Self {
        Self {
            log_point: val.log_point.map(|l| l.into()),
        }
    }
}
