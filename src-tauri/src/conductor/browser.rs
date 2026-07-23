#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectionStop {
    Continue,
    ItemLimit,
    StepLimit,
    NoNewItems,
}

#[derive(Debug, Clone)]
pub struct CollectionTracker {
    maximum_items: u32,
    maximum_steps: u32,
    item_count: u32,
    step: u32,
    no_new_steps: u8,
}

impl CollectionTracker {
    pub fn new(maximum_items: u32, maximum_steps: u32) -> Self {
        Self {
            maximum_items,
            maximum_steps,
            item_count: 0,
            step: 0,
            no_new_steps: 0,
        }
    }

    pub fn record(&mut self, new_items: u32) -> CollectionStop {
        self.step = self.step.saturating_add(1);
        self.item_count = self.item_count.saturating_add(new_items);
        self.no_new_steps = if new_items == 0 {
            self.no_new_steps.saturating_add(1)
        } else {
            0
        };
        if self.item_count >= self.maximum_items {
            CollectionStop::ItemLimit
        } else if self.step >= self.maximum_steps {
            CollectionStop::StepLimit
        } else if self.no_new_steps >= 3 {
            CollectionStop::NoNewItems
        } else {
            CollectionStop::Continue
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CollectionStop, CollectionTracker};

    #[test]
    fn collection_stops_deterministically() {
        let mut tracker = CollectionTracker::new(50, 25);
        assert_eq!(tracker.record(0), CollectionStop::Continue);
        assert_eq!(tracker.record(0), CollectionStop::Continue);
        assert_eq!(tracker.record(0), CollectionStop::NoNewItems);
        let mut item_limited = CollectionTracker::new(2, 25);
        assert_eq!(item_limited.record(2), CollectionStop::ItemLimit);
    }
}
