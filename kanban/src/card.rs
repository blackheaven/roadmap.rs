#[derive(PartialEq, Debug, Clone)]
pub struct Card {
    pub title: String,
}

impl Card {
    pub fn rename(&self, title: String) -> Card {
        return Card { title };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rename() {
        let old = Card { title: String::from("old") };
        assert_eq!(old.rename(String::from("new")), Card { title: String::from("new") });
    }
}
