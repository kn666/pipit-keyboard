use std::collections::{HashMap, HashSet};
use std::fmt::{self, Display};

use types::{AllData, AnagramNum, Chord, KmapPath, Name};

/// The Checker warns about sub-optimal configuration, like conflicting chords
/// or skipped anagram numbers. Any config issues that would break the firmware
/// should be caught in the loading or formatting code instead.
#[derive(Debug)]
struct Checker {
    // TODO use more references, instead of duplicating lots of stuff
    reverse_kmaps: HashMap<KmapPath, HashMap<Chord, AnagramSet>>,
    seq_names: HashSet<Name>,
    chord_names: HashSet<Name>,
    word_mod_names: HashSet<Name>,
}

#[derive(Debug)]
struct AnagramSet(HashMap<AnagramNum, Vec<Name>>);

////////////////////////////////////////////////////////////////////////////////

impl AllData {
    pub fn check(&self) {
        let checker = self.checker();
        checker.check_unused();
        checker.check_conflicts();
    }

    fn checker(&self) -> Checker {
        Checker {
            reverse_kmaps: self.reverse_chords(),
            seq_names: self.seq_names(),
            chord_names: self.chord_names(),
            word_mod_names: self.word_mods(),
        }
    }

    fn reverse_chords(&self) -> HashMap<KmapPath, HashMap<Chord, AnagramSet>> {
        let mut reversed = HashMap::new();
        for (kmap, chord_map) in &self.chords {
            for (name, chord) in chord_map {
                let mut base_chord = chord.to_owned();
                base_chord.anagram_num = AnagramNum(0);

                reversed
                    .entry(kmap.to_owned())
                    .or_insert_with(HashMap::new)
                    .entry(base_chord)
                    .or_insert_with(AnagramSet::new)
                    .insert(chord.anagram_num, name.clone());
            }
        }
        reversed
    }

    fn seq_names(&self) -> HashSet<Name> {
        self.sequences
            .iter()
            .flat_map(|(_, seq_type)| seq_type.keys())
            .cloned()
            .collect()
    }

    fn chord_names(&self) -> HashSet<Name> {
        self.chords
            .iter()
            .flat_map(|(_, kmap)| kmap.keys())
            .cloned()
            .collect()
    }

    fn word_mods(&self) -> HashSet<Name> {
        self.word_mods
            .iter()
            .chain(self.anagram_mods.iter())
            .cloned()
            .collect()
    }
}

impl Checker {
    /// If two names have the same chord, or there's a skipped anagram
    /// number, print out that whole set of names.
    pub fn check_conflicts(&self) {
        // TODO option to check for mode conflicts instead
        for (kmap, reversed) in &self.reverse_kmaps {
            let heading = format!(
                "\nConflicting chords (in parens) or skipped anagrams \
                 (\"{}\") in \'{}\':",
                AnagramSet::missing_symbol(),
                kmap
            );
            print_iter(
                &heading,
                reversed
                    .iter()
                    .filter(|&(_chord, set)| {
                        set.is_invalid(&self.word_mod_names)
                    })
                    .map(|(_chord, set)| set),
            );
        }
    }

    /// Compare the stored chords and sequences (and word mods), and print
    /// any names that don't appear in both.
    pub fn check_unused(&self) {
        let seqs_and_mods: HashSet<_> = self.seq_names
            .union(&self.word_mod_names)
            .cloned()
            .collect();
        print_iter(
            "Unused chords:",
            self.chord_names.difference(&seqs_and_mods),
        );
        print_iter(
            "Unused sequences:",
            seqs_and_mods.difference(&self.chord_names),
        );
    }
}

impl AnagramSet {
    fn new() -> AnagramSet {
        AnagramSet(HashMap::new())
    }

    fn insert(&mut self, anagram_num: AnagramNum, name: Name) {
        self.0
            .entry(anagram_num)
            .or_insert_with(Vec::new)
            .push(name);
    }

    fn is_invalid(&self, word_mod_names: &HashSet<Name>) -> bool {
        if self.has_chord_conflict(word_mod_names) {
            return true;
        }
        for num in self.max_anagram().up_to() {
            if !self.contains_anagram(num) {
                return true;
            }
        }
        false
    }

    fn has_chord_conflict(&self, word_mod_names: &HashSet<Name>) -> bool {
        for v in self.0.values() {
            match v.len() {
                1 => continue,
                2 => if !is_pair_legal(&v[0], &v[1], word_mod_names) {
                    return true;
                },
                _ => return true,
            }
        }
        false
    }

    fn contains_anagram(&self, anagram_num: AnagramNum) -> bool {
        self.0.contains_key(&anagram_num)
    }

    fn max_anagram(&self) -> AnagramNum {
        self.0
            .keys()
            .max()
            .expect("failed to get max anagram num")
            .to_owned()
    }

    fn anagram_to_string(&self, anagram_num: AnagramNum) -> String {
        match self.0.get(&anagram_num) {
            None => AnagramSet::missing_symbol(),
            Some(v) => {
                let strings: Vec<_> = v.iter().map(|x| x.to_string()).collect();
                if strings.len() == 1 {
                    strings[0].clone()
                } else {
                    format!("({})", strings.join(", "))
                }
            }
        }
    }

    /// String used when there is no mapping for a give anagram number.
    fn missing_symbol() -> String {
        "???".to_string()
    }
}

impl fmt::Display for AnagramSet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO use color?
        let mut strings = Vec::new();
        for num in self.max_anagram().up_to() {
            // TODO get iterator over all AnagramNums?
            strings.push(self.anagram_to_string(num));
        }
        let s = format!("anagrams: [{}]", strings.join(", "));
        fmt::Display::fmt(&s, f)
    }
}

/// If the iterator is non-empty, print the given heading and then print each
/// element of the iterator on a separate line with some spaces before it.
fn print_iter<T>(heading: &str, iter: T)
where
    T: Iterator,
    <T as Iterator>::Item: Display,
{
    let mut iter = iter.peekable();
    if !iter.peek().is_some() {
        return;
    }
    println!("{}", heading);
    for thing in iter {
        println!("  {}", thing);
    }
    println!("");
}

/// Return true if one is a word_mod or anagram_mod, and the other cannot be
/// used in a word chord (letters, mostly). This lets us map
/// word_mods/anagram_mods to chords that do something different when pressed
/// separately from a word (like mod_shift and mod_capital). It doesn't catch
/// all bad mappings, though! If a word_mod/anagram_mod shares even a single
/// switch with a multi-switch letter, that will mess up the word lookup
/// process. We don't check for that yet, because it's unlikely to happen
/// accidentally and it's annoying to implement.
// TODO make sure the above warning is easily findable
fn is_pair_legal(
    name1: &Name,
    name2: &Name,
    word_mod_names: &HashSet<Name>,
) -> bool {
    let okay = |a, b| {
        word_mod_names.contains(a) && !word_mod_names.contains(b)
        // TODO re-add some test like this one
        // && !WordBuilder::allowed_in_chord(b)
    };
    okay(name1, name2) || okay(name2, name1)
}
