use crate::card::Card;
use crate::containers::Container;

#[allow(dead_code)]
pub type Column = Container<Card>;
#[allow(unused_imports)]
pub use crate::containers::MoveSpec;

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
}
