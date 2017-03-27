use std::collections::HashMap;

use options::*;
use words::*;
use key_types::*;

#[derive(Debug)]
pub struct Maps{
    pub chords: HashMap<String, Chord>,
    pub plains: HashMap<String, Sequence>,
    pub macros: HashMap<String, Sequence>,
    pub words: HashMap<String, Sequence>,
    pub specials: HashMap<String, Sequence>,
    len_chord: usize,
}

impl Maps{
    pub fn new(num_bytes_in_chord: usize) -> Maps{
        Maps{
            chords: HashMap::new(),
            plains: HashMap::new(),
            macros: HashMap::new(),
            words: HashMap::new(),
            specials: HashMap::new(),
            len_chord: 8*num_bytes_in_chord,
        }
    }

    pub fn add_word(&mut self, entry: &Vec<String>) {
        let name = get_word_name(entry);
        let seq_spelling = entry.first().unwrap();
        let chord_spelling = entry.last().unwrap();

        self.words.insert(name.clone(),
                          make_word_sequence(seq_spelling));

        let word_chord = make_word_chord(chord_spelling, &self.chords, self.len_chord);
        self.chords.insert(name.clone(), word_chord);
        // println!("{:?}", name);
    }

    // pub fn add_chord(&mut self, entry: &Vec<String>) {
}


