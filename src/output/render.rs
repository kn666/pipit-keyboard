use bit_vec::BitVec;
use std::collections::BTreeMap;
use time::*;

use error::Error;
use types::{
    AllData, CCode, CEnumVariant, CTree, Chord, ChordSpec, Command, Field,
    HuffmanTable, KeyDefs, KeyPress, KmapPath, ModeName, Modifier, Name,
    SeqType, Sequence, ToC,
};
use util::usize_to_u8;

use output::{KmapBuilder, ModeBuilder};

c_struct!(struct HuffmanChar {
    bits: CCode,
    num_bits: usize,
    key_code: CCode,
    is_mod: bool,
});

impl AllData {
    /// Generate and save the c code containing the keyboard firmware
    /// configuration. `file_name_base` should have no extension. `.h` and
    /// `.cpp` will be added to it as needed.
    pub fn save_as(&self, file_name_base: &str) -> Result<(), Error> {
        self.save_helper(file_name_base, true)
    }

    /// Used for testing. The message contains a timestamp that would make the
    /// same output files look different if they're only generated at different
    /// times.
    #[allow(dead_code)]
    pub fn save_without_message_as(
        &self,
        file_name_base: &str,
    ) -> Result<(), Error> {
        self.save_helper(file_name_base, false)
    }

    fn save_helper(
        &self,
        file_name_base: &str,
        with_message: bool,
    ) -> Result<(), Error> {
        let main_files =
            self.render_main(with_message)?.format(file_name_base)?;

        let early_name_base = format!("{}_early", file_name_base);
        let early_files = self
            .render_early_config(with_message)?
            .format(&early_name_base)?;

        let mut file_names =
            main_files.save(&self.output_directory, file_name_base)?;

        file_names.extend(
            early_files.save(&self.output_directory, &early_name_base)?,
        );

        let file_name_list = file_names
            .into_iter()
            .map(|path| format!("{:?}", path))
            .collect::<Vec<_>>()
            .join(", ");

        println!("Saved keyboard configuration to: {}", file_name_list);
        Ok(())
    }

    /// Render c code defining any constants etc. that need to be included
    /// before / separately from the main auto_config.h file.
    fn render_early_config(&self, with_message: bool) -> Result<CTree, Error> {
        let mut group = Vec::new();
        if with_message {
            group.push(CTree::LiteralH(autogen_message()));
        }
        group.push(CTree::LiteralH("#pragma once\n".to_c()));
        group.extend(self.early_options.clone());
        Ok(CTree::Group(group))
    }

    fn render_main(&self, with_message: bool) -> Result<CTree, Error> {
        Ok(CTree::Group(vec![
            intro(with_message)?,
            CTree::Namespace {
                name: "conf".to_c(),
                contents: Box::new(CTree::Group(vec![
                    self.render_options(),
                    self.huffman_table.render()?,
                    self.render_modifiers()?,
                    Command::render_c_enum(self.commands.iter()),
                    SeqType::render_c_enum(self.sequences.seq_types()),
                    ModeName::render_c_enum(self.modes.keys()),
                    self.render_modes()?,
                ])),
            },
            make_debug_macros(),
        ]))
    }

    fn render_modes(&self) -> Result<CTree, Error> {
        let mut g = Vec::new();

        g.push(CTree::ConstVar {
            name: "MAX_KEYS_IN_SEQUENCE".to_c(),
            value: usize_to_u8(self.sequences.max_seq_length())?.to_c(),
            c_type: "uint8_t".to_c(),
            is_extern: true,
        });

        let (tree, kmap_struct_names) = self.render_kmaps()?;
        g.push(tree);

        let mut mode_struct_names = Vec::new();
        for (mode, info) in &self.modes {
            let m = ModeBuilder {
                mode_name: mode,
                info,
                kmap_struct_names: &kmap_struct_names,
                mod_chords: self.modifier_chords(mode),
                anagram_mask: self.get_anagram_mask(mode),
                chord_spec: self.chord_spec.clone(),
            };
            let (tree, name) = m.render()?;
            g.push(tree);
            mode_struct_names.push(name);
        }

        g.push(CTree::Array {
            name: "mode_structs".to_c(),
            values: CCode::map_prepend("&", &mode_struct_names),
            c_type: "ModeStruct*".to_c(),
            is_extern: true,
        });
        Ok(CTree::Group(g))
    }

    fn render_kmaps(
        &self,
    ) -> Result<(CTree, BTreeMap<KmapPath, CCode>), Error> {
        // Render all keymap structs as CTrees, and return their names
        let mut kmap_struct_names = BTreeMap::new();
        let mut g = Vec::new();
        for (i, (kmap_name, chords)) in self.chords.iter().enumerate() {
            let builder = KmapBuilder {
                kmap_nickname: format!("kmap{}", i),
                chord_map: chords,
                seq_maps: &self.sequences,
                huffman_table: &self.huffman_table,
                chord_spec: self.chord_spec.clone(),
            };
            let (tree, kmap_struct_name) = builder.render()?;
            g.push(tree);
            kmap_struct_names.insert(kmap_name.to_owned(), kmap_struct_name);
        }
        Ok((CTree::Group(g), kmap_struct_names))
    }

    fn render_options(&self) -> CTree {
        CTree::Group(self.options.clone())
    }

    fn render_modifiers(&self) -> Result<CTree, Error> {
        fn to_variants(mod_names: &[Name]) -> Vec<CCode> {
            mod_names
                .iter()
                .map(|name| Modifier::new(name).qualified_enum_variant())
                .collect()
        }

        let mut group = Vec::new();

        let all_mods: Vec<_> = self
            .modifier_names()
            .into_iter()
            .map(|name| Modifier::new(name))
            .collect();

        group.push(Modifier::render_c_enum(all_mods.iter()));

        group.push(CTree::Define {
            name: "NUM_MODIFIERS".to_c(),
            value: all_mods.len().to_c(),
        });

        group.push(CTree::ConstVar {
            name: "MAX_ANAGRAM_NUM".to_c(),
            value: self.chords.max_anagram_num().to_c(),
            c_type: "uint8_t".to_c(),
            is_extern: true,
        });

        group.push(CTree::Array {
            name: "word_mods".to_c(),
            values: to_variants(&self.word_mods),
            c_type: Modifier::enum_type(),
            is_extern: true,
        });

        group.push(CTree::Array {
            name: "plain_mods".to_c(),
            values: to_variants(&self.plain_mods),
            c_type: Modifier::enum_type(),
            is_extern: true,
        });

        group.push(CTree::Array {
            name: "anagram_mods".to_c(),
            values: to_variants(&self.anagram_mods),
            c_type: Modifier::enum_type(),
            is_extern: true,
        });

        group.push(CTree::Array {
            name: "anagram_mod_numbers".to_c(),
            values: self
                .get_anagram_mod_numbers()?
                .iter()
                .map(|num| num.to_c())
                .collect(),
            c_type: "uint8_t".to_c(),
            is_extern: true,
        });

        group.push(CTree::Array {
            name: "plain_mod_keys".to_c(),
            values: self.get_plain_mod_codes()?,
            c_type: "uint8_t".to_c(),
            is_extern: true,
        });

        Ok(CTree::Group(group))
    }

    fn get_plain_mod_codes(&self) -> Result<Vec<CCode>, Error> {
        // TODO this should be easier...
        self.plain_mods
            .iter()
            .map(|name| {
                Ok(self
                    .sequences
                    .get_seq_of_any_type(name)?
                    .lone_keypress()?
                    .format_mods())
            }).collect()
    }
}

impl ChordSpec {
    /// Convert the chord into CCode strings containing the byte representation
    /// used in the firmware.
    fn to_c_bytes(&self, chord: &Chord) -> Result<Vec<CCode>, Error> {
        let ordered_bools = self.to_firmware_order.permute(chord.switches())?;
        Ok(ordered_bools.blocks().map(|x| x.to_c()).collect())
    }

    fn to_c_initializer(&self, chord: &Chord) -> Result<CCode, Error> {
        Ok(format!("{{{}}}", self.to_c_bytes(chord)?.join(", ")).to_c())
    }

    pub fn to_c_constructor(&self, chord: &Chord) -> Result<CCode, Error> {
        Ok(format!(
            "{}({})",
            Self::c_type_name(),
            self.to_c_initializer(chord)?
        ).to_c())
    }

    pub fn c_type_name() -> CCode {
        "ChordData".to_c()
    }
}

impl HuffmanTable {
    fn render(&self) -> Result<CTree, Error> {
        let mut group = Vec::new();

        group.push(CTree::ConstVar {
            name: "MIN_HUFFMAN_CODE_BIT_LEN".to_c(),
            value: self.min_bit_length().to_c(),
            c_type: "uint8_t".to_c(),
            is_extern: true,
        });

        let mut initializers = Vec::new();
        for key in self.0.keys() {
            // TODO Don't repeatedly look up key
            let entry = self.get(key)?;
            let init = HuffmanChar {
                bits: entry.as_uint32(),
                num_bits: entry.num_bits(),
                key_code: KeyPress::truncate(key),
                is_mod: entry.is_mod,
            }.render(CCode::new())
            .initializer();

            initializers.push(init);
        }

        group.push(CTree::Array {
            name: "huffman_lookup".to_c(),
            values: initializers,
            c_type: "HuffmanChar".to_c(),
            is_extern: true,
        });

        Ok(CTree::Group(group))
    }
}

impl KeyPress {
    fn truncate(contents: &CCode) -> CCode {
        // CCode(format!("({})&0xff", contents))
        // TODO don't cast everything? Just enums.
        CCode(format!("static_cast<uint8_t>({})", contents))
    }

    fn format_mods(&self) -> CCode {
        // TODO think about this
        if self.mods.is_empty() {
            Self::empty_code()
        } else {
            Self::truncate(&CCode::join(&self.mods, "|"))
        }
    }

    fn empty_code() -> CCode {
        "0".to_c()
    }

    fn huffman(&self, table: &HuffmanTable) -> Result<BitVec<u8>, Error> {
        self.ensure_non_empty()?;

        let mut bits: BitVec<u8> = BitVec::default();
        for modifier in &self.mods {
            bits.extend(table.bits(modifier)?);
        }

        // There must be a key following the mod(s), so that we know when the
        // entry ends. If a keypress has a mod but no key,
        // use a blank dummy value for the key.
        bits.extend(table.bits(&self.key_or_blank())?);
        Ok(bits)
    }
}

impl Sequence {
    pub fn as_bytes(&self, table: &HuffmanTable) -> Result<Vec<CCode>, Error> {
        // TODO different name for "bytes"?
        Ok(self
            .as_bits(table)?
            .blocks()
            .map(|x: u8| x.to_c())
            .collect())
    }

    pub fn formatted_length_in_bits(
        &self,
        table: &HuffmanTable,
    ) -> Result<usize, Error> {
        // TODO don't compute twice!
        Ok(self.as_bits(table)?.len())
    }

    pub fn as_bits(&self, table: &HuffmanTable) -> Result<BitVec<u8>, Error> {
        let mut bits = BitVec::default();
        for keypress in self.keypresses() {
            bits.extend(keypress.huffman(table)?)
        }
        Ok(bits)
    }
}

/// Generate the top of the main auto_config files. If `with_message` is true,
/// add a commented message about when the file was autogenerated.
fn intro(with_message: bool) -> Result<CTree, Error> {
    let mut group = Vec::new();
    if with_message {
        let msg = autogen_message();
        group.push(CTree::LiteralC(msg.clone()));
        group.push(CTree::LiteralH(msg));
    }
    group.push(CTree::LiteralH("#pragma once\n".to_c()));
    group.push(CTree::IncludeSelf);

    group.push(CTree::IncludeH {
        path: "<stdint.h>".to_c(),
    });
    group.push(CTree::IncludeH {
        path: "\"config_types.h\"".to_c(),
    });

    group.push(CTree::LiteralH(
        "typedef void (*voidFuncPtr)(void);\n".to_c(),
    ));

    group.push(render_keycode_definitions());

    Ok(CTree::Group(group))
}

fn autogen_message() -> CCode {
    const AUTHOR: &str = "pipit-keyboard";

    let mut s = format!(
        "/**\n * Automatically generated by {} on:  {}\n",
        AUTHOR,
        now().strftime("%c").unwrap()
    );
    s += " * Do not make changes here, they will be overwritten.\n */\n\n";
    s.to_c()
}

fn make_debug_macros() -> CTree {
    // TODO clean up debug macros
    // TODO use Define variant instead
    let mut s = String::new();
    s += "\n#if DEBUG_MESSAGES == 0\n";
    s += "    #define DEBUG1(msg)\n";
    s += "    #define DEBUG1_LN(msg)\n";
    s += "    #define DEBUG2(msg)\n";
    s += "    #define DEBUG2_LN(msg)\n";
    s += "#else\n";
    s += "   #define ENABLE_SERIAL_DEBUG\n";
    s += "   #include <Arduino.h>\n";
    s += "   #define DEBUG1(msg) Serial.print(msg)\n";
    s += "   #define DEBUG1_LN(msg) Serial.println(msg)\n";
    s += "   #if DEBUG_MESSAGES == 1\n";
    s += "       #define DEBUG2(msg)\n";
    s += "       #define DEBUG2_LN(msg)\n";
    s += "   #else\n";
    s += "       #define DEBUG2(msg) Serial.print(msg)\n";
    s += "       #define DEBUG2_LN(msg) Serial.println(msg)\n";
    s += "   #endif\n\n";
    s += "#endif\n\n";
    CTree::LiteralH(CCode(s))
}

fn render_keycode_definitions() -> CTree {
    let keycodes = KeyDefs::scancode_table();
    let example = keycodes
        .keys()
        .nth(0)
        .expect("KeyDefs::scancode_table() is empty!")
        .to_owned();

    let keycode_definitions = CTree::Group(
        keycodes
            .iter()
            .map(|(&name, &value)| CTree::Define {
                name: name.to_owned(),
                value: value.to_c(),
            }).collect(),
    );

    CTree::Ifndef {
        conditional: example.to_owned(),
        contents: Box::new(keycode_definitions),
    }
}
