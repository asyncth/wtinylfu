#![forbid(unsafe_code)]

mod slru;

use bloomfilter::Bloom;
use count_min_sketch::CountMinSketch16;
use lru::LruCache;
use slru::SlruCache;
use std::cmp;
use std::hash::Hash;
use std::num::NonZeroUsize;

/// W-TinyLFU cache that uses Count Min Sketch as an approximation sketch.
pub struct WTinyLfuCache<K: Hash + Eq, V> {
	approximation_sketch: CountMinSketch16<K>,
	sample_size: usize,
	sample_counter: usize,
	doorkeeper: Bloom<K>,
	window_cache: LruCache<K, V>,
	main_cache: SlruCache<K, V>,
}

impl<K: Hash + Eq, V> WTinyLfuCache<K, V> {
	/// Creates an W-TinyLFU cache that can hold up to `cap` key-value pairs.
	pub fn new(cap: usize, sample_size: usize) -> Self {
		let f64_cap: f64 = cap as f64;
		let window_cache_cap =
			NonZeroUsize::new(cmp::max(1, (f64_cap * 0.01) as usize)).expect("non zero");
		let main_cache_cap = cmp::max(1, cap - window_cache_cap.get());

		Self {
			approximation_sketch: CountMinSketch16::new(sample_size * 2, 0.97, 4.0).unwrap(),
			sample_size,
			sample_counter: 0,
			doorkeeper: Bloom::new_for_fp_rate(sample_size, 0.01),
			window_cache: LruCache::new(window_cache_cap),
			main_cache: SlruCache::new(main_cache_cap),
		}
	}

	/// Inserts a new key-value pair or updates it if a pair with the same key exists, returning the old value.
	/// Otherwise, returns `None`.
	pub fn put(&mut self, k: K, v: V) -> Option<V> {
		if self.window_cache.contains(&k) {
			return self.window_cache.put(k, v);
		}

		if self.main_cache.contains(&k) {
			return self.main_cache.put(k, v);
		}

		self.push(k, v);
		None
	}

	/// Inserts a new key-value pair or updates it if a pair with the same key exists, returning the the old pair.
	/// Returns the evicted key-value pair if there is one.
	/// Otherwise, returns `None`.
	pub fn push(&mut self, k: K, v: V) -> Option<(K, V)> {
		if self.window_cache.contains(&k) {
			return self.window_cache.push(k, v);
		}

		if self.main_cache.contains(&k) {
			return self.main_cache.push(k, v);
		}

		match self.window_cache.push(k, v) {
			Some((window_cache_victim_k, window_cache_victim_v)) => {
				match self.main_cache.peek_lru_if_full() {
					Some((main_cache_victim_k, _)) => {
						let window_cache_victim_estimation = self.estimate(&window_cache_victim_k);
						let main_cache_victim_estimation = self.estimate(main_cache_victim_k);

						if window_cache_victim_estimation > main_cache_victim_estimation {
							return self
								.main_cache
								.push(window_cache_victim_k, window_cache_victim_v);
						}

						Some((window_cache_victim_k, window_cache_victim_v))
					}
					None => self
						.main_cache
						.push(window_cache_victim_k, window_cache_victim_v),
				}
			}
			None => None,
		}
	}

	/// Retrieves a value for the specified key from the cache and returns an immutable reference if it exists.
	/// If such key-value pair exists, its count in the approximation sketch is incremented.
	/// Otherwise, returns `None`.
	pub fn get(&mut self, k: &K) -> Option<&V> {
		let v = match self.window_cache.get(k) {
			Some(v) => Some(v),
			None => self.main_cache.get(k),
		};

		if v.is_some() {
			if self.doorkeeper.check(k) {
				self.approximation_sketch.increment(k);
				self.sample_counter += 1;

				if self.sample_counter >= self.sample_size {
					self.approximation_sketch.reset();
					self.doorkeeper.clear();
					self.sample_counter = 0;
				}
			} else {
				self.doorkeeper.set(k);
			}
		}

		v
	}

	/// Retrieves a value for the specified key from the cache and returns a mutable reference if it exists.
	/// If such key-value pair exists, its count in the approximation sketch is incremented.
	/// Otherwise, returns `None`.
	pub fn get_mut(&mut self, k: &K) -> Option<&mut V> {
		let v = match self.window_cache.get_mut(k) {
			Some(v) => Some(v),
			None => self.main_cache.get_mut(k),
		};

		if v.is_some() {
			if self.doorkeeper.check(k) {
				self.approximation_sketch.increment(k);
				self.sample_counter += 1;

				if self.sample_counter >= self.sample_size {
					self.approximation_sketch.reset();
					self.doorkeeper.clear();
					self.sample_counter = 0;
				}
			} else {
				self.doorkeeper.set(k);
			}
		}

		v
	}

	/// Retrieves a value for the specified key from the cache and returns an immutable reference if it exists.
	/// Does not increment pair's count in the approximation sketch.
	/// If the pair doesn't exist, returns `None`.
	pub fn peek(&self, k: &K) -> Option<&V> {
		match self.window_cache.peek(k) {
			Some(v) => Some(v),
			None => self.main_cache.peek(k),
		}
	}

	/// Retrieves a value for the specified key from the cache and returns a mutable reference if it exists.
	/// Does not increment pair's count in the approximation sketch.
	/// If the pair doesn't exist, returns `None`.
	pub fn peek_mut(&mut self, k: &K) -> Option<&mut V> {
		match self.window_cache.peek_mut(k) {
			Some(v) => Some(v),
			None => self.main_cache.peek_mut(k),
		}
	}

	/// Returns a reference to the least recently used key-value pair from the window cache.
	/// Returns `None` if the cache is empty.
	#[inline]
	pub fn peek_lru_window(&self) -> Option<(&K, &V)> {
		self.window_cache.peek_lru()
	}

	/// Returns a reference to the least recently used key-value pair from the main cache.
	/// Returns `None` if the cache is empty.
	#[inline]
	pub fn peek_lru_main(&self) -> Option<(&K, &V)> {
		self.main_cache.peek_lru()
	}

	/// Returns a bool indicating whether a key-value pair stored in the cache.
	pub fn contains(&self, k: &K) -> bool {
		match self.window_cache.contains(k) {
			true => true,
			false => self.main_cache.contains(k),
		}
	}

	/// Removes a key-value pair with the specified key and returns pair's value.
	pub fn pop(&mut self, k: &K) -> Option<V> {
		match self.window_cache.pop(k) {
			Some(v) => Some(v),
			None => self.main_cache.pop(k),
		}
	}

	/// Removes a key-value pair with the specified key and returns the pair.
	pub fn pop_entry(&mut self, k: &K) -> Option<(K, V)> {
		match self.window_cache.pop_entry(k) {
			Some(v) => Some(v),
			None => self.main_cache.pop_entry(k),
		}
	}

	/// Removes the least recently used key-value pair from the window cache and returns the pair.
	pub fn pop_lru_window(&mut self) -> Option<(K, V)> {
		self.window_cache.pop_lru()
	}

	/// Removes the least recently used key-value pair from the main cache and returns the pair.
	pub fn pop_lru_main(&mut self) -> Option<(K, V)> {
		self.main_cache.pop_lru()
	}

	/// Returns the number of stored key-value pairs.
	pub fn len(&self) -> usize {
		self.window_cache.len() + self.main_cache.len()
	}

	/// Returns a bool indicating whether the cache is empty.
	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}

	/// Returns the capacity of the cache (the maximum number of key-value pairs that the cache can store).
	pub fn cap(&self) -> usize {
		self.window_cache.cap().get() + self.main_cache.cap()
	}

	/// Resizes the cache. If the new capacity is smaller than the size of the current cache any entries past the new capacity are discarded.
	pub fn resize(&mut self, cap: usize) {
		let f64_cap: f64 = cap as f64;
		let window_cache_cap =
			NonZeroUsize::new(cmp::max(1, (f64_cap * 0.01) as usize)).expect("non zero size");
		let main_cache_cap = cmp::max(1, cap - window_cache_cap.get());

		self.window_cache.resize(window_cache_cap);
		self.main_cache.resize(main_cache_cap);
	}

	/// Removes all key-value pairs from the cache.
	pub fn clear(&mut self) {
		self.window_cache.clear();
		self.main_cache.clear();
	}

	#[inline]
	fn estimate(&self, k: &K) -> u16 {
		let mut estimate = self.approximation_sketch.estimate(k);
		if self.doorkeeper.check(k) {
			estimate += 1;
		}

		estimate
	}

	/// An iterator visiting all entries in roughly most-recently used order.
	///
	/// # Examples
	///
	/// ```
	/// use wtinylfu::WTinyLfuCache;
	///
	/// let mut cache = WTinyLfuCache::new(3, 10);
	/// cache.put("a", 1);
	/// cache.put("b", 2);
	/// cache.put("c", 3);
	///
	/// for (key, val) in cache.iter() {
	///     println!("key: {} val: {}", key, val);
	/// }
	/// ```
	pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
		self.window_cache.iter().chain(self.main_cache.iter())
	}
}

#[cfg(test)]
mod tests {
	use super::WTinyLfuCache;
	use std::hash::Hash;

	fn iter_keys<K: Hash + Eq + Ord + Copy, V>(cache: &WTinyLfuCache<K, V>) -> Vec<K> {
		let mut out = cache.iter().map(|(k, _)| *k).collect::<Vec<_>>();
		out.sort();
		out
	}

	#[test]
	fn store_and_retrieve_items() {
		let mut cache = WTinyLfuCache::new(2, 10);
		cache.push(1, "one");
		cache.push(2, "two");
		assert_eq!(cache.get(&1), Some(&"one"));
		assert_eq!(cache.get(&2), Some(&"two"));
		assert_eq!(&iter_keys(&cache), &[1, 2]);
	}

	#[test]
	fn store_retrieve_and_pop_items() {
		let mut cache = WTinyLfuCache::new(2, 10);
		cache.push(1, "one");
		cache.push(2, "two");
		assert_eq!(cache.get(&1), Some(&"one"));
		assert_eq!(cache.get(&2), Some(&"two"));

		cache.pop(&1);
		assert_eq!(cache.get(&1), None);
		assert_eq!(cache.get(&2), Some(&"two"));
		assert_eq!(&iter_keys(&cache), &[2]);
	}

	#[test]
	fn check_if_lru_is_correct() {
		let mut cache = WTinyLfuCache::new(500, 10);
		cache.push(1, "one");
		cache.push(2, "two");
		cache.push(3, "three");
		cache.push(4, "four");
		cache.push(5, "five");
		assert_eq!(cache.peek_lru_window(), Some((&1, &"one")));
		assert_eq!(cache.peek_lru_main(), None);

		cache.get(&1);
		cache.get(&2);
		cache.get(&3);
		cache.get(&4);
		cache.get(&5);
		assert_eq!(cache.peek_lru_window(), Some((&1, &"one")));
		assert_eq!(cache.peek_lru_main(), None);

		cache.get(&3);
		cache.get(&2);
		cache.get(&4);
		cache.get(&1);
		cache.get(&5);
		assert_eq!(cache.peek_lru_window(), Some((&3, &"three")));
		assert_eq!(cache.peek_lru_main(), None);
	}

	#[test]
	fn check_if_cap_and_len_are_correct() {
		let mut cache = WTinyLfuCache::new(3, 10);
		cache.push(1, "one");
		cache.push(2, "two");
		assert_eq!(cache.cap(), 3);
		assert_eq!(cache.len(), 2);

		cache.get(&1);
		cache.get(&2);
		assert_eq!(cache.cap(), 3);
		assert_eq!(cache.len(), 2);

		cache.push(3, "three");
		assert_eq!(cache.cap(), 3);
		assert_eq!(cache.len(), 3);

		cache.get(&3);
		assert_eq!(cache.cap(), 3);
		assert_eq!(cache.len(), 3);

		assert_eq!(&iter_keys(&cache), &[1, 2, 3]);
	}

	#[test]
	fn clear_cache() {
		let mut cache = WTinyLfuCache::new(10, 10);
		cache.push(1, "one");
		cache.push(2, "two");
		assert_eq!(cache.get(&1), Some(&"one"));
		assert_eq!(cache.get(&2), Some(&"two"));
		assert_eq!(cache.len(), 2);
		assert_eq!(cache.cap(), 10);
		assert_eq!(&iter_keys(&cache), &[1, 2]);

		cache.clear();
		assert_eq!(cache.get(&1), None);
		assert_eq!(cache.get(&2), None);
		assert_eq!(cache.len(), 0);
		assert_eq!(cache.cap(), 10);
		assert_eq!(&iter_keys(&cache), &[]);
	}
}
