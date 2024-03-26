#[derive(Debug, PartialEq, Clone, Eq, PartialOrd, Ord)]
pub struct Name {
    name: String,
}

impl Name {
    fn capitalize_word(original: String) -> String {
        let mut chars = original.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first
                .to_uppercase()
                .chain(chars.map(|c| c.to_ascii_lowercase()))
                .collect(),
        }
    }

    fn capitalize(original: String) -> String {
        original
            .split_inclusive(&['-', ' '])
            .map(String::from)
            .map(Self::capitalize_word)
            .collect()
    }
    pub fn new(original: &str) -> Name {
        Name {
            name: Self::capitalize(String::from(original)),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Age {
    age: u8,
}

impl Age {
    pub fn new(age: u8) -> Age {
        Age { age }
    }
    pub fn is_adult(&self) -> bool {
        self.age >= 18
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Employee {
    name: Name,
    age: Age,
}

impl Employee {
    pub fn is_adult(&self) -> bool {
        self.age.is_adult()
    }
}

#[derive(Debug, PartialEq)]
pub struct Employees {
    employees: Vec<Employee>,
}

impl Employees {
    fn adults(&self) -> Employees {
        Employees {
            employees: Vec::from_iter(
                self.employees
                    .clone()
                    .into_iter()
                    .filter(Employee::is_adult),
            ),
        }
    }
    pub fn sunday_workers(&self) -> Employees {
        let mut employees = self.adults();
        employees
            .employees
            .sort_by_key(|employee| employee.name.clone());
        return employees;
    }
}

pub fn employees() -> Employees {
    Employees {
        employees: vec![
            Employee {
                name: Name::new("max"),
                age: Age::new(17),
            },
            Employee {
                name: Name::new("SePP lUça"),
                age: Age::new(18),
            },
            Employee {
                name: Name::new("NiNa"),
                age: Age::new(15),
            },
            Employee {
                name: Name::new("jEan-mIkE"),
                age: Age::new(51),
            },
        ],
    }
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_list_employees_sorted_by_name() {
        let employees = employees();
        assert_eq!(
            employees.sunday_workers(),
            Employees {
                employees: vec![
                    Employee {
                        name: Name::new("Jean-Mike"),
                        age: Age::new(51),
                    },
                    Employee {
                        name: Name::new("Sepp Luça"),
                        age: Age::new(18),
                    },
                ]
            }
        );
    }
}
