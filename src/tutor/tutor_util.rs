use std::fs::{self, File};
use std::path::PathBuf;
use std::sync::Mutex;
use std::collections::{BTreeMap, HashMap};

use itertools::Itertools;
use unicode_segmentation::UnicodeSegmentation;
use ron::de::from_reader;

use types::{Chord, ModeName, Name, Spelling, TutorData};
use failure::{Error, ResultExt};

lazy_static! {
    static ref STATE: Mutex<State> = Mutex::new(State::new());
}

#[derive(Debug, Clone)]
pub struct State {
    tutor_data: Option<TutorData>,
    learning_map: HashMap<String, LearnState>,
    initial_learn_state: usize,
    mode: ModeName,
}

#[derive(Debug, Clone)]
pub struct LearnState(pub usize);

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LessonConfig {
    pub slides: Vec<Slide>,
    #[serde(default)]
    pub popup: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Slide {
    #[serde(default)]
    pub instruction: String,
    pub line: SlideLine,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum SlideLine {
    Letters(String),
    Words {
        words: Vec<SlideWord>,
        #[serde(default = "return_true")]
        show_errors: bool,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SlideWord {
    pub names: Vec<Name>,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub length_override: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct SlideEntry {
    pub text: String,
    pub chord: Chord,
    pub length: usize,
}

#[derive(Debug, Clone)]
pub enum PrevCharStatus {
    Correct,
    Incorrect(Option<LabeledChord>),
}

#[derive(Clone, Debug)]
pub struct LabeledChord {
    pub chord: Chord,
    pub label: Label,
}

#[derive(Debug, Clone)]
pub struct Label(String);

////////////////////////////////////////////////////////////////////////////////

impl State {
    fn new() -> State {
        State {
            tutor_data: None,
            learning_map: HashMap::new(),
            initial_learn_state: 10,
            mode: ModeName::default(),
        }
    }

    pub fn set_tutor_data(data: TutorData) {
        let mut state = STATE.lock().unwrap();
        match state.tutor_data {
            Some(_) => panic!("tutor data can only be set once"),
            None => state.tutor_data = Some(data),
        }
    }

    pub fn chord(name: &Name) -> Option<Chord> {
        if name == &Name(String::new()) {
            // Used for skipping colors in the cheatsheet config
            return Some(Chord::default());
        }

        let state = STATE.lock().unwrap();
        if let Some(ref data) = state.tutor_data {
            data.chords.get(&state.mode)?.get(name).cloned()
        } else {
            panic!("tutor data was not set")
        }
    }

    fn name(spelling: &Spelling) -> Option<Name> {
        let state = STATE.lock().unwrap();
        if let Some(ref data) = state.tutor_data {
            data.spellings.get(spelling).map(|name| name.clone())
        } else {
            panic!("tutor data was not set")
        }
    }

    pub fn is_learned(name: &str) -> Option<bool> {
        STATE
            .lock()
            .unwrap()
            .learning_map
            .get(name)
            .map(|state| state.is_learned())
    }

    pub fn update_learn_state(name: String, was_correct: bool) {
        let mut state = STATE.lock().unwrap();
        let initial_learn_state = state.initial_learn_state;
        state
            .learning_map
            .entry(name)
            .or_insert_with(|| LearnState(initial_learn_state))
            .update(was_correct);
    }

    pub fn initial_learn_state() -> usize {
        STATE.lock().unwrap().initial_learn_state
    }

    pub fn set_initial_learn_state(initial: usize) {
        let mut state = STATE.lock().unwrap();
        for (_, learn_state) in state.learning_map.iter_mut() {
            learn_state.reset(initial);
        }
        state.initial_learn_state = initial;
    }

    pub fn mode_list() -> Vec<String> {
        let state = STATE.lock().unwrap();
        if let Some(ref data) = state.tutor_data {
            data.chords.keys().map(|mode| mode.0.clone()).collect()
        } else {
            panic!("tutor data was not set")
        }
    }

    pub fn set_mode(mode_str: &str) {
        let mode = ModeName::from(mode_str);
        let mut state = STATE.lock().unwrap();
        if let Some(ref data) = state.tutor_data {
            if !data.chords.contains_key(&mode) {
                panic!("tried to switch unknown ModeName: {}", mode);
            }
        } else {
            panic!("tutor data was not set")
        }

        state.mode = mode;

        // Reset the learning state for each letter, since the chords could be
        // totally different now.
        let initial = state.initial_learn_state;
        for (_, learn_state) in state.learning_map.iter_mut() {
            learn_state.reset(initial);
        }
    }
}

impl TutorData {
    pub fn chord(&self, name: &Name, mode: &ModeName) -> Option<Chord> {
        if name == &Name(String::new()) {
            // Used for skipping colors in the cheatsheet config
            return Some(Chord::default());
        }
        self.chords.get(mode)?.get(name).cloned()
    }
}

impl LearnState {
    pub fn update(&mut self, was_correct: bool) {
        if was_correct {
            if self.0 > 0 {
                self.0 -= 1;
            }
        } else {
            self.0 += 1;
        }
    }

    pub fn reset(&mut self, initial: usize) {
        self.0 = initial;
    }

    pub fn is_learned(&self) -> bool {
        self.0 == 0
    }
}

impl PrevCharStatus {
    pub fn backspace(&self) -> Option<LabeledChord> {
        match self {
            PrevCharStatus::Correct => None,
            PrevCharStatus::Incorrect(_) => LabeledChord::backspace(),
        }
    }

    pub fn error(&self) -> Option<LabeledChord> {
        match self {
            PrevCharStatus::Correct => None,
            PrevCharStatus::Incorrect(x) => x.clone(),
        }
    }

    pub fn is_correct(&self) -> bool {
        match self {
            PrevCharStatus::Correct => true,
            PrevCharStatus::Incorrect(_) => false,
        }
    }
}

impl SlideLine {
    pub fn show_errors(&self) -> bool {
        match self {
            SlideLine::Letters(_) => true,
            SlideLine::Words { show_errors, .. } => *show_errors,
        }
    }

    pub fn has_length_overrides(&self) -> bool {
        match self {
            SlideLine::Letters(_) => false,
            SlideLine::Words { words, .. } => {
                words.iter().any(|w| w.length_override.is_some())
            }
        }
    }

    pub fn to_entries(&self) -> Result<(Vec<SlideEntry>, String), Error> {
        Ok(match self {
            SlideLine::Letters(string) => (Vec::new(), string.to_owned()),
            SlideLine::Words { words, .. } => {
                let entries: Result<Vec<_>, _> = words
                    .iter()
                    .map(|word| SlideEntry::from_word(word))
                    .collect();
                let entries = entries?;
                let string =
                    entries.iter().map(|entry| entry.text.clone()).join("");
                (entries, string)
            }
        })
    }
}

impl SlideEntry {
    // fn from_letter(letter: String) -> Result<SlideEntry, Error> {
    //     Ok(SlideEntry {
    //         chord: char_to_chord(&letter).ok_or_else(|| BadValueErr {
    //             thing: "character".into(),
    //             value: letter.clone(),
    //         })?,
    //         length: letter.graphemes(true).count(),
    //         text: letter.clone(),
    //     })
    // }

    fn from_word(word: &SlideWord) -> Result<SlideEntry, Error> {
        let chords: Option<Vec<_>> =
            word.names.iter().map(|name| State::chord(name)).collect();

        let chord = match chords {
            None => bail!("failed to create chords for word"),
            Some(v) => v.into_iter().fold(Chord::default(), |acc, mut c| {
                c.intersect(&acc);
                c
            }),
        };

        Ok(SlideEntry {
            chord: chord,
            length: word.length_override
                .unwrap_or_else(|| word.text.graphemes(true).count()),
            text: word.text.clone(),
        })
    }

    pub fn len(&self) -> usize {
        self.length
    }

    pub fn to_labeled_chord(&self) -> LabeledChord {
        LabeledChord {
            label: self.label(),
            chord: self.chord.clone(),
        }
    }

    pub fn label(&self) -> Label {
        if self.text.graphemes(true).count() == 1 {
            Label::from_char(&self.text)
        } else {
            Label::default()
        }
    }
}

impl LabeledChord {
    pub fn from_letter(letter: &str) -> Option<LabeledChord> {
        Some(LabeledChord {
            chord: char_to_chord(&letter)?,
            label: Label::from_char(&letter),
        })
    }

    fn backspace() -> Option<LabeledChord> {
        Some(LabeledChord {
            chord: backspace_chord()?,
            label: "bak".into(),
        })
    }
}

pub fn char_to_chord(character: &str) -> Option<Chord> {
    let spelling = Spelling::new(character).ok()?;
    let name = State::name(&spelling)?;
    let mut chord = State::chord(&name)?;
    if spelling.is_uppercase() {
        chord.intersect(&State::chord(&Name("mod_shift".into()))?)
    }
    Some(chord)
}

impl Label {
    pub fn from_char(character: &str) -> Label {
        match character {
            "\n" => "ret".into(), // "⏎"
            "\t" => "tab".into(),
            " " => "spc".into(),
            c => c.into(),
        }
    }

    pub fn pad(&self, width: usize) -> String {
        let start = (width - self.len()) / 2. as usize;
        " ".repeat(start) + &self.0 + &" ".repeat(width - start)
    }

    fn len(&self) -> usize {
        self.0.graphemes(true).count()
    }
}

impl Default for Label {
    fn default() -> Label {
        Label(String::new())
    }
}

impl Into<Label> for String {
    fn into(self) -> Label {
        Label(self)
    }
}

impl<'a> Into<Label> for &'a str {
    fn into(self) -> Label {
        self.to_owned().into()
    }
}

pub fn offset(width1: usize, width2: usize) -> usize {
    ((width2 - width1) as f32 / 2.).round() as usize
}

fn backspace_chord() -> Option<Chord> {
    State::chord(&"key_backspace".into())
}

pub fn load_lessons(
    lesson_dir: &str,
) -> Result<BTreeMap<String, LessonConfig>, Error> {
    let entries = fs::read_dir(lesson_dir)?;
    let mut map = BTreeMap::new();
    for entry in entries {
        let path = entry?.path();
        let name = lesson_path_to_name(&path);
        map.insert(name, read_lesson_file(&path)?);
    }
    Ok(map)
}

fn lesson_path_to_name(path: &PathBuf) -> String {
    let s = path.file_stem()
        .expect("invalid lesson file name")
        .to_str()
        .expect("lesson path is not valid unicode");
    let mut sections = s.split("_");
    let number = sections.next().expect("invalid lesson file name");
    let words: Vec<_> = sections.collect();
    format!("{}) {}", number, words.join(" "))
}

pub fn read_lesson_file(path: &PathBuf) -> Result<LessonConfig, Error> {
    let file = open_file(path)
        .context(format!("failed to open file: {}", path.display()))?;
    let lesson: LessonConfig = from_reader(file)
        .context(format!("failed to read RON file: {}", path.display()))?;
    Ok(lesson)
}

fn open_file(path: &PathBuf) -> Result<File, Error> {
    Ok(File::open(path)?)
}

fn return_true() -> bool {
    true
}

pub fn grapheme_slice<'a>(
    s: &'a str,
    start: usize,
    end: usize,
) -> Box<Iterator<Item = &'a str> + 'a> {
    Box::new(s.graphemes(true).skip(start).take(end))
}
