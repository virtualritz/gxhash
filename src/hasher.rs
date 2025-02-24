use crate::gxhash::platform::*;
use crate::gxhash::*;

use core::hash::{BuildHasher, Hasher};
use core::mem::{size_of, MaybeUninit};
use core::slice;
use rand::RngCore;

/// A `Hasher` for hashing an arbitrary stream of bytes.
/// # Features
/// - The fastest [`Hasher`] of its class<sup>1</sup>, for all input sizes
/// - Highly collision resitant
/// - DOS resistance thanks to seed randomization when using [`GxHasher::default()`]
///
/// *<sup>1</sup>There might me faster alternatives, such as `fxhash` for very small input sizes, but that usually have low quality properties.*
#[derive(Clone, Debug)]
pub struct GxHasher {
    state: State,
}

impl GxHasher {
    #[inline]
    fn with_state(state: State) -> GxHasher {
        GxHasher { state }
    }
}

impl Default for GxHasher {
    /// Creates a new hasher with a empty seed.
    ///
    /// # Warning ⚠️
    ///
    /// Not using a seed may make your [`Hasher`] vulnerable to DOS attacks.
    /// It is recommended to use [`GxBuildHasher::default()`] for improved DOS resistance.
    ///
    /// # Example
    ///
    /// ```
    /// use std::hash::Hasher;
    /// use gxhash::GxHasher;
    ///
    /// let mut hasher = GxHasher::default();
    ///
    /// hasher.write(b"Hello");
    /// hasher.write_u32(123);
    /// hasher.write_u8(42);
    ///
    /// println!("Hash is {:x}!", hasher.finish());
    /// ```
    #[inline]
    fn default() -> GxHasher {
        GxHasher::with_state(unsafe { create_empty() })
    }
}

impl GxHasher {
    /// Creates a new hasher using the provided seed.
    ///
    /// # Warning ⚠️
    ///
    /// Hardcoding a seed may make your [`Hasher`] vulnerable to DOS attacks.
    /// It is recommended to use [`GxBuildHasher::default()`] for improved DOS resistance.
    ///
    /// # Example
    ///
    /// ```
    /// use std::hash::Hasher;
    /// use gxhash::GxHasher;
    ///
    /// let mut hasher = GxHasher::with_seed(1234);
    ///
    /// hasher.write(b"Hello");
    /// hasher.write_u32(123);
    /// hasher.write_u8(42);
    ///
    /// println!("Hash is {:x}!", hasher.finish());
    /// ```
    #[inline]
    pub fn with_seed(seed: i64) -> GxHasher {
        // Use gxhash64 to generate an initial state from a seed
        GxHasher::with_state(unsafe { create_seed(seed) })
    }

    /// Finish this hasher and return the hashed value as a 128 bit
    /// unsigned integer.
    #[inline]
    pub fn finish_u128(&self) -> u128 {
        debug_assert!(size_of::<State>() >= size_of::<u128>());

        unsafe {
            let p = &finalize(self.state) as *const State as *const u128;
            *p
        }
    }
}

impl Hasher for GxHasher {
    #[inline]
    fn finish(&self) -> u64 {
        unsafe {
            let p = &finalize(self.state) as *const State as *const u64;
            *p
        }
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        // Improvement: only compress at this stage and finalize in finish
        self.state = unsafe { compress_fast(compress_all(bytes), self.state) };
    }
}

/// A builder for building GxHasher with randomized seeds by default, for improved DOS resistance.
#[derive(Clone, Debug)]
pub struct GxBuildHasher(State);

impl Default for GxBuildHasher {
    #[inline]
    fn default() -> GxBuildHasher {
        let mut uninit: MaybeUninit<State> = MaybeUninit::uninit();
        let mut rng = rand::thread_rng();
        unsafe {
            let ptr = uninit.as_mut_ptr() as *mut u8;
            let slice = slice::from_raw_parts_mut(ptr, VECTOR_SIZE);
            rng.fill_bytes(slice);
            GxBuildHasher(uninit.assume_init())
        }
    }
}

impl BuildHasher for GxBuildHasher {
    type Hasher = GxHasher;
    #[inline]
    fn build_hasher(&self) -> GxHasher {
        GxHasher::with_state(self.0)
    }
}

/// A `HashMap` using a (DOS-resistant) [`GxBuildHasher`].
#[cfg(feature = "std")]
pub type HashMap<K, V> = std::collections::HashMap<K, V, GxBuildHasher>;

/// A convenience trait that can be used together with the type aliases defined
/// to get access to the `new()` and `with_capacity()` methods for the
/// [`HashMap`] type alias.
#[cfg(feature = "std")]
pub trait HashMapExt {
    /// Constructs a new HashMap.
    fn new() -> Self;
    /// Constructs a new HashMap with a given initial capacity.
    fn with_capacity(capacity: usize) -> Self;
}

#[cfg(feature = "std")]
impl<K, V, S> HashMapExt for std::collections::HashMap<K, V, S>
where
    S: BuildHasher + Default,
{
    fn new() -> Self {
        std::collections::HashMap::with_hasher(S::default())
    }

    fn with_capacity(capacity: usize) -> Self {
        std::collections::HashMap::with_capacity_and_hasher(capacity, S::default())
    }
}

/// A `HashSet` using a (DOS-resistant) [`GxBuildHasher`].
#[cfg(feature = "std")]
pub type HashSet<T> = std::collections::HashSet<T, GxBuildHasher>;

/// A convenience trait that can be used together with the type aliases defined
/// to get access to the `new()` and `with_capacity()` methods for the
/// [`HashSet`] type alias.
#[cfg(feature = "std")]
pub trait HashSetExt {
    /// Constructs a new HashMap.
    fn new() -> Self;
    /// Constructs a new HashMap with a given initial capacity.
    fn with_capacity(capacity: usize) -> Self;
}

#[cfg(feature = "std")]
impl<K, S> HashSetExt for std::collections::HashSet<K, S>
where
    S: BuildHasher + Default,
{
    fn new() -> Self {
        std::collections::HashSet::with_hasher(S::default())
    }

    fn with_capacity(capacity: usize) -> Self {
        std::collections::HashSet::with_capacity_and_hasher(capacity, S::default())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn contructors_work() {
        let mut map = HashMap::new();
        map.insert("foo", 42);

        let mut map = HashMap::with_capacity(3);
        map.insert("friday", 13);

        let mut set = HashSet::new();
        set.insert(42);

        let mut map = HashSet::with_capacity(3);
        map.insert(13);
    }

    #[test]
    fn hasher_produces_stable_hashes() {
        let mut hashset = HashSet::default();
        assert!(hashset.insert(1234));
        assert!(!hashset.insert(1234));
        assert!(hashset.insert(42));

        let mut hashset = HashSet::default();
        assert!(hashset.insert("hello"));
        assert!(hashset.insert("world"));
        assert!(!hashset.insert("hello"));
        assert!(hashset.insert("bye"));
    }

    // This is important for DOS resistance
    #[test]
    fn gxhashset_uses_default_gxhasherbuilder() {
        let hashset_1 = HashSet::<u32>::default();
        let hashset_2 = HashSet::<u32>::default();

        let mut hasher_1 = hashset_1.hasher().build_hasher();
        let mut hasher_2 = hashset_2.hasher().build_hasher();

        hasher_1.write_i32(42);
        let hash_1 = hasher_1.finish();

        hasher_2.write_i32(42);
        let hash_2 = hasher_2.finish();

        assert_ne!(hash_1, hash_2);
    }

    // This is important for DOS resistance
    #[test]
    fn default_gxhasherbuilder_is_randomly_seeded() {
        let buildhasher_1 = GxBuildHasher::default();
        let buildhasher_2 = GxBuildHasher::default();

        let mut hasher_1 = buildhasher_1.build_hasher();
        let mut hasher_2 = buildhasher_2.build_hasher();

        hasher_1.write_i32(42);
        let hash_1 = hasher_1.finish();

        hasher_2.write_i32(42);
        let hash_2 = hasher_2.finish();

        assert_ne!(hash_1, hash_2);
    }

    #[test]
    fn gxhasherbuilder_builds_same_hashers() {
        let buildhasher = GxBuildHasher::default();

        let mut hasher = buildhasher.build_hasher();

        hasher.write_i32(42);
        let hash = hasher.finish();

        let mut hasher = buildhasher.build_hasher();

        hasher.write_i32(42);
        assert_eq!(hash, hasher.finish());
    }
}
