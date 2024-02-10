use crate::column::Column;
use crate::containers::Container;

#[allow(dead_code)]
pub type Board = Container<Column>;
#[allow(unused_imports)]
pub use crate::containers::MoveSpec;

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
}
