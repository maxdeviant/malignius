pub struct Sequence<T> {
    counter: usize,
    produce: Box<dyn Fn(usize) -> T>,
}

impl<T> Sequence<T> {
    pub fn new(produce: impl Fn(usize) -> T + 'static) -> Self {
        Self {
            counter: 1,
            produce: Box::new(produce),
        }
    }

    /// Returns the next value in the sequence.
    pub fn next(&mut self) -> T {
        let n = self.counter;
        self.counter += 1;

        (self.produce)(n)
    }

    /// Returns the next *n* values in the sequence.
    pub fn take(&mut self, n: usize) -> Vec<T> {
        let mut values = Vec::with_capacity(n);

        for _ in 0..n {
            values.push(self.next());
        }

        values
    }
}

#[cfg(test)]
mod tests {
    use crate::sequence::Sequence;

    #[test]
    fn next_produces_a_value() {
        let mut emails = Sequence::new(|n| format!("user{n}@example.com"));

        assert_eq!(emails.next(), "user1@example.com");
        assert_eq!(emails.next(), "user2@example.com");
        assert_eq!(emails.next(), "user3@example.com");
        assert_eq!(emails.next(), "user4@example.com");
    }

    #[test]
    fn take_produces_multiple_values() {
        let mut usernames = Sequence::new(|n| format!("jsmith{n}"));

        assert_eq!(usernames.take(3), vec!["jsmith1", "jsmith2", "jsmith3"]);
        assert_eq!(usernames.take(2), vec!["jsmith4", "jsmith5"]);
    }
}
