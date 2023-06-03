use super::*;

#[derive(PartialEq, Eq, Clone, Copy, Hash, Serialize, Deserialize)]
pub struct Id(u64);

impl Id {
    pub fn raw(&self) -> u64 {
        self.0
    }
}

impl Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "#{}", self.0)
    }
}

pub struct Gen {
    next_id: u64,
}

impl Gen {
    pub fn new() -> Self {
        Self { next_id: 0 }
    }
    pub fn gen(&mut self) -> Id {
        let id = Id(self.next_id);
        self.next_id += 1;
        id
    }
}
