use core::fmt;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Key<K> {
    Wildcard,
    Exact(K),
}

impl<K: fmt::Display> fmt::Display for Key<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wildcard => write!(f, "*"),
            Self::Exact(key) => write!(f, "{key}"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Node<K, V> {
    children: BTreeMap<Key<K>, Node<K, V>>,
    value: Option<V>,
}

impl<K, V> Default for Node<K, V> {
    fn default() -> Self {
        Self {
            children: BTreeMap::new(),
            value: None,
        }
    }
}

fn pretty<K: fmt::Display, V: fmt::Display>(
    node: &Node<K, V>,
    indent: usize,
    f: &mut fmt::Formatter,
) -> fmt::Result {
    let spacer = "└──";

    if indent == 0 {
        write!(f, "\n.")?;
    }

    for (key, child) in node.children.iter() {
        write!(f, "\n{:indent$}{spacer} {key}", "")?;
        pretty(child, indent + 4, f)?;
    }

    if let Some(value) = &node.value {
        write!(f, "\n{:indent$}{spacer} {value}", "")?;
    }

    Ok(())
}

impl<K: fmt::Display, V: fmt::Display> fmt::Display for Node<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        pretty(self, 0, f)
    }
}

impl<K, V> Node<K, V> {
    pub fn insert(&mut self, keys: &[Key<K>], val: V)
    where
        K: Clone + Ord,
    {
        if let Some((head, tail)) = keys.split_first() {
            let node = self
                .children
                .entry(head.clone())
                .or_insert_with(Node::default);

            node.insert(tail, val);
        } else {
            self.value = Some(val);
        }
    }

    pub fn lookup(&self, keys: &[Key<K>]) -> Option<&V>
    where
        K: Clone + Ord,
    {
        if let Some((head, tail)) = keys.split_first() {
            if let Some(child) = self.children.get(head) {
                child.lookup(tail)
            } else if let Some(child) = self.children.get(&Key::Wildcard) {
                child.lookup(tail)
            } else {
                None
            }
        } else {
            self.value.as_ref()
        }
    }
}

#[derive(Clone, Debug)]
pub struct Trie<K, V> {
    root: Node<K, V>,
}

impl<K, V> Default for Trie<K, V> {
    fn default() -> Self {
        Self {
            root: Node::default(),
        }
    }
}

impl<K: fmt::Display, V: fmt::Display> fmt::Display for Trie<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.root, f)
    }
}

impl<K, V> Trie<K, V> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, keys: &[Key<K>], val: V)
    where
        K: Clone + Ord,
    {
        self.root.insert(keys, val)
    }

    pub fn lookup(&self, keys: &[Key<K>]) -> Option<&V>
    where
        K: Clone + Ord,
    {
        self.root.lookup(keys)
    }
}

#[cfg(test)]
#[allow(clippy::redundant_clone)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_normal() {
        let mut trie = Trie::new();

        let foo = Key::Exact("foo");
        let bar = Key::Exact("bar");
        let key = &[foo, bar];

        trie.insert(key, 1);

        assert_eq!(trie.lookup(key), Some(&1));
    }

    #[test]
    fn test_lookup_wildcard() {
        let mut trie = Trie::new();

        let foo = Key::Exact("foo");
        let bar = Key::Exact("bar");

        trie.insert(&[foo.clone()], 1);
        trie.insert(&[foo.clone(), Key::Wildcard], 2);

        assert_eq!(trie.lookup(&[foo.clone()]), Some(&1));
        assert_eq!(trie.lookup(&[foo.clone(), bar.clone()]), Some(&2));
    }

    #[test]
    fn test_lookup_none() {
        let mut trie = Trie::new();

        let foo = Key::Exact("foo");
        let bar = Key::Exact("bar");
        let key = &[foo.clone(), bar.clone()];

        trie.insert(key, 1);

        assert_eq!(trie.lookup(&[foo.clone()]), None);
        assert_eq!(trie.lookup(key), Some(&1));
    }
}
