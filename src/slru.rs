use lru::LruCache;
use std::borrow::Borrow;
use std::cmp;
use std::hash::Hash;
use std::num::NonZeroUsize;

pub(crate) struct SlruCache<K: Hash + Eq, V> {
	probationary_segment: LruCache<K, V>,
	protected_segment: LruCache<K, V>,
}

impl<K: Hash + Eq, V> SlruCache<K, V> {
	pub(crate) fn new(cap: usize) -> Self {
		let f64_cap = cap as f64;
		let probationary_cap = NonZeroUsize::new(cmp::max(1, (f64_cap * 0.2) as usize)).expect("non zero size");
		let protected_cap = NonZeroUsize::new(cmp::max(1, cap - probationary_cap.get())).expect("non zero size");

		Self {
			probationary_segment: LruCache::new(probationary_cap),
			protected_segment: LruCache::new(protected_cap),
		}
	}

	pub(crate) fn put(&mut self, k: K, v: V) -> Option<V> {
		if self.probationary_segment.contains(&k) {
			return self.probationary_segment.put(k, v);
		}

		if self.protected_segment.contains(&k) {
			return self.protected_segment.put(k, v);
		}

		self.probationary_segment.put(k, v)
	}

	pub(crate) fn push(&mut self, k: K, v: V) -> Option<(K, V)> {
		if self.probationary_segment.contains(&k) {
			return self.probationary_segment.push(k, v);
		}

		if self.protected_segment.contains(&k) {
			return self.protected_segment.push(k, v);
		}

		self.probationary_segment.push(k, v)
	}

	pub(crate) fn get<'a, Q>(&'a mut self, k: &Q) -> Option<&'a V>
	where
		K: Borrow<Q>,
		Q: Hash + Eq + ?Sized,
	{
		if let Some((k, v)) = self.probationary_segment.pop_entry(k) {
			if let Some((k, v)) = self.protected_segment.push(k, v) {
				self.probationary_segment.push(k, v);
			}
		}

		self.protected_segment.get(k)
	}

	pub(crate) fn get_mut<'a, Q>(&'a mut self, k: &Q) -> Option<&'a mut V>
	where
		K: Borrow<Q>,
		Q: Hash + Eq + ?Sized,
	{
		if let Some((k, v)) = self.probationary_segment.pop_entry(k) {
			if let Some((k, v)) = self.protected_segment.push(k, v) {
				self.probationary_segment.push(k, v);
			}
		}

		self.protected_segment.get_mut(k)
	}

	pub(crate) fn peek<'a, Q>(&'a self, k: &Q) -> Option<&'a V>
	where
		K: Borrow<Q>,
		Q: Hash + Eq + ?Sized,
	{
		match self.probationary_segment.peek(k) {
			Some(v) => Some(v),
			None => self.protected_segment.peek(k),
		}
	}

	pub(crate) fn peek_mut<'a, Q>(&'a mut self, k: &Q) -> Option<&'a mut V>
	where
		K: Borrow<Q>,
		Q: Hash + Eq + ?Sized,
	{
		match self.probationary_segment.peek_mut(k) {
			Some(v) => Some(v),
			None => self.protected_segment.peek_mut(k),
		}
	}

	#[inline]
	pub(crate) fn peek_lru<'a>(&'a self) -> Option<(&'a K, &'a V)> {
		match self.probationary_segment.peek_lru() {
			Some((k, v)) => Some((k, v)),
			None => self.protected_segment.peek_lru(),
		}
	}

	pub(crate) fn peek_lru_if_full<'a>(&'a self) -> Option<(&'a K, &'a V)> {
		if self.probationary_segment.len() != self.probationary_segment.cap().get() {
			return None;
		}

		self.probationary_segment.peek_lru()
	}

	pub(crate) fn contains<Q>(&self, k: &Q) -> bool
	where
		K: Borrow<Q>,
		Q: Hash + Eq + ?Sized,
	{
		match self.probationary_segment.contains(k) {
			true => true,
			false => self.protected_segment.contains(k),
		}
	}

	pub(crate) fn pop<Q>(&mut self, k: &Q) -> Option<V>
	where
		K: Borrow<Q>,
		Q: Hash + Eq + ?Sized,
	{
		match self.probationary_segment.pop(k) {
			Some(v) => Some(v),
			None => self.protected_segment.pop(k),
		}
	}

	pub(crate) fn pop_entry<Q>(&mut self, k: &Q) -> Option<(K, V)>
	where
		K: Borrow<Q>,
		Q: Hash + Eq + ?Sized,
	{
		match self.probationary_segment.pop_entry(k) {
			Some(v) => Some(v),
			None => self.protected_segment.pop_entry(k),
		}
	}

	pub(crate) fn pop_lru(&mut self) -> Option<(K, V)> {
		match self.probationary_segment.pop_lru() {
			Some((k, v)) => Some((k, v)),
			None => self.protected_segment.pop_lru(),
		}
	}

	pub(crate) fn len(&self) -> usize {
		self.probationary_segment.len() + self.protected_segment.len()
	}

	pub(crate) fn cap(&self) -> usize {
		self.probationary_segment.cap().get() + self.protected_segment.cap().get()
	}

	pub(crate) fn resize(&mut self, cap: usize) {
		let f64_cap = cap as f64;
		let probationary_cap = NonZeroUsize::new(cmp::max(1, (f64_cap * 0.2) as usize)).expect("non zero size");
		let protected_cap = NonZeroUsize::new(cmp::max(1, cap - probationary_cap.get())).expect("non zero size");

		self.probationary_segment.resize(probationary_cap);
		self.protected_segment.resize(protected_cap);
	}

	pub(crate) fn clear(&mut self) {
		self.probationary_segment.clear();
		self.protected_segment.clear();
	}
}

#[cfg(test)]
mod tests {
	use super::SlruCache;

	#[test]
	fn store_and_retrieve_items() {
		let mut cache = SlruCache::new(10);
		cache.push(1, "one");
		cache.push(2, "two");
		assert_eq!(cache.get(&1), Some(&"one"));
		assert_eq!(cache.get(&2), Some(&"two"));
	}

	#[test]
	fn store_retrieve_and_pop_items() {
		let mut cache = SlruCache::new(10);
		cache.push(1, "one");
		cache.push(2, "two");
		assert_eq!(cache.get(&1), Some(&"one"));
		assert_eq!(cache.get(&2), Some(&"two"));

		cache.pop(&1);
		assert_eq!(cache.get(&1), None);
		assert_eq!(cache.get(&2), Some(&"two"));
	}

	#[test]
	fn check_if_lru_is_correct() {
		let mut cache = SlruCache::new(25);
		cache.push(1, "one");
		cache.push(2, "two");
		cache.push(3, "three");
		cache.push(4, "four");
		cache.push(5, "five");
		assert_eq!(cache.peek_lru(), Some((&1, &"one")));

		cache.get(&1);
		cache.get(&2);
		cache.get(&3);
		cache.get(&4);
		cache.get(&5);
		assert_eq!(cache.peek_lru(), Some((&1, &"one")));

		cache.get(&3);
		cache.get(&2);
		cache.get(&4);
		cache.get(&1);
		cache.get(&5);
		assert_eq!(cache.peek_lru(), Some((&3, &"three")));
	}

	#[test]
	fn check_if_cap_and_len_are_correct() {
		let mut cache = SlruCache::new(10);
		cache.push(1, "one");
		cache.push(2, "two");
		assert_eq!(cache.cap(), 10);
		assert_eq!(cache.len(), 2);

		cache.get(&1);
		cache.get(&2);
		assert_eq!(cache.cap(), 10);
		assert_eq!(cache.len(), 2);

		cache.push(3, "three");
		assert_eq!(cache.cap(), 10);
		assert_eq!(cache.len(), 3);

		cache.get(&3);
		assert_eq!(cache.cap(), 10);
		assert_eq!(cache.len(), 3);
	}

	#[test]
	fn clear_cache() {
		let mut cache = SlruCache::new(10);
		cache.push(1, "one");
		cache.push(2, "two");
		assert_eq!(cache.get(&1), Some(&"one"));
		assert_eq!(cache.get(&2), Some(&"two"));
		assert_eq!(cache.len(), 2);
		assert_eq!(cache.cap(), 10);

		cache.clear();
		assert_eq!(cache.get(&1), None);
		assert_eq!(cache.get(&2), None);
		assert_eq!(cache.len(), 0);
		assert_eq!(cache.cap(), 10);
	}
}
