use std::{collections::BTreeMap, ops::RangeInclusive};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(i32);

impl Timestamp {
    pub fn new(v: i32) -> Self {
        Self(v)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Price(i32);

impl Price {
    pub fn new(v: i32) -> Self {
        Self(v)
    }

    pub fn inner(&self) -> i32 {
        self.0
    }
}

pub trait Aggregator {
    fn add(&mut self, price: Price);
    fn result(&self) -> Price;
}

#[derive(Default)]
pub struct Mean {
    sum: i64,
    count: i64,
}

impl Aggregator for Mean {
    fn add(&mut self, price: Price) {
        self.sum += price.inner() as i64;
        self.count += 1;
    }

    fn result(&self) -> Price {
        if self.count == 0 {
            Price::new(0)
        } else {
            let mean = self.sum / self.count;
            // saturating to mantain clean api and no panic
            let saturated = mean.clamp(i32::MIN as i64, i32::MAX as i64);

            Price::new(saturated as i32)
        }
    }
}

#[derive(Default)]
pub struct State(BTreeMap<Timestamp, Price>);

impl State {
    pub fn insert(&mut self, timestamp: Timestamp, price: Price) {
        self.0.insert(timestamp, price);
    }

    pub fn aggregate<A>(&self, range: RangeInclusive<Timestamp>) -> Price
    where
        A: Aggregator + Default,
    {
        if range.start() > range.end() {
            return Price(0);
        }

        let mut aggregator = A::default();

        for (_, price) in self.0.range(range) {
            aggregator.add(*price);
        }

        aggregator.result()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mean() {
        let mut state = State::default();
        state.insert(Timestamp::new(0), Price::new(10));
        state.insert(Timestamp::new(10), Price::new(20));
        state.insert(Timestamp::new(20), Price::new(30));

        assert_eq!(
            state.aggregate::<Mean>(Timestamp::new(0)..=Timestamp::new(20)),
            Price::new(20)
        );
        assert_eq!(
            state.aggregate::<Mean>(Timestamp::new(0)..=Timestamp::new(10)),
            Price::new(15)
        );
        assert_eq!(
            state.aggregate::<Mean>(Timestamp::new(5)..=Timestamp::new(15)),
            Price::new(20)
        );
    }

    #[test]
    fn test_mean_empty_range() {
        let mut state = State::default();
        state.insert(Timestamp::new(0), Price::new(10));
        state.insert(Timestamp::new(10), Price::new(20));
        state.insert(Timestamp::new(20), Price::new(30));

        assert_eq!(
            state.aggregate::<Mean>(Timestamp::new(10)..=Timestamp::new(10)),
            Price::new(20)
        );

        assert_eq!(
            state.aggregate::<Mean>(Timestamp::new(20)..=Timestamp::new(0)),
            Price::new(0)
        );
    }
}
