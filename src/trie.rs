use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Label(String),
    Wildcard,
}

#[derive(Clone, Debug)]
pub struct DnsTrie<Value> {
    root: Node<Value>,
}

#[derive(Clone, Debug)]
pub struct Node<Value> {
    children: HashMap<Key, Node<Value>>,
    value: Option<Value>,
}

impl<Value> Default for Node<Value> {
    fn default() -> Self {
        Self {
            children: HashMap::new(),
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
                children: HashMap::new(),
                value: None,
            },
        }
    }
}

impl<Value> DnsTrie<Value> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn leaf(value: Value) -> Self {
        Self {
            root: Node {
                children: HashMap::new(),
                value: Some(value),
            },
        }
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
