use std;
use std::collections::BTreeMap;
use std::collections::binary_heap::BinaryHeap;

use types::{CCode, KeyPress, ToC};
use types::errors::LookupErr;

#[derive(Debug, Clone)]
pub struct HuffmanTable(pub BTreeMap<CCode, HuffmanEntry>);

#[derive(Debug, Clone)]
pub struct HuffmanEntry {
    bits: Vec<bool>,
    is_mod: bool,
}

#[derive(Debug, Eq, PartialEq)]
enum HuffmanNode {
    Leaf {
        count: usize,
        key: (CCode, bool),
    },
    Branch {
        left: Box<HuffmanNode>,
        right: Box<HuffmanNode>,
    },
}

impl HuffmanTable {
    pub fn new(keys: Vec<KeyPress>) -> HuffmanTable {
        assert!(!keys.is_empty());
        let counts = count(keys);
        let tree = make_tree(counts).expect("failed to get huffman tree");
        let mut map = BTreeMap::new();
        make_codes(&tree, Vec::new(), &mut map);
        HuffmanTable(map)
    }

    pub fn bits(&self, key: &CCode) -> Result<Vec<bool>, LookupErr> {
        Ok(self.get(key)?.to_owned().bits)
    }

    fn get(&self, key: &CCode) -> Result<&HuffmanEntry, LookupErr> {
        Ok(self.0.get(key).ok_or_else(|| LookupErr {
            key: key.into(),
            container: "huffman code table".into(),
        })?)
    }

    pub fn as_c_bits(&self, key: &CCode) -> Result<Vec<CCode>, LookupErr> {
        Ok(self.bits(key)?.into_iter().map(|b| b.to_c()).collect())
    }

    pub fn num_bits(&self, key: &CCode) -> Result<usize, LookupErr> {
        Ok(self.get(key)?.bits.len())
    }

    pub fn is_mod(&self, key: &CCode) -> Result<bool, LookupErr> {
        Ok(self.get(key)?.is_mod)
    }
}

impl HuffmanNode {
    fn count(&self) -> usize {
        match self {
            HuffmanNode::Leaf { count, .. } => *count,
            HuffmanNode::Branch { left, right } => left.count() + right.count(),
        }
    }
}

impl Ord for HuffmanNode {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.count().cmp(&self.count())
    }
}

impl PartialOrd for HuffmanNode {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(&other))
    }
}

fn make_codes(
    node: &HuffmanNode,
    prefix: Vec<bool>,
    out: &mut BTreeMap<CCode, HuffmanEntry>,
) {
    match node {
        HuffmanNode::Branch {
            ref left,
            ref right,
        } => {
            let mut left_prefix = prefix.clone();
            left_prefix.push(false);
            make_codes(&left, left_prefix, out);

            let mut right_prefix = prefix;
            right_prefix.push(true);
            make_codes(&right, right_prefix, out);
        }
        HuffmanNode::Leaf { ref key, .. } => {
            out.insert(
                key.0.to_owned(),
                HuffmanEntry {
                    bits: prefix,
                    is_mod: key.1,
                },
            );
        }
    }
}

fn make_tree(counts: BTreeMap<CCode, (usize, bool)>) -> Option<HuffmanNode> {
    let mut queue = BinaryHeap::new();
    for (key, (count, is_mod)) in counts {
        queue.push(HuffmanNode::Leaf {
            key: (key, is_mod),
            count: count,
        });
    }
    while queue.len() >= 2 {
        let left = queue.pop().unwrap();
        let right = queue.pop().unwrap();
        queue.push(HuffmanNode::Branch {
            left: Box::new(left),
            right: Box::new(right),
        });
    }
    queue.pop()
}

fn count(keys: Vec<KeyPress>) -> BTreeMap<CCode, (usize, bool)> {
    let mut counts: BTreeMap<CCode, (usize, bool)> = BTreeMap::new();
    for key_press in keys {
        increment(&mut counts, key_press.key_or_blank(), false);
        if let Some(modifiers) = key_press.mods {
            for modifier in modifiers {
                increment(&mut counts, modifier, true);
            }
        }
    }
    counts
}

fn increment(
    map: &mut BTreeMap<CCode, (usize, bool)>,
    key: CCode,
    is_mod: bool,
) {
    let count = map.entry(key).or_insert((0, is_mod));
    (*count).0 += 1;
}
