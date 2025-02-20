use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

pub trait Ex: Clone + Any + Send + Sync + 'static {}
/// A type map for request extensions.
///
/// All entries into this map must be owned types (or static references).
#[derive(Debug, Default)]
pub struct Extensions {
    /// Use AHasher with a std HashMap with for faster lookups on the small `TypeId` keys.
    map: HashMap<TypeId, Box<dyn Any + Send + Sync + 'static>, BuildHasherDefault<NoOpHasher>>,
}

impl Extensions {
    /// 创建一个空的 Extensions
    pub fn new() -> Extensions {
        Extensions {
            map: HashMap::default(),
        }
    }
    /// 插入数据
    pub fn insert<T: Send + Sync + 'static>(&mut self, val: T) -> Option<T> {
        self.map
            .insert(TypeId::of::<T>(), Box::new(val))
            .and_then(downcast_owned)
    }
    /// 是否包含某个元素
    pub fn contains<T: Send + Sync + 'static>(&self) -> bool {
        self.map.contains_key(&TypeId::of::<T>())
    }
    ///获取元素
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|b| b.downcast_ref())
    }
    /// 获取可更改的元素
    pub fn get_mut<T: Send + Sync + 'static>(&mut self) -> Option<&mut T> {
        self.map
            .get_mut(&TypeId::of::<T>())
            .and_then(|b| b.downcast_mut())
    }
    ///移除元素
    pub fn remove<T: Send + Sync + 'static>(&mut self) -> Option<T> {
        self.map.remove(&TypeId::of::<T>()).and_then(downcast_owned)
    }
    /// 清空所有元素
    pub fn clear(&mut self) {
        self.map.clear()
    }
    /// 扩展
    pub fn extend(&mut self, other: Extensions) {
        self.map.extend(other.map);
    }
}
#[derive(Debug, Clone, Default)]
pub struct NoOpHasher(u64);

impl Hasher for NoOpHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, _bytes: &[u8]) {
        //不支持u8
        unimplemented!("This NoOpHasher can only handle u64s")
    }
    fn write_u64(&mut self, i: u64) {
        self.0 = i;
    }
}

fn downcast_owned<T: Send + Sync + 'static>(boxed: Box<dyn Any + Send + Sync>) -> Option<T> {
    boxed.downcast().ok().map(|boxed| *boxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove() {
        let mut map = Extensions::new();

        map.insert::<i8>(123);
        assert!(map.get::<i8>().is_some());

        map.remove::<i8>();
        assert!(map.get::<i8>().is_none());
    }
}
