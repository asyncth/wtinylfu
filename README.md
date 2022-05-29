# An implementation of W-TinyLFU cache

Implements W-TinyLFU cache as proposed in "TinyLFU: A Highly Efficient
Cache Admission Policy" paper using only safe Rust. The API of this
crate is meant to be similar to the API of `lru` crate.

# Example usage

```rust
use wtinylfu::WTinyLfuCache;

fn main() {
    let mut cache = WTinyLfuCache::new(2, 10);
    cache.push(1, "one");
    cache.push(2, "two");
    assert_eq!(cache.get(&1), Some(&"one"));
    assert_eq!(cache.get(&2), Some(&"two"));
}
```

# Contributing

Contributions are welcome! Currently this project is hosted both on [
GitHub](https://github.com/asyncth/wtinylfu) and [sr.ht](
https://git.sr.ht/~asyncth/wtinylfu). Contributions from either of these
are accepted. Please follow [contributing guidelines](CONTRIBUTING.md).
