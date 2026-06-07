#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GraveId(i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PersonId(i64);

impl GraveId {
    pub fn new(value: i64) -> Self {
        Self(value)
    }

    pub fn value(self) -> i64 {
        self.0
    }
}

impl std::fmt::Display for GraveId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value())
    }
}

impl PersonId {
    pub fn new(value: i64) -> Self {
        Self(value)
    }

    pub fn value(self) -> i64 {
        self.0
    }
}

impl std::fmt::Display for PersonId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value())
    }
}
