use std::str::FromStr;
use unicode_segmentation::UnicodeSegmentation;

use types::{AllData, Chord, KeyPress, KmapPath, Name, Sequence, Spelling,
            Validate};
use types::errors::*;
use failure::{Error, ResultExt};

#[derive(Debug)]
pub struct Word {
    pub name: Name,
    pub seq: Sequence,
    pub chord: Chord,
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy, Eq, PartialEq, Ord,
         PartialOrd, Hash)]
#[serde(deny_unknown_fields)]
pub struct AnagramNum(pub u8);

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct WordConfig {
    pub word: String,
    pub anagram: Option<AnagramNum>,
    pub chord: Option<String>,
}

pub struct WordBuilder<'a> {
    pub info: WordConfig,
    pub kmap: &'a KmapPath,
    pub all_data: &'a AllData,
}

////////////////////////////////////////////////////////////////////////////////

impl AnagramNum {
    pub fn max_allowable() -> u8 {
        // This depends on the representation in the lookup table
        15
    }

    /// Return an iterator over all the anagram numbers from zero to self,
    /// in order.
    pub fn up_to(&self) -> Box<Iterator<Item = AnagramNum>> {
        Box::new((0..=self.0).map(|i| AnagramNum(i)))
    }
}

impl Default for AnagramNum {
    fn default() -> AnagramNum {
        AnagramNum(0)
    }
}

impl Validate for AnagramNum {
    fn validate(&self) -> Result<(), Error> {
        // TODO move max out somewere?
        // TODO how to keep this matched with make_prefix_byte()?
        // const MAX_ANAGRAM: u8 = 7;
        let max = AnagramNum::max_allowable();
        if self.0 > max {
            Err(OutOfRangeErr {
                name: "anagram number".into(),
                value: self.0 as usize,
                min: 0,
                max: max as usize,
            })?
        }
        Ok(())
    }
}

impl From<AnagramNum> for usize {
    fn from(num: AnagramNum) -> usize {
        num.0 as usize
    }
}

// impl fmt::Display for AnagramNum {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "{}", self.0)
//     }
// }

impl WordConfig {
    fn seq_spelling(&self) -> String {
        self.word.clone()
    }

    fn chord_spelling(&self) -> String {
        self.chord.clone().unwrap_or(self.word.clone())
    }

    fn anagram_num(&self) -> AnagramNum {
        self.anagram.unwrap_or(AnagramNum(0))
    }

    fn has_alternate_chord(&self) -> bool {
        self.chord.is_some()
    }
}

impl Validate for WordConfig {
    fn validate(&self) -> Result<(), Error> {
        self.anagram.validate()
        // TODO check for illegal characters?
        // self.word
        // self.chord
    }
}

impl<'a> WordBuilder<'a> {
    pub fn finalize(&self) -> Result<Word, Error> {
        let seq_spelling = self.info.seq_spelling();
        let error_message = |thing| {
            format!("Failed to make {} for: '{}'", thing, &seq_spelling)
        };

        Ok(Word {
            name: self.make_name(),
            chord: self.make_chord().with_context(|_| error_message("chord"))?,
            seq: self.make_sequence()
                .with_context(|_| error_message("sequence"))?,
        })
    }

    fn make_name(&self) -> Name {
        // Ensure that each word has a unique name.

        let mut name = format!("word_{}", self.info.seq_spelling());
        if self.info.has_alternate_chord() {
            name += &format!("_{}", self.info.chord_spelling());
        }
        name += &format!("_{}", self.info.anagram_num().0);
        Name(name)
    }

    fn make_sequence(&self) -> Result<Sequence, Error> {
        let mut seq = Sequence::new();
        for letter in self.info.seq_spelling().graphemes(true) {
            let key_press = KeyPress::from_str(&letter.to_string())?;
            seq.push(key_press);
        }
        Ok(seq)
    }

    fn make_chord(&self) -> Result<Chord, Error> {
        let mut chord = Chord::new();

        let num = self.info.anagram_num();
        num.validate()?;
        chord.anagram_num = num;

        for letter in self.info.chord_spelling().chars() {
            let name = self.all_data
                .name_from_spelling(&Spelling(letter).to_lowercase())?;

            let new_chord: Chord = self.all_data
                .get_chord(&name, self.kmap)
                .ok_or_else(|| LookupErr {
                    key: name.to_string(),
                    container: "chords".into(),
                })?;
            chord.intersect(&new_chord);
        }
        Ok(chord)
    }
}
