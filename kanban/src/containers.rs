#[derive(PartialEq, Debug, Clone)]
pub struct Container<T> {
    pub title: String,
    pub items: Vec<T>,
}

pub enum MoveSpec {
    Beginning,
    End,
    After(usize),
}

impl<T> Container<T> {
    pub fn rename(&mut self, new_title: String) {
        self.title = new_title;
    }
}

impl<T: Clone> Container<T> {
    pub fn add_item(&mut self, item: T) {
        self.items.insert(0, item);
    }

    pub fn move_item(&mut self, origin: usize, spec: MoveSpec) -> bool {
        if origin >= self.items.len() {
            return false
        }
        let element = self.items[origin].clone();
        self.items.remove(origin);

        match spec {
            MoveSpec::Beginning => self.items.insert(0, element),
            MoveSpec::End => self.items.push(element),
            MoveSpec::After(n) => self.items.insert(if n < origin {n + 1} else {n}, element),
        };

        return true;
    }

    pub fn update_item(&mut self, origin: usize, f: impl Fn(T) -> T) -> bool {
        if origin >= self.items.len() {
            return false
        }

        self.items[origin] = f(self.items[origin].clone());
        return true;
    }

    pub fn remove_item(&mut self, index: usize) -> bool {
        if index >= self.items.len() {
            return false
        }

        self.items.remove(index);
        return true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_column(title: &str, titles: Vec<&str>) -> Container<String> {
        return Container {
            title: String::from(title),
            items: Vec::from_iter(titles.iter().map(|t| String::from(*t)))
        };
    }

    #[test]
    fn rename() {
        let mut old = mk_column("t", vec!["0", "1"]);
        old.rename(String::from("!"));
        assert_eq!(old, mk_column("!", vec!["0", "1"]));
    }

    #[test]
    fn add() {
        let mut old = mk_column("t", vec!["0"]);
        old.add_item(String::from("1"));
        assert_eq!(old, mk_column("t", vec!["1", "0"]));
    }

    #[test]
    fn move_beginning() {
        let mut old = mk_column("t", vec!["0", "1"]);
        assert_eq!(true, old.move_item(1, MoveSpec::Beginning));
        assert_eq!(old, mk_column("t", vec!["1", "0"]));
    }

    #[test]
    fn move_end() {
        let mut old = mk_column("t", vec!["0", "1"]);
        assert_eq!(true, old.move_item(0, MoveSpec::End));
        assert_eq!(old, mk_column("t", vec!["1", "0"]));
    }

    #[test]
    fn move_after_from_end() {
        let mut old = mk_column("t", vec!["0", "1", "2"]);
        assert_eq!(true, old.move_item(2, MoveSpec::After(0)));
        assert_eq!(old, mk_column("t", vec!["0", "2", "1"]));
    }

    #[test]
    fn move_after_from_beginning() {
        let mut old = mk_column("t", vec!["0", "1", "2"]);
        assert_eq!(true, old.move_item(0, MoveSpec::After(1)));
        assert_eq!(old, mk_column("t", vec!["1", "0", "2"]));
    }

    #[test]
    fn move_out_of_indexes() {
        let mut old = mk_column("t", vec!["0", "1", "2"]);
        assert_eq!(false, old.move_item(3, MoveSpec::After(1)));
        assert_eq!(old, mk_column("t", vec!["0", "1", "2"]));
    }

    #[test]
    fn update_existing() {
        let mut old = mk_column("t", vec!["0", "1", "2"]);
        assert_eq!(true, old.update_item(1, |_| String::from("!") ));
        assert_eq!(old, mk_column("t", vec!["0", "!", "2"]));
    }

    #[test]
    fn update_missing() {
        let mut old = mk_column("t", vec!["0", "1", "2"]);
        assert_eq!(false, old.update_item(3, |_| String::from("!") ));
        assert_eq!(old, mk_column("t", vec!["0", "1", "2"]));
    }

    #[test]
    fn remove_existing() {
        let mut old = mk_column("t", vec!["0", "1", "2"]);
        assert_eq!(true, old.remove_item(1));
        assert_eq!(old, mk_column("t", vec!["0", "2"]));
    }

    #[test]
    fn remove_missing() {
        let mut old = mk_column("t", vec!["0", "1", "2"]);
        assert_eq!(false, old.remove_item(3));
        assert_eq!(old, mk_column("t", vec!["0", "1", "2"]));
    }
}
