use std::collections::{BTreeMap};
use std::clone::Clone;

use types::{Chord, Sequence, KeyPress, WordBuilder, Word, Options, SeqType, KmapPath,
            KmapInfo, Name, ModeName, CCode, ToC};


// TODO typealias kmap names
#[derive(Debug)]
pub struct Maps {
    pub chords:       BTreeMap<KmapPath, BTreeMap<Name, Chord>>,
    pub sequences:    BTreeMap<SeqType, BTreeMap<Name, Sequence>>,
    pub wordmods:     Vec<Name>,
    pub modifierkeys: Vec<Name>,
    pub anagrams:     Vec<Name>,
    pub modes:        BTreeMap<ModeName, Vec<KmapInfo>>,
    pub kmap_ids:     BTreeMap<KmapPath, String>,
    pub options:      Options,
    pub anagram_records: BTreeMap<KmapPath, Vec<Word>>,
}

impl Maps {
    pub fn new() -> Maps {
        Maps {
            chords: BTreeMap::new(),
            sequences: BTreeMap::new(),
            wordmods: Vec::new(),
            modifierkeys: Vec::new(),
            anagrams: Vec::new(),
            modes: BTreeMap::new(),
            kmap_ids: BTreeMap::new(),
            options: Options::new(),
            anagram_records: BTreeMap::new(),
        }
    }

    pub fn add_command(&mut self, entry: &Name) {
        if self.get_sequences(&SeqType::Command).contains_key(entry){
            panic!(format!("commands map already contains key: {}", entry));
        }
        // commands are a single byte code, not an actual key sequence.
        // But we'll store it as a KeyPress for convenience.
        let mut fake_seq_with_command_code = Sequence::new();
        fake_seq_with_command_code.push(KeyPress::new_fake(&entry.to_uppercase()));

        self.get_sequences_mut(SeqType::Command)
            .insert(entry.to_owned(), fake_seq_with_command_code);
    }


    pub fn add_word(&mut self, seq_spelling: &str, chord_spelling: &str,
                    anagram: u64, kmap: &KmapPath)
    {
        // TODO build word in loader code instead?
        let word = WordBuilder {
            seq_spelling: seq_spelling,
            chord_spelling: chord_spelling,
            anagram: anagram,
            kmap: kmap,
            maps: &self,
        }.finalize();
        self.get_sequences_mut(SeqType::Word).insert(word.name.clone(),  word.seq.clone());
        self.get_chords_mut(&kmap).insert(word.name.clone(), word.chord.clone());
        self.add_anagram_record(kmap, word);
    }

    pub fn add_modifierkey(&mut self, name: Name, seq: &Sequence) {
        self.modifierkeys.push(name.clone());
        self.get_sequences_mut(SeqType::Plain).insert(name, seq.clone());
    }

    pub fn set_sequences(&mut self, seq_type: &SeqType, val: BTreeMap<Name, Sequence>) {
        assert!(!self.sequences.contains_key(&seq_type));
        self.sequences.insert(seq_type.clone(), val);
    }

    pub fn get_sequences(&self, seq_type: &SeqType) -> &BTreeMap<Name, Sequence>{
        self.sequences.get(&seq_type)
            .expect(&format!("Sequences has not been initialized for SeqType: {:?}", seq_type))
    }

    pub fn get_sequences_mut(&mut self, seq_type: SeqType) -> &mut BTreeMap<Name, Sequence>{
        self.sequences.get_mut(&seq_type)
            .expect(&format!("Sequences has not been initialized for SeqType: {:?}", seq_type))
    }

    fn get_sequence(&self, name: &Name) -> &Sequence {
        for seq_type in self.sequences.keys() {
            match self.get_sequences(seq_type).get(name){
                Some(s) => return s,
                None => (),
            }
        }
        panic!("No sequence found for name");
    }

    pub fn get_mod_key(&self, name: &Name) -> CCode {
        let seq = self.get_sequence(name);
        if seq.len() != 1 || seq.0[0].key != "0".to_c() {
            panic!("bad modifierkey sequence");
        }
        seq.0[0].modifier.clone()
    }

    // TODO handle missing mods better - here or firmware?
    pub fn get_chord_in_mode(&self, chord_name: &Name, mode: &ModeName) -> Chord {
        for kmap_info in self.modes.get(mode).expect("unknown mode"){
            if let Some(chord) = self.get_chord(chord_name, &kmap_info.path) {
                return chord;
            }
        }
        Chord::new()
    }


    pub fn get_anagrams(&self, mode: &ModeName) -> Vec<Chord> {
        let mut out = Vec::new();
        for name in self.anagrams.iter() {
            out.push(self.get_chord_in_mode(&name, mode));
        }
        out
    }

    pub fn get_mod_names(&self) -> Vec<Name> {
        let mut names = Vec::new();
        names.extend(self.modifierkeys.clone());
        names.extend(self.wordmods.clone());
        names
    }

    pub fn get_mod_chords(&self, mode: &ModeName) -> Vec<Chord> {
        let mut chords = Vec::new();
        for name in self.get_mod_names() {
            chords.push(self.get_chord_in_mode(&name, &mode));
        }
        chords
    }
    pub fn get_chord(&self, chord_name: &Name, kmap: &KmapPath) -> Option<Chord>{
        match self.chords[kmap].get(chord_name) {
            Some(chord) => Some(chord.clone()),
            None => None,
        }
    }

    pub fn get_chords(&self, kmap: &KmapPath) -> &BTreeMap<Name, Chord>{
        self.chords.get(kmap)
            .expect(&format!("Failed to get chords for kmap: {}", kmap))
    }

    pub fn get_chords_mut(&mut self, kmap: &KmapPath) -> &mut BTreeMap<Name, Chord>{
        self.chords.get_mut(kmap)
            .expect(&format!("Failed to get chords for kmap: {}", kmap))
    }

    pub fn get_anagram_chord(&self, num: u64, kmap: &KmapPath) -> Chord {
        let anagram_name = self.anagrams.iter()
            .nth(num as usize)
            .expect("invalid anagram number");
        self.get_chord(anagram_name, kmap)
            .expect("invalid anagram name")
    }

    pub fn get_kmap_paths(&self) -> Vec<KmapPath> {
        let v: Vec<_> = self.kmap_ids.keys()
            .map(|s| s.clone())
            .collect();
        v
    }

    pub fn get_kmaps_with_words(&self) -> Vec<KmapPath> {
        let mut out = Vec::new();
        for mode in self.modes.values() {
            for kmap_info in mode.iter() {
                if kmap_info.use_words {
                    out.push(kmap_info.path.clone());
                }
            }
        }
        out
    }

    pub fn get_kmaps_for_mode(&self, mode: &ModeName) -> Vec<KmapPath> {
        let v: Vec<_> = self.modes.get(mode)
            .expect("mode not found")
            .iter()
            .map(|x| x.path.to_owned())
            .collect();
        v
    }

    pub fn get_seq_types(&self) -> Vec<SeqType> {
        let v: Vec<_> = self.sequences.keys()
            .map(|s| s.to_owned())
            .collect();
        v
    }

    pub fn add_mode(&mut self, mode: ModeName, kmaps: Vec<KmapInfo>) {
        assert!(!self.modes.contains_key(&mode));
        // Store the kmaps that are included in this mode
        for kmap_info in kmaps.iter() {
            let kmap_path = &kmap_info.path;
            if self.kmap_ids.contains_key(kmap_path){
                continue;
            }
            // Initialize new chord map
            self.chords.insert(kmap_path.clone(), BTreeMap::new());
            // Generate nickname, for later formatting
            let nickname = format!("kmap{}", self.kmap_ids.len());
            self.kmap_ids.insert(kmap_path.clone(), nickname);
        }
        self.modes.insert(mode, kmaps);
    }

    pub fn add_chords(&mut self, kmap: &KmapPath, mut new_chords: BTreeMap<Name, Chord>) {
        self.chords
            .get_mut(kmap)
            .expect(&format!("Failed to add chord because kmap is unknown: {}",
                             kmap))
            .append(&mut new_chords);
    }


    pub fn add_anagram_record(&mut self, kmap: &KmapPath, word: Word) {
        self.anagram_records
            .entry(kmap.to_owned())
            .or_insert(Vec::new())
            .push(word)
    }

}
