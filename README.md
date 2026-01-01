> [!IMPORTANT]
> Given that this library is rarely updated and is quite barebones,
> consider using [Moka](https://crates.io/crates/moka) or
> [Mini Moka](https://crates.io/crates/mini-moka) for caching. Strictly
> speaking, the README claims that these libraries do not currently
> implement W-TinyLFU, but rather use TinyLFU alone, but it probably
> should be good enough anyway.

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

Contributions are welcome! Please follow
[contributing guidelines](CONTRIBUTING.md).
