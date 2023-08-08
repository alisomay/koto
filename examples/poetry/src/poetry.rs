use indexmap::IndexMap;
use rand::{seq::SliceRandom, thread_rng, Rng};
use std::rc::Rc;

/// A basic Markov chain,
#[derive(Clone, Debug, Default)]
pub struct Poetry {
    links: IndexMap<Rc<str>, Vec<Rc<str>>>,
    previous: Option<Rc<str>>,
}

impl Poetry {
    pub fn add_source_material(&mut self, source: &str) {
        let mut words =
            source.split(|c: char| c.is_whitespace() || matches!(c, '(' | ')' | '[' | ']'));

        if let Some(first) = words.next() {
            let mut previous: Rc<str> = first.into();

            for word in words {
                if word.chars().any(char::is_alphabetic) {
                    let word: Rc<str> = word.into();
                    self.links
                        .entry(previous.clone())
                        .or_insert_with(Vec::new)
                        .push(word.clone());
                    previous = word;
                }
            }
        }
    }

    pub fn next_word(&mut self) -> Option<Rc<str>> {
        let result = self
            .previous
            .as_ref()
            .map(|previous| {
                // Given a previous word, find its links
                self.links
                    .get(previous)
                    .map(|words| {
                        // Given some links, choose the next word
                        let mut rng = thread_rng();
                        words.choose(&mut rng)
                    })
                    .unwrap_or(None)
            })
            .unwrap_or(None);

        let result = if let Some(result) = result {
            Some(result.clone())
        } else {
            // If no link was found, choose a new starting point
            let start = thread_rng().gen_range(0..self.links.len());
            self.links
                .get_index(start)
                .map(|(key, _value)| key)
                .cloned()
        };

        self.previous = result.clone();
        result
    }
}
