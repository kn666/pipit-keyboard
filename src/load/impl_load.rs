use std::collections::BTreeMap;
use std::io::prelude::*;
use std::fs::File;
use toml::Value;

use load::{KmapParser, FromToml, toml_to_vec, get_key};
use types::{Maps, SeqType, Options, ModeName, ModeInfo, WordInfo};
use types::errors::*;


impl Options {
    pub fn load(parsed_options: &Value) -> Result<Options>{
        let mut options = Options::from_toml(parsed_options)?;
        options.verify_requirements()?;
        options.set_auto();
            // .chain_err("failure to auto-generate options")
        Ok(options)
    }
}

impl Maps {
    pub fn load(toml_path: &str) -> Result<Maps> {
        Ok(Maps::load_helper(toml_path)
            .chain_err(|| format!("failure to load from settings: {}",
                                  toml_path))?)
    }

    fn load_helper(toml_path: &str) -> Result<Maps> {
        /// Load stuff into both Options and Maps
        // TODO don't clone things so much?
        let toml = parse_toml(toml_path)
            .chain_err(|| "failure to parse toml format")?;

        let other = get_section(&toml, "other")?;

        let options = Options::load(get_section(&toml, "options")?)
            .chain_err(|| "failure to load options")?;

        let mut maps = Maps::new();

        maps.load_modes(&toml)
            .chain_err(|| "failure to load modes")?;

        maps.load_chords(&options)
            .chain_err(|| "failure to load chords")?;
        maps.options = options;

        maps.load_plains(&toml)
            .chain_err(|| "failure to load plain keys")?;

        maps.load_macros(&toml)
            .chain_err(|| "failure to load macros")?;

        maps.load_plain_mods(&toml)
            .chain_err(|| "failure to load plain modifiers")?;

        maps.load_word_mods(other)
            .chain_err(|| "failure to load word modifiers")?;

        maps.load_anagram_mods(other)
            .chain_err(|| "failure to load anagram modifiers")?;

        maps.load_word_list(other) // TODO rename word_list?
            .chain_err(|| "failure to load dictionary")?;

        maps.load_commands(other)
            .chain_err(|| "failure to load commands")?;
        Ok(maps)
    }

    fn load_modes(&mut self, toml: &Value) -> Result<()>{
        let modes = toml_to_vec(get_section(toml, "mode")?, |x| Ok(x.clone()))?;
        for mode_table in &modes{
            let name = get_key(mode_table, "name")?;

            self.add_mode(
                ModeName::from_toml(name)?,
                ModeInfo::from_toml(mode_table)?
            )?;
        }
        Ok(())
    }

    fn load_chords(&mut self, options: &Options) -> Result<()>{
        let mut kmap_parser = KmapParser::new(options)?;
        for kmap in self.get_kmap_paths() {
            let chords = kmap_parser.parse(&kmap)
                .chain_err(||format!("failure to load kmap: '{}'", kmap))?;
            self.add_chords(&kmap, chords)?;
        }
        Ok(())
    }

    fn load_macros(&mut self, toml: &Value) -> Result<()>{
        let table = get_section(toml, "macros")?;
        self.set_sequences(SeqType::Macro, BTreeMap::from_toml(table)?)?;
        Ok(())
    }

    fn load_plains(&mut self, toml: &Value) -> Result<()>{
        let table = get_section(toml, "plain_keys")?;
        self.set_sequences(SeqType::Plain, BTreeMap::from_toml(table)?)?;
        Ok(())
    }

    fn load_plain_mods(&mut self, toml: &Value) -> Result<()>{
        let table = get_section(toml, "plain_modifiers")?;
        for (name, seq) in &BTreeMap::from_toml(table)? {
            self.add_plain_mod(name.to_owned(), seq)?;
        }
        Ok(())
    }

    fn load_word_mods(&mut self, other: &Value) -> Result<()>{
        self.word_mods = Vec::from_toml(get_section(other, "word_modifiers")?)?;
        Ok(())
    }

    fn load_anagram_mods(&mut self, other: &Value) -> Result<()>{
        let array = get_key(other, "anagram_modifiers")?;
        self.anagram_mods = Vec::from_toml(array)?;
        Ok(())
    }

    fn load_commands(&mut self, other: &Value) -> Result<()>{
        // TODO use set sequences, and then process after?
        let array = get_section(other, "commands")?;
        let command_list = Vec::from_toml(array)?;
        for name in &command_list {
            self.add_command(name)?;
        }
        Ok(())
    }

    fn load_word_list(&mut self, other: &Value) -> Result<()>{
        let array = get_section(other, "dictionary")?;
        let word_list = toml_to_vec(array, WordInfo::from_toml)?;
        for kmap in &self.get_kmaps_with_words(){
            for info in &word_list {
                self.add_word(info.to_owned(), kmap)?
            }
        }
        Ok(())
    }
}

fn parse_toml(toml_path: &str) -> Result<Value> {
    let mut f: File = File::open(toml_path)?;
    let mut buffer = String::new();
    f.read_to_string(&mut buffer)?;
    let toml = buffer.parse::<Value>()?;
    Ok(toml)
}

fn get_section<'a>(toml: &'a Value, section_name: &str) -> Result<&'a Value> {
    // By section I mean a table that's used as 1 section of the settings
    toml.get(section_name)
        .ok_or_else(
            || ErrorKind::MissingValue(
                "table".into(),
                Some(section_name.into())
            ).into()
        )
}