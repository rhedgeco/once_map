# OnceMap
A small library for establishing a map of items that need to be initialized. Similar to [`OnceCell`](https://github.com/matklad/once_cell), data can be inserted only once. After that, it can only be retrieved as a read only reference. The difference is that data is retrieved by via key into a map, and different parts of the map may be initialized at different times and in different threads.

## Example
```rust
static GLOBAL_MAP: OnceMap<u8, String> = OnceCell::default();

fn main() {
    let string0 = GLOBAL_MAP.get_or_init(&0, || "Hello".into());
    let string1 = GLOBAL_MAP.get_or_init(&1, || ", ".into());
    let string2 = GLOBAL_MAP.get_or_init(&3, || "World!".into());

    println!("{string0}{string1}{string2}");
}
```

## [MIT License](./LICENSE.md)
