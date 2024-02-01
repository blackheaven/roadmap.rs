pub fn fizzbuzz(n: usize) -> String {
    if n % 15 == 0 {
        return String::from("FizzBuzz")
    }
    if n % 3 == 0 {
        return String::from("Fizz")
    }
    if n % 5 == 0 {
        return String::from("Buzz")
    }
    n.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is1() {
        assert_eq!(fizzbuzz(1), "1");
    }

    #[test]
    fn is2() {
        assert_eq!(fizzbuzz(2), "2");
    }

    #[test]
    fn is3() {
        assert_eq!(fizzbuzz(3), "Fizz");
    }

    #[test]
    fn is5() {
        assert_eq!(fizzbuzz(5), "Buzz");
    }

    #[test]
    fn is6() {
        assert_eq!(fizzbuzz(3), "Fizz");
    }

    #[test]
    fn is10() {
        assert_eq!(fizzbuzz(10), "Buzz");
    }

    #[test]
    fn is15() {
        assert_eq!(fizzbuzz(15), "FizzBuzz");
    }

    #[test]
    fn is30() {
        assert_eq!(fizzbuzz(30), "FizzBuzz");
    }

}
