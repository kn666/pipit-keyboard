use time::*;
use std::path::Path;

use types::{Sequence, KeyPress, Maps, Options, OpDef, OpType};
use format::{Format, CArray, format_lookups, compress, make_compression_macros};



impl KeyPress {
    pub fn as_bytes(&self, use_mods: bool) -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        v.push(format!("{}&0xff", self.key));
        if use_mods{
            v.push(format!("({})&0xff", self.modifier));
        }
        v
    }
}

impl Options {
    pub fn format(&self) -> Format {
        let mut f = Format::new();
        for (name, op) in self.get_non_internal() {
            f.append(&op.format(&name));
        }
        f.append_newline();
        f
    }
}

impl OpDef {
    pub fn format(&self, name: &str) -> Format {
        match self.op_type {
            OpType::DefineInt => {
                self.format_define_int(name)
            }
            OpType::DefineString => {
                self.format_define_string(name)
            }
            OpType::IfdefValue => {
                self.format_ifdef_value()
            }
            OpType::IfdefKey => {
                self.format_ifdef_key(name)
            }
            OpType::Uint8 => {
                self.format_uint8(name)
            }
            OpType::Array1D => {
                self.format_array1d(name)
            }
            OpType::Array2D => {
                self.format_array2d(name)
            }
            _ => panic!(format!("option cannot be formatted: {}", name)),
        }
    }

    fn format_define_int(&self, name: &str) -> Format {
        Format {
            h: format!("#define {} {}\n",
                       name.to_uppercase(),
                       self.get_val().unwrap_int()),
            c: String::new(),
        }
    }

    fn format_define_string(&self, name: &str) -> Format {
        Format {
            h: format!("#define {} {}\n",
                        name.to_uppercase(),
                        self.get_val().unwrap_str()),
            c: String::new(),
        }
    }

    fn format_ifdef_value(&self) -> Format {
        Format {
            h: format!("#define {}\n",
                       self.get_val().unwrap_str()),
            c: String::new(),
        }
    }

    fn format_ifdef_key(&self, name: &str) -> Format {
        Format {
            h: if self.get_val().unwrap_bool() {
                format!("#define {}\n",
                        name.to_uppercase())
            } else {
                String::new()
            },

            c: String::new(),
        }
    }

    fn format_uint8(&self, name: &str) -> Format {
        Format {
            h: format!("extern const uint8_t {};\n",
                       name),

            c: format!("extern const uint8_t {} = {};\n\n",
                       name,
                       self.get_val().unwrap_int()),
        }
    }

    fn format_array1d(&self, name: &str) -> Format {
        CArray::new(name)
            .fill_1d(self.get_val().unwrap_vec())
            .format()
    }


    fn format_array2d(&self, name: &str) -> Format {
            CArray::new(name)
            .fill_2d(self.get_val().unwrap_vec2())
            .format()
    }

}


impl Sequence {

    pub fn to_bytes(&self, use_compression: bool, use_mods: bool) -> Vec<String>{
        // TODO different name for "bytes"?
        if use_compression{
            self.to_compressed_bytes(use_mods)
        }
        else {
            self.to_raw_bytes(use_mods)
        }
    }

    fn to_raw_bytes(&self, use_mods: bool) -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        for keypress in self.0.iter() {
            v.extend(keypress.as_bytes(use_mods));
        }
        v
    }

    fn to_compressed_bytes(&self, use_mods: bool) -> Vec<String> {
        compress(self, use_mods)
    }

}


impl Maps {

    pub fn format(&self, file_name_base: &str) -> Format {
        let mut f = Format::new();
        f.append(&format_intro(&format!("{}.h", file_name_base)));
        f.append(&self.options.format());
        f.append(&self.format_wordmods());
        f.append(&self.format_specials());
        f.append(&self.format_plains());
        f.append(&self.format_macros());
        f.append(&self.format_words());
        f.append(&format_outro());
        f
    }


    fn format_words (&self) -> Format {
        let chord_map = &self.chords;
        format_lookups(&self.words, chord_map, "word", true, false)
    }

    fn format_plains (&self) -> Format {
        let chord_map = &self.chords;
        format_lookups(&self.plains, chord_map, "plain", false, true)
    }

    fn format_macros (&self) -> Format {
        let chord_map = &self.chords;
        format_lookups(&self.macros, chord_map, "macro", false, true)
    }

    fn format_specials (&self) -> Format {
        let mut f = Format {
            h: self.specials.keys()
                .fold(String::new(),
                      |acc, name|
                      acc + &format!("#define {} {}\n",
                                     name.to_uppercase(),
                                     self.specials[name].get_only_value()))
                + "\n",
            c: String::new(),
        };
        let chord_map = &self.chords;
        f.append(&format_lookups(&self.specials, chord_map, "special", false, false));
        f
    }

    fn format_wordmods(&self) -> Format {
        let mut f = Format::new();
        for name in &self.wordmods {
            let full_name = format!("{}_chord_bytes", name);
            f.append(&CArray::new(&full_name)
                     .fill_1d(&self.chords[name].to_ints())
                     .format())
        }
        f
    }
}


pub fn format_intro(h_file_name: &str) -> Format{
    let autogen_message = make_autogen_message();
    let guard_name = make_guard_name(h_file_name);
    let mut f = Format::new();

    f.h += &autogen_message;
    f.h += &format!("#ifndef {}\n#define {}\n\n", guard_name, guard_name);
    f.h += "#include <Arduino.h>\n";
    f.h += "#include \"keycodes.h\"\n\n";
    f.h += "typedef void (*voidFuncPtr)(void);\n\n";
    f.h += &make_compression_macros();

    f.c += &autogen_message;
    f.c += &format!("#include \"{}\"\n\n", h_file_name);
    f
}


fn make_debug_macros() -> String {
    // TODO clean up debug macros
    let mut s = String::new();
    s += "#if DEBUG_MESSAGES == 0\n";
    s += "#define DEBUG1(msg)\n";
    s += "#define DEBUG1_LN(msg)\n";
    s += "#define DEBUG2(msg)\n";
    s += "#define DEBUG2_LN(msg)\n";
    s += "#endif\n\n";
    s += "#if DEBUG_MESSAGES == 1\n";
    s += "#define DEBUG1(msg) Serial.print(msg)\n";
    s += "#define DEBUG1_LN(msg) Serial.println(msg)\n";
    s += "#define DEBUG2(msg)\n";
    s += "#define DEBUG2_LN(msg)\n";
    s += "#endif\n\n";
    s += "#if DEBUG_MESSAGES == 2\n";
    s += "#define DEBUG1(msg) Serial.print(msg)\n";
    s += "#define DEBUG1_LN(msg) Serial.println(msg)\n";
    s += "#define DEBUG2(msg) Serial.print(msg)\n";
    s += "#define DEBUG2_LN(msg) Serial.println(msg)\n";
    s += "#endif\n\n ";
    s
}

pub fn format_outro() -> Format {
    let mut f = Format::new();
    f.h += make_debug_macros().as_ref();
    f.h += "\n#endif\n";
    f
}

fn make_autogen_message( ) -> String {
    const AUTHOR: &str = "rusty-pipit";

    let s = format!("/**\n * Automatically generated by {} on:  {}\n",
                    AUTHOR,
                    now().strftime("%c").unwrap()
    );
    s + " * Do not make changes here, they will be overwritten.\n */\n\n"
}

fn make_guard_name(h_file_name: &str) -> String {
    // TODO remove unsafe characters, like the python version
    let error_message = format!("invalid header file name: {}", h_file_name);
    let p: String = Path::new(h_file_name)
        .file_name()
        .expect("failed to get file name")
        .to_str().unwrap().to_string()
        .to_uppercase()
        .chars()
        .map(|c| if c.is_alphanumeric() {c} else {'_'})
        .collect();
    let first = p.chars().nth(0)
        .expect(&error_message);
    if !first.is_alphabetic() && first != '_' {
        panic!(error_message);
    }
    p + "_"
}
