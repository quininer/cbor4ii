pub struct ScopeGuard<'a, T>(pub &'a mut T, pub fn(&mut T));

impl<T> ScopeGuard<'_, T> {
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> Drop for ScopeGuard<'_, T> {
    fn drop(&mut self) {
        (self.1)(self.0);
    }
}
