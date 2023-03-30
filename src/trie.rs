use std::collections::HashMap;

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Key {
    Label(String),
    Wildcard,
}

pub struct DnsTrie<Value> {
    root: Node<Value>,
}

pub enum Node<Value> {
    Leaf(Value),
    Branch {
        children: HashMap<Key, Node<Value>>,
        value: Option<Value>,
    },
}

impl<Value> Default for DnsTrie<Value> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Value> DnsTrie<Value> {
    pub fn new() -> Self {
        Self {
            root: Node::Branch {
                children: HashMap::new(),
                value: None,
            },
        }
    }

    pub fn insert(&mut self, keys: &[Key], val: Value) {
        let mut node = &mut self.root;

        for key in keys {
            match node {
                Node::Leaf(_) => panic!("Tried to insert into a leaf node"),
                Node::Branch { children, value: _ } => {
                    node = children.entry(key.clone()).or_insert_with(|| Node::Branch {
                        children: HashMap::new(),
                        value: None,
                    });
                }
            }
        }

        match node {
            Node::Leaf(_) => panic!("Tried to insert into a leaf node"),
            Node::Branch { children: _, value } => {
                *value = Some(val);
            }
        }
    }

    pub fn lookup(&self, keys: &[Key]) -> Option<&Value> {
        let mut node = &self.root;

        for key in keys {
            match node {
                Node::Leaf(value) => return Some(value),
                Node::Branch { children, value } => {
                    if let Some(child) = children.get(key) {
                        node = child;
                    } else if let Some(child) = children.get(&Key::Wildcard) {
                        node = child;
                    } else {
                        return value.as_ref();
                    }
                }
            }
        }

        match node {
            Node::Leaf(value) => Some(value),
            Node::Branch { children: _, value } => value.as_ref(),
        }
    }
}

#[allow(clippy::redundant_clone)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup() {
        let mut trie = DnsTrie::new();

        let foo = Key::Label("foo".to_string());
        let bar = Key::Label("bar".to_string());

        trie.insert(&[foo.clone()], 1);
        trie.insert(&[foo.clone(), Key::Wildcard], 2);

        assert_eq!(trie.lookup(&[foo.clone()]), Some(&1));
        assert_eq!(trie.lookup(&[foo.clone(), bar.clone()]), Some(&2));
    }
}