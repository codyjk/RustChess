/// A generic stack for managing state that needs to be pushed/popped during move application/undo.
#[derive(Clone)]
pub struct StateStack<T> {
    stack: Vec<T>,
}

impl<T: Clone> StateStack<T> {
    pub fn new(initial: T) -> Self {
        Self {
            stack: vec![initial],
        }
    }

    pub fn push(&mut self, value: T) -> T {
        let cloned = value.clone();
        self.stack.push(value);
        cloned
    }

    pub fn peek(&self) -> &T {
        self.stack.last().expect("StateStack should never be empty")
    }

    pub fn pop(&mut self) -> T {
        self.stack.pop().expect("StateStack should never be empty")
    }
}

