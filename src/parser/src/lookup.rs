use crate::{AstIndex, ConstantIndex};

// TODO are the Index / Lookup types still useful?
#[derive(Clone, Debug, PartialEq)]
pub struct Index(pub AstIndex);

#[derive(Clone, Debug, PartialEq)]
pub struct Lookup(pub Vec<LookupNode>);

#[derive(Clone, Debug, PartialEq)]
pub enum LookupNode {
    Id(ConstantIndex),
    Index(Index),
    Call(Vec<AstIndex>),
}

impl Lookup {
    pub fn as_slice(&self) -> LookupSlice {
        LookupSlice(self.0.as_slice())
    }

    pub fn value_slice(&self) -> LookupSlice {
        LookupSlice(&self.0[self.0.len() - 1..])
    }

    pub fn value_node(&self) -> &LookupNode {
        &self.0[self.0.len() - 1]
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LookupSlice<'a>(pub &'a [LookupNode]);

impl<'a> LookupSlice<'a> {
    pub fn value_slice(&self) -> LookupSlice {
        LookupSlice(&self.0[self.0.len() - 1..])
    }

    pub fn value_node(&self) -> &LookupNode {
        &self.0[self.0.len() - 1]
    }

    pub fn first_n(&self, n: usize) -> LookupSlice {
        LookupSlice(&self.0[..=n])
    }
}
