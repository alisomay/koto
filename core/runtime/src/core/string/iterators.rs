//! A collection of string iterators

use crate::{
    make_runtime_error,
    value_iterator::{KotoIterator, ValueIterator, ValueIteratorOutput as Output},
    CallArgs, Result, Value, ValueString, Vm,
};
use unicode_segmentation::UnicodeSegmentation;

/// An iterator that outputs the individual bytes contained in a string
#[derive(Clone)]
pub struct Bytes {
    input: ValueString,
    index: usize,
}

impl Bytes {
    /// Creates a new [Bytes] iterator
    pub fn new(input: ValueString) -> Self {
        Self { input, index: 0 }
    }
}

impl KotoIterator for Bytes {
    fn make_copy(&self) -> Result<ValueIterator> {
        Ok(ValueIterator::new(self.clone()))
    }
}

impl Iterator for Bytes {
    type Item = Output;

    fn next(&mut self) -> Option<Self::Item> {
        match self.input.as_bytes().get(self.index) {
            Some(byte) => {
                self.index += 1;
                Some(Output::Value(byte.into()))
            }
            None => None,
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.input.len() - self.index;
        (remaining, Some(remaining))
    }
}

/// An iterator that yields the lines contained in a string
///
/// - Lines end with either `\r\n` or `\n`.
/// - Line end characters aren't included in the resulting output.
/// - Empty lines are yielded as empty strings.
#[derive(Clone)]
pub struct Lines {
    input: ValueString,
    start: usize,
}

impl Lines {
    /// Creates a new [Lines] iterator
    pub fn new(input: ValueString) -> Self {
        Self { input, start: 0 }
    }
}

impl KotoIterator for Lines {
    fn make_copy(&self) -> Result<ValueIterator> {
        Ok(ValueIterator::new(self.clone()))
    }
}

impl Iterator for Lines {
    type Item = Output;

    fn next(&mut self) -> Option<Self::Item> {
        let start = self.start;
        if start < self.input.len() {
            let mut newline_bytes = 1;
            let remaining = &self.input[start..];

            let end = match remaining.find('\n') {
                Some(end) => {
                    if end > 0 && remaining.as_bytes()[end - 1] == b'\r' {
                        newline_bytes += 1;
                        start + end - 1
                    } else {
                        start + end
                    }
                }
                None => self.input.len(),
            };

            let result = Value::Str(self.input.with_bounds(start..end).unwrap());
            self.start = end + newline_bytes;
            Some(Output::Value(result))
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining_bytes = self.input.len() - self.start;
        (1.min(remaining_bytes), Some(remaining_bytes))
    }
}

/// An iterator that splits up a string into parts, separated by a provided pattern
#[derive(Clone)]
pub struct Split {
    input: ValueString,
    pattern: ValueString,
    start: usize,
}

impl Split {
    /// Creates a new [Split] iterator
    pub fn new(input: ValueString, pattern: ValueString) -> Self {
        Self {
            input,
            pattern,
            start: 0,
        }
    }
}

impl KotoIterator for Split {
    fn make_copy(&self) -> Result<ValueIterator> {
        Ok(ValueIterator::new(self.clone()))
    }
}

impl Iterator for Split {
    type Item = Output;

    fn next(&mut self) -> Option<Self::Item> {
        let start = self.start;
        if start <= self.input.len() {
            let end = match self.input[start..].find(self.pattern.as_str()) {
                Some(end) => start + end,
                None => self.input.len(),
            };

            let output = Value::Str(self.input.with_bounds(start..end).unwrap());
            self.start = end + self.pattern.len();
            Some(Output::Value(output))
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining_bytes = self.input.len() - self.start;
        (1.min(remaining_bytes), Some(remaining_bytes))
    }
}

/// An iterator that splits up a string into parts, separated when a char passes a predicate
pub struct SplitWith {
    input: ValueString,
    predicate: Value,
    vm: Vm,
    start: usize,
}

impl SplitWith {
    /// Creates a new [SplitWith] iterator
    pub fn new(input: ValueString, predicate: Value, vm: Vm) -> Self {
        Self {
            input,
            predicate,
            vm,
            start: 0,
        }
    }
}

impl KotoIterator for SplitWith {
    fn make_copy(&self) -> Result<ValueIterator> {
        let result = Self {
            input: self.input.clone(),
            predicate: self.predicate.clone(),
            vm: self.vm.spawn_shared_vm(),
            start: self.start,
        };
        Ok(ValueIterator::new(result))
    }
}

impl Iterator for SplitWith {
    type Item = Output;

    fn next(&mut self) -> Option<Self::Item> {
        use Value::{Bool, Str};

        let start = self.start;
        if start < self.input.len() {
            let mut end = None;
            let mut grapheme_len = 0;

            for (grapheme_index, grapheme) in self.input[start..].grapheme_indices(true) {
                grapheme_len = grapheme.len();
                let grapheme_start = start + grapheme_index;
                let grapheme_end = grapheme_start + grapheme_len;
                let x = self
                    .input
                    .with_bounds(grapheme_start..grapheme_end)
                    .unwrap();
                match self
                    .vm
                    .run_function(self.predicate.clone(), CallArgs::Single(Str(x)))
                {
                    Ok(Bool(split_match)) => {
                        if split_match {
                            end = Some(grapheme_start);
                            break;
                        }
                    }
                    Ok(unexpected) => {
                        let error = make_runtime_error!(format!(
                            "string.split: Expected a Bool from the match function, found '{}'",
                            unexpected.type_as_string()
                        ));
                        return Some(Output::Error(error));
                    }
                    Err(error) => return Some(Output::Error(error.with_prefix("string.split"))),
                }
            }

            let end = end.unwrap_or(self.input.len());
            let output = Str(self.input.with_bounds(start..end).unwrap());
            self.start = end + grapheme_len;

            Some(Output::Value(output))
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining_bytes = self.input.len() - self.start;
        (1.min(remaining_bytes), Some(remaining_bytes))
    }
}
