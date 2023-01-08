use std::{
    hash::Hash,
    sync::{Arc, RwLock},
};

use hashbrown::{
    hash_map::{DefaultHashBuilder, Entry},
    HashMap,
};
use once_cell::sync::OnceCell;
pub enum GetOrInitData<T> {
    Init(T),
    Get(T),
}

impl<T> GetOrInitData<T> {
    pub fn into_data(self) -> T {
        match self {
            GetOrInitData::Get(data) => data,
            GetOrInitData::Init(data) => data,
        }
    }
}

/// A map of cells that can be written to only once.
pub struct OnceMap<K, V, S = DefaultHashBuilder> {
    cache: OnceCell<Arc<RwLock<HashMap<K, Arc<V>, S>>>>,
}

impl<K, V> Clone for OnceMap<K, V> {
    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
        }
    }
}

impl<K, V> Default for OnceMap<K, V>
where
    K: Eq + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> OnceMap<K, V>
where
    K: Eq + Hash,
{
    /// Creates a new empty OnceMap.
    pub const fn new() -> Self {
        Self {
            cache: OnceCell::new(),
        }
    }

    /// Gets a reference to the underlying value associated with `key`.
    ///
    /// Returns `None` if the entry is empty.
    pub fn get(&self, key: &K) -> Option<&V> {
        let cache = self.cache.get()?;
        let map = cache.read().unwrap();
        let item = map.get(key)?;
        Some(unsafe { &*Arc::<V>::as_ptr(item) })
    }

    /// Initializes data associated with `key` using `f`.
    ///
    /// Returns `false` if the data has already been initialized.
    pub fn init<F>(&self, key: K, f: F) -> bool
    where
        F: FnOnce() -> V,
    {
        // if the map isnt even created, return false
        let Some(cache) = self.cache.get() else {
            return false;
        };

        // check if the item exists. If it does, do not initialize and return false.
        let map = cache.read().unwrap();
        match map.get(&key) {
            Some(_) => return false,
            _ => (),
        }

        // drop the map so we can lock it as write after this
        drop(map);

        // since there is no item in the map, initialize it.
        let mut map = cache.write().unwrap();
        return match map.entry(key) {
            // if somehow the data has been cached between locks, return false.
            Entry::Occupied(_) => false,
            // otherwise proceed with initializing the data
            Entry::Vacant(e) => {
                e.insert(Arc::new(f()));
                true
            }
        };
    }

    /// Gets the contents of the entry associated with `key`,
    /// initializing it with `f` if the cell was empty.
    pub fn get_or_init<F>(&self, key: K, f: F) -> GetOrInitData<&V>
    where
        F: FnOnce() -> V,
    {
        // first initialize the map using the `once_cell` crate.
        // this allows us to have a const fn new()
        let cache = self.cache.get_or_init(|| Default::default());

        // First try only reading from the map. This will only lock if there is a write currently happening.
        // This means that accesses should be fairly fast, because the map will only be locked to read when there is already a write happening.
        // If the cache data is there, a reference to it will be returned.
        // The reference is created by extracting the pointer inside the Arc.
        // The validity of this is documented here: https://doc.rust-lang.org/std/sync/struct.Arc.html#method.as_ptr
        // > The docs say, "The pointer is valid for as long as there are strong counts in the Arc."
        // This is SAFE because the data in the Arc, once in the hashmap, will never be modified or destroyed, and will live as long as the OnceMap.
        let map = cache.read().unwrap();
        match map.get(&key) {
            Some(item) => return GetOrInitData::Get(unsafe { &*Arc::<V>::as_ptr(item) }),
            _ => (),
        }

        // drop the map so we can lock it as write after this
        drop(map);

        // If the data has not been uploaded before, we need get a write lock and create the new entry for the data.
        let mut map = cache.write().unwrap();
        return match map.entry(key) {
            // if somehow the data has been cached between locks, well great! We will return it now.
            Entry::Occupied(e) => GetOrInitData::Get(unsafe { &*Arc::<V>::as_ptr(e.into_mut()) }),
            Entry::Vacant(e) => {
                GetOrInitData::Init(unsafe { &*Arc::<V>::as_ptr(e.insert(Arc::new(f()))) })
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use crate::OnceMap;

    #[test]
    fn get_or_init() {
        // create map
        static STATIC_MAP: OnceMap<u8, String> = OnceMap::new();

        // initialize the map locations
        STATIC_MAP.get_or_init(0, || "Hello, ".into());
        STATIC_MAP.get_or_init(1, || "World!".into());

        // test the locations are valid
        assert!(STATIC_MAP.get(&0) == Some(&"Hello, ".into()));
        assert!(STATIC_MAP.get(&1) == Some(&"World!".into()));
    }

    #[test]
    fn get_or_init_threaded() {
        // create map
        static STATIC_MAP: OnceMap<u8, String> = OnceMap::new();

        // initialize the map location in thread
        let thread = thread::spawn(|| {
            STATIC_MAP.get_or_init(0, || "Hello, ".into());
        });

        // initialize map location normally
        STATIC_MAP.get_or_init(1, || "World!".into());

        // join the thread
        thread.join().unwrap();

        // test the locations are valid
        assert!(STATIC_MAP.get(&0) == Some(&"Hello, ".into()));
        assert!(STATIC_MAP.get(&1) == Some(&"World!".into()));
    }
}
