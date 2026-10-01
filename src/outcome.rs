#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Delivered { note: Option<String> },
    Failed(String),
    MayNotHaveLanded(String),
}

impl Outcome {
    pub fn delivered() -> Self {
        Self::Delivered { note: None }
    }

    pub fn failed(reason: impl Into<String>) -> Self {
        Self::Failed(reason.into())
    }

    pub fn may_not_have_landed(reason: impl Into<String>) -> Self {
        Self::MayNotHaveLanded(reason.into())
    }
}
