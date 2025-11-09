pub struct StackVec<'a, T> {
    vec: &'a mut Vec<T>,
    pushed: usize,
}

impl<'a, T> StackVec<'a, T> {
    pub fn new(vec: &'a mut Vec<T>) -> Self {
        Self { vec, pushed: 0 }
    }

    pub fn push(&mut self, value: T) {
        self.pushed += 1;
        self.vec.push(value);
    }

    pub fn inner(&mut self) -> &mut Vec<T> {
        self.vec
    }
}

impl<'a, T> Drop for StackVec<'a, T> {
    fn drop(&mut self) {
        while self.pushed > 0 {
            self.vec.pop();
            self.pushed -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_auto_pop() {
        let mut vec = vec![1, 2, 3];
        {
            let mut stack_vec = StackVec::new(&mut vec);
            stack_vec.push(4);
            stack_vec.push(5);
            assert_eq!(stack_vec.inner(), &mut vec![1, 2, 3, 4, 5]);
        }
        // After drop, pushed elements should be removed
        assert_eq!(vec, vec![1, 2, 3]);
    }

    #[test]
    fn test_empty_vec() {
        let mut vec: Vec<i32> = Vec::new();
        {
            let mut stack_vec = StackVec::new(&mut vec);
            stack_vec.push(1);
            stack_vec.push(2);
            assert_eq!(stack_vec.inner(), &mut vec![1, 2]);
        }
        assert_eq!(vec, Vec::<i32>::new());
    }

    #[test]
    fn test_no_push() {
        let mut vec = vec![1, 2, 3];
        {
            let _stack_vec = StackVec::new(&mut vec);
            // Don't push anything
        }
        // Original vec should remain unchanged
        assert_eq!(vec, vec![1, 2, 3]);
    }

    #[test]
    fn test_nested_stack_vecs() {
        let mut vec = vec![1, 2];
        {
            let mut stack_vec1 = StackVec::new(&mut vec);
            stack_vec1.push(3);
            {
                let mut stack_vec2 = StackVec::new(stack_vec1.inner());
                stack_vec2.push(4);
                stack_vec2.push(5);
                assert_eq!(stack_vec2.inner(), &mut vec![1, 2, 3, 4, 5]);
            }
            // After inner stack_vec2 drops
            assert_eq!(stack_vec1.inner(), &mut vec![1, 2, 3]);
        }
        // After outer stack_vec1 drops
        assert_eq!(vec, vec![1, 2]);
    }
}
