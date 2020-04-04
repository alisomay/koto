mod constant_pool;
mod lookup;
mod node;
mod parser;
mod prec_climber;
pub mod vec4;

pub use lookup::*;
pub use node::*;
pub use parser::*;
pub use constant_pool::ConstantPool;

#[derive(Clone, Debug)]
pub struct AstNode {
    pub node: Node,
    pub start_pos: Position,
    pub end_pos: Position,
}

impl AstNode {
    pub fn new(span: pest::Span, node: Node) -> Self {
        let line_col = span.start_pos().line_col();
        let start_pos = Position {
            line: line_col.0,
            column: line_col.1,
        };
        let line_col = span.end_pos().line_col();
        let end_pos = Position {
            line: line_col.0,
            column: line_col.1,
        };
        Self {
            node,
            start_pos,
            end_pos,
        }
    }
}
impl Default for AstNode {
    fn default() -> Self {
        Self {
            node: Default::default(),
            start_pos: Position { line: 0, column: 0 },
            end_pos: Position { line: 0, column: 0 },
        }
    }
}
