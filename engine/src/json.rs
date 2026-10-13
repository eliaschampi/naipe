//! A deliberately tiny JSON writer. The engine only ever *emits* JSON;
//! commands arrive as a compact pipe-delimited text protocol, so no parser
//! and no serialization dependency is needed.

pub struct Obj {
    s: String,
}

impl Obj {
    pub fn new() -> Self {
        Obj { s: String::from("{") }
    }
    fn sep(&mut self) {
        if self.s.len() > 1 {
            self.s.push(',');
        }
    }
    pub fn raw(mut self, k: &str, v: &str) -> Self {
        self.sep();
        self.s.push('"');
        self.s.push_str(k);
        self.s.push_str("\":");
        self.s.push_str(v);
        self
    }
    pub fn num(self, k: &str, v: i64) -> Self {
        let t = v.to_string();
        self.raw(k, &t)
    }
    pub fn flag(self, k: &str, v: bool) -> Self {
        self.raw(k, if v { "true" } else { "false" })
    }
    pub fn text(self, k: &str, v: &str) -> Self {
        let t = quote(v);
        self.raw(k, &t)
    }
    pub fn nil(self, k: &str) -> Self {
        self.raw(k, "null")
    }
    pub fn opt_i64(self, k: &str, v: Option<i64>) -> Self {
        match v {
            Some(n) => self.num(k, n),
            None => self.nil(k),
        }
    }
    pub fn done(mut self) -> String {
        self.s.push('}');
        self.s
    }
}

impl Default for Obj {
    fn default() -> Self {
        Self::new()
    }
}

pub fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => {}
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub fn arr<T, F: Fn(&T) -> String>(items: &[T], f: F) -> String {
    let mut out = String::from("[");
    for (i, it) in items.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&f(it));
    }
    out.push(']');
    out
}

pub fn usizes(items: &[usize]) -> String {
    arr(items, |v| v.to_string())
}

pub fn strings(items: &[String]) -> String {
    arr(items, |v| quote(v))
}
