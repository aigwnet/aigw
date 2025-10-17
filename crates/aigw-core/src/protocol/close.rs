use super::pb;

#[derive(Clone, Debug)]
pub struct Close {}

impl From<&Close> for pb::Close {
    fn from(_val: &Close) -> Self {
        pb::Close {}
    }
}
