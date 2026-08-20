#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ActiveTurn {
    thread_id: String,
    turn_id: String,
}

impl ActiveTurn {
    pub(crate) fn new(thread_id: String, turn_id: String) -> Self {
        Self { thread_id, turn_id }
    }

    pub(crate) fn thread_id(&self) -> &str {
        &self.thread_id
    }

    pub(crate) fn turn_id(&self) -> &str {
        &self.turn_id
    }

    #[cfg(test)]
    pub(crate) fn synthetic(label: &str) -> Self {
        Self::new(format!("thread-{label}"), format!("turn-{label}"))
    }
}
