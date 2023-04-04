use core::fmt;
use std::collections::BTreeMap;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Key {
    Wildcard,
    Label(String),
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wildcard => write!(f, "*"),
            Self::Label(label) => write!(f, "{label}"),
        }
    }
}

impl fmt::Debug for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

#[derive(Clone)]
pub struct DnsTrie<Value> {
    root: Node<Value>,
}

impl<Value: fmt::Debug> fmt::Debug for DnsTrie<Value> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.root, f)
    }
}

#[derive(Clone)]
pub struct Node<Value> {
    children: BTreeMap<Key, Node<Value>>,
    value: Option<Value>,
}

fn pretty<Value: fmt::Debug>(
    node: &Node<Value>,
    indent: usize,
    f: &mut fmt::Formatter,
) -> fmt::Result {
    let spacer = "└──";

    if indent == 0 {
        write!(f, "\n.")?;
    }

    for (key, child) in node.children.iter() {
        write!(f, "\n{:indent$}{spacer} {key:?}", "")?;
        pretty(child, indent + 4, f)?;
    }

    if let Some(value) = &node.value {
        write!(f, "\n{:indent$}{spacer} {value:?}", "")?;
    }

    Ok(())
}

impl<Value: fmt::Debug> fmt::Debug for Node<Value> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        pretty(self, 0, f)
    }
}

impl<Value> Default for Node<Value> {
    fn default() -> Self {
        Self {
            children: BTreeMap::new(),
            value: None,
        }
    }
}

impl<Value> Node<Value> {
    pub fn insert(&mut self, keys: &[Key], val: Value) {
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

    pub fn lookup(&self, keys: &[Key]) -> Option<&Value> {
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

impl<Value> Default for DnsTrie<Value> {
    fn default() -> Self {
        Self {
            root: Node {
                children: BTreeMap::new(),
                value: None,
            },
        }
    }
}

impl<Value> DnsTrie<Value> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, keys: &[Key], val: Value) {
        self.root.insert(keys, val)
    }

    pub fn lookup(&self, keys: &[Key]) -> Option<&Value> {
        self.root.lookup(keys)
    }
}

#[cfg(test)]
#[allow(clippy::redundant_clone)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_normal() {
        let mut trie = DnsTrie::new();

        let foo = Key::Label("foo".to_string());
        let bar = Key::Label("bar".to_string());
        let key = &[foo, bar];

        trie.insert(key, 1);

        assert_eq!(trie.lookup(key), Some(&1));
    }

    #[test]
    fn test_lookup_wildcard() {
        let mut trie = DnsTrie::new();

        let foo = Key::Label("foo".to_string());
        let bar = Key::Label("bar".to_string());

        trie.insert(&[foo.clone()], 1);
        trie.insert(&[foo.clone(), Key::Wildcard], 2);

        assert_eq!(trie.lookup(&[foo.clone()]), Some(&1));
        assert_eq!(trie.lookup(&[foo.clone(), bar.clone()]), Some(&2));
    }

    #[test]
    fn test_lookup_none() {
        let mut trie = DnsTrie::new();

        let foo = Key::Label("foo".to_string());
        let bar = Key::Label("bar".to_string());
        let key = &[foo.clone(), bar.clone()];

        trie.insert(key, 1);

        assert_eq!(trie.lookup(&[foo.clone()]), None);
        assert_eq!(trie.lookup(key), Some(&1));
    }
}
