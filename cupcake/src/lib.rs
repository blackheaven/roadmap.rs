struct Cupcake {}
struct Cookie {}

struct Chocolate<'a, T> { wrapped: &'a T }
struct Nuts<'a, T> { wrapped: &'a T }
struct Sugar<'a, T> { wrapped: &'a T }

trait Food {
    fn name(&self) -> String;
    fn price_usd_cents(&self) -> u16;
}

impl Food for Cupcake {
    fn name(&self) -> String { String::from("cupcake") }
    fn price_usd_cents(&self) -> u16 { 100 }
}

impl Food for Cookie {
    fn name(&self) -> String { String::from("cookie") }
    fn price_usd_cents(&self) -> u16 { 200 }
}

fn add_topping_name(base: String, topping: &str) -> String {
    if base.contains("with") {
        base + " and " + topping
    } else {
        base + " with " + topping
    }
}

impl<T: Food> Food for Chocolate<'_, T> {
    fn name(&self) -> String { add_topping_name(self.wrapped.name(), "chocolate") }
    fn price_usd_cents(&self) -> u16 { self.wrapped.price_usd_cents() + 10 }
}

impl<T: Food> Food for Sugar<'_, T> {
    fn name(&self) -> String { add_topping_name(self.wrapped.name(), "sugar") }
    fn price_usd_cents(&self) -> u16 { self.wrapped.price_usd_cents() + 10 }
}

impl<T: Food> Food for Nuts<'_, T> {
    fn name(&self) -> String { add_topping_name(self.wrapped.name(), "nuts") }
    fn price_usd_cents(&self) -> u16 { self.wrapped.price_usd_cents() + 20 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_cupcake() {
        let food = Cupcake{};
        assert_eq!(food.name(), "cupcake");
        assert_eq!(food.price_usd_cents(), 100);
    }

    #[test]
    fn natural_cookie() {
        let food = Cookie{};
        assert_eq!(food.name(), "cookie");
        assert_eq!(food.price_usd_cents(), 200);
    }

    #[test]
    fn chocolate_cupcake() {
        let food = Chocolate{ wrapped: &Cupcake{} };
        assert_eq!(food.name(), "cupcake with chocolate");
        assert_eq!(food.price_usd_cents(), 110);
    }

    #[test]
    fn chocolate_sugar_nuts_cupcake() {
        let food = Nuts{ wrapped: &Sugar{ wrapped: &Chocolate{ wrapped: &Cupcake{} }}};
        assert_eq!(food.name(), "cupcake with chocolate and sugar and nuts");
        assert_eq!(food.price_usd_cents(), 140);
    }

    #[test]
    fn chocolate_cookie() {
        let food = Chocolate{ wrapped: &Cookie{} };
        assert_eq!(food.name(), "cookie with chocolate");
        assert_eq!(food.price_usd_cents(), 210);
    }

}
