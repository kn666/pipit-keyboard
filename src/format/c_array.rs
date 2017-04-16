use std::fmt::Display;
use std::clone::Clone;
use format::format::Format;
use itertools::Itertools;



pub struct CArray <T> where T: Display + Clone
{
    name: String,
    is_extern: bool,
    c_type: String,
    contents_1d: Option<Vec<T>>,
    contents_2d: Option<Vec<Vec<T>>>,
}

impl <T> CArray<T> where T: Display + Clone
{
    pub fn new(name: &str) -> CArray<T>  {
        CArray {
            name: name.to_owned(),
            is_extern: true,
            c_type: "uint8_t".to_owned(),
            contents_1d: None,
            contents_2d: None,
        }
    }
    pub fn is_extern(mut self, is_extern: bool) -> CArray<T> {
        self.is_extern = is_extern;
        self
    }
    pub fn c_type(mut self, c_type: &str) -> CArray<T> {
        self.c_type = c_type.to_owned();
        self
    }
    pub fn fill_1d(mut self, contents: &Vec<T>) -> CArray<T> {
        self.contents_1d = Some(contents.to_vec());
        assert!(self.contents_2d.is_none());
        self
    }
    pub fn fill_2d(mut self, contents: &Vec<Vec<T>>) -> CArray<T> {
        self.contents_2d = Some(contents.to_vec());
        assert!(self.contents_1d.is_none());
        self
    }
    pub fn format(self) -> Format {
        if let Some(contents) = self.contents_1d {
            return format_c_array(&self.name, &contents, &self.c_type, self.is_extern);
        }
        else if let Some(contents) = self.contents_2d {
           return format_c_array2(&self.name, &contents, &self.c_type, self.is_extern);
        }
        else {
            panic!("CArray: no array contents were given");
        }
    }
}


fn format_c_array<T>(name: &str, v: &Vec<T>, ctype: &str, is_extern: bool) -> Format
    where T: Display + Clone
{
    let contents = make_c_array(&v);
    if is_extern {
        Format {
            h: format!("extern const {} {}[];\n", ctype, name),
            c: format!("extern const {} {}[] = {};\n\n", ctype, name, contents),
        }
    }
    else {
        Format {
            h: String::new(),
            c: format!("const {} {}[] = {};\n\n", ctype, name, contents),
        }
    }
}

fn format_c_array2<T>(name: &str, v: &Vec<Vec<T>>, ctype: &str, is_extern: bool) -> Format
    where T: Display
{
    let contents = make_c_array2(&v);
    let len_2nd_dim = v[0].len();

    if is_extern {
        Format {
            h: format!("extern const {} {}[][{}];\n",
                       ctype, name, len_2nd_dim),
            c: format!("extern const {} {}[][{}] = {};\n\n",
                       ctype, name, len_2nd_dim, contents),
        }
    }
    else {
        Format {
            h: String::new(),
            c: format!("const {} {}[][{}] = {};\n\n",
                       ctype, name, len_2nd_dim, contents),
        }
    }
}


fn make_c_array<T>(v: &Vec<T>) -> String
    where T: Display
{
    let lines = wrap_in_braces(&to_string_vec(v));
    // println!("{:?}", lines);
    lines.join("\n")
}

fn make_c_array2<T>(v: &Vec<Vec<T>>) -> String
    where T: Display
{
    assert!(is_rectangular(v));

    let mut rows: Vec<String> = Vec::new();
    for row in v {
        rows.extend(wrap_in_braces(&to_string_vec(&row)));
    }
    wrap_in_braces(&rows).join("\n")
}

fn wrap_in_braces(lines: &Vec<String>) -> Vec<String> {
    let mut new: Vec<_> = lines.iter().map(|s| format!(" {}", s)).collect();
    new.insert(0, "{".to_owned());
    new.push("}".to_owned());
    new
}

fn to_string_vec<T>(v: &Vec<T>) -> Vec<String>
    where T: Display
{
    let items_per_line = 4;
    let mut lines: Vec<String> = Vec::new();
    let chunks = &v.iter().map(|x| x.to_string()).chunks(items_per_line);
    for chunk in chunks {
        let tmp: Vec<_> = chunk.collect();
        lines.push(tmp.join(", ") + ", ");
    }
    lines
}

fn is_rectangular<T>(v: &Vec<Vec<T>>) -> bool
    where T: Display
{
    let len_2nd_dim = v[0].len();
    v.iter()
        .map(|v| v.len())
        .all(|x| x == len_2nd_dim)
}
