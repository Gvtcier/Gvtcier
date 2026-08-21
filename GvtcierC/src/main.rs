use std::env;
use std::fs;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: gvtcierc <file.gc>");
        std::process::exit(1);
    }
    let src = fs::read_to_string(&args[1]).expect("read source");
    let mut p = Parser::new(&src);
    let prog = p.parse_program();
    let asm = gen_asm(&prog);
    let out = args[1].replace(".gc", ".s");
    fs::write(&out, asm).expect("write asm");
    let gvtcierk = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("gvtcierk.exe")))
        .unwrap_or_else(|| std::path::PathBuf::from("gvtcierk.exe"));
    let st = Command::new(&gvtcierk)
        .arg(&out)
        .status()
        .expect("gvtcierk link");
    if !st.success() {
        eprintln!("link failed");
        std::process::exit(1);
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(i64),
    Flt(u32),
    Dbl(u64),
    Ident(String),
    Str(String),
    Kw(String),
    Op(String),
    Punct(char),
    Eof,
}

struct Lexer<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> Lexer<'a> {
    fn new(s: &'a str) -> Self {
        Lexer { s: s.as_bytes(), i: 0 }
    }
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }
    fn bump(&mut self) {
        self.i += 1;
    }
    fn skip_ws(&mut self) {
        loop {
            while matches!(self.peek(), Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r')) {
                self.bump();
            }
            if self.peek() == Some(b'/') && self.s.get(self.i + 1) == Some(&b'/') {
                while let Some(c) = self.peek() {
                    if c == b'\n' {
                        break;
                    }
                    self.bump();
                }
                continue;
            }
            if self.peek() == Some(b'/') && self.s.get(self.i + 1) == Some(&b'*') {
                self.bump();
                self.bump();
                while !(self.peek() == Some(b'*') && self.s.get(self.i + 1) == Some(&b'/')) {
                    self.bump();
                }
                self.bump();
                self.bump();
                continue;
            }
            break;
        }
    }
    fn next(&mut self) -> Tok {
        self.skip_ws();
        let c = match self.peek() {
            Some(c) => c,
            None => return Tok::Eof,
        };
        if c == b'\'' {
            self.bump();
            let ch = match self.peek() {
                Some(x) => x,
                None => 0,
            };
            self.bump();
            if self.peek() == Some(b'\'') {
                self.bump();
            }
            return Tok::Num(ch as i64);
        }
        if c.is_ascii_digit() {
            let mut v: f64 = 0.0;
            while let Some(d) = self.peek() {
                if d.is_ascii_digit() {
                    v = v * 10.0 + (d - b'0') as f64;
                    self.bump();
                } else {
                    break;
                }
            }
            if self.peek() == Some(b'.') {
                self.bump();
                let mut frac = 1.0;
                while let Some(d) = self.peek() {
                    if d.is_ascii_digit() {
                        frac *= 10.0;
                        v += (d - b'0') as f64 / frac;
                        self.bump();
                    } else {
                        break;
                    }
                }
                if self.peek() == Some(b'f') {
                    self.bump();
                    return Tok::Flt((v as f32).to_bits());
                }
                return Tok::Dbl(v.to_bits());
            }
            return Tok::Num(v as i64);
        }
        if c == b'"' {
            self.bump();
            let mut s = String::new();
            while let Some(ch) = self.peek() {
                if ch == b'"' {
                    self.bump();
                    break;
                }
                s.push(ch as char);
                self.bump();
            }
            return Tok::Str(s);
        }
        if c.is_ascii_alphabetic() || c == b'_' {
            let mut name = String::new();
            while let Some(ch) = self.peek() {
                if ch.is_ascii_alphanumeric() || ch == b'_' {
                    name.push(ch as char);
                    self.bump();
                } else {
                    break;
                }
            }
            return match name.as_str() {
                "ZhengShu" | "ZiFu" | "FuDian" | "ShuangJing" | "FanHui" | "RuGuo" | "FouZe"
                | "Dang" | "XunHuan" | "TiaoChu" | "JiXu" | "HuiBian" => {
                    Tok::Kw(name)
                }
                _ => Tok::Ident(name),
            };
        }
        if c == b'<' || c == b'>' || c == b'=' || c == b'!' || c == b'&' || c == b'|' {
            let two = if self.s.get(self.i + 1).is_some() {
                String::from_utf8_lossy(&self.s[self.i..self.i + 2]).to_string()
            } else {
                String::new()
            };
            match two.as_str() {
                "<=" | ">=" | "==" | "!=" | "&&" | "||" | "<<" | ">>" | "+=" | "-=" | "*="
                | "/=" | "%=" => {
                    self.bump();
                    self.bump();
                    return Tok::Op(two);
                }
                _ => {
                    self.bump();
                    return Tok::Op((c as char).to_string());
                }
            }
        }
        if c == b'~' || c == b'^' {
            self.bump();
            return Tok::Op((c as char).to_string());
        }
        if c == b'*' || c == b'/' || c == b'%' {
            if self.s.get(self.i + 1) == Some(&b'=') {
                self.bump();
                self.bump();
                return Tok::Op((c as char).to_string() + "=");
            }
        }
        if c == b'+' || c == b'-' {
            if self.s.get(self.i + 1) == Some(&b'=') {
                self.bump();
                self.bump();
                return Tok::Op((c as char).to_string() + "=");
            }
        }
        if b"+-*/%&*".contains(&c) {
            self.bump();
            return Tok::Op((c as char).to_string());
        }
        if b"(),;{}[]".contains(&c) {
            self.bump();
            return Tok::Punct(c as char);
        }
        panic!("unexpected char: {}", c as char);
    }
}

struct Parser<'a> {
    l: Lexer<'a>,
    tok: Tok,
}

#[derive(Clone, Debug)]
enum Expr {
    Num(i64),
    Flt(i64),
    Dbl(i64),
    Str(String),
    Var(String),
    Index(String, Box<Expr>),
    AssignIndex(String, Box<Expr>, Box<Expr>),
    Addr(String),
    Deref(Box<Expr>),
    Call(String, Vec<Expr>),
    Bin(String, Box<Expr>, Box<Expr>),
    Assign(String, Box<Expr>),
}

#[derive(Clone, Debug)]
enum Stmt {
    Return(Option<Expr>),
    Expr(Expr),
    Asm(String),
    If(Expr, Vec<Stmt>, Vec<Stmt>),
    While(Expr, Vec<Stmt>),
    For(Option<Expr>, Option<Expr>, Option<Expr>, Vec<Stmt>),
    Break,
    Continue,
    Decl(String, u8, Expr),
    ArrDecl(String, usize, u8),
}

#[derive(Clone, Debug)]
struct Func {
    name: String,
    params: Vec<String>,
    param_tys: Vec<u8>,
    ret_ty: u8,
    body: Vec<Stmt>,
}

#[derive(Clone, Debug)]
struct Program {
    funcs: Vec<Func>,
}

impl<'a> Parser<'a> {
    fn new(s: &'a str) -> Self {
        let mut l = Lexer::new(s);
        let tok = l.next();
        Parser { l, tok }
    }
    fn next(&mut self) {
        self.tok = self.l.next();
    }
    fn expect_punct(&mut self, c: char) {
        if self.tok == Tok::Punct(c) {
            self.next();
        } else {
            panic!("expected '{}' got {:?}", c, self.tok);
        }
    }
    fn expect_kw(&mut self, k: &str) {
        if self.tok == Tok::Kw(k.to_string()) {
            self.next();
        } else {
            panic!("expected keyword {}", k);
        }
    }
    fn parse_program(&mut self) -> Program {
        let mut funcs = Vec::new();
        while self.tok != Tok::Eof {
            let ty: u8 = match &self.tok {
                Tok::Kw(k) if k == "ZiFu" => {
                    self.next();
                    1
                }
                Tok::Kw(k) if k == "ZhengShu" => {
                    self.next();
                    0
                }
                Tok::Kw(k) if k == "FuDian" => {
                    self.next();
                    2
                }
                Tok::Kw(k) if k == "ShuangJing" => {
                    self.next();
                    3
                }
                _ => panic!("expected type"),
            };
            match &self.tok {
                Tok::Ident(name) => {
                    let n = name.clone();
                    self.next();
                    if self.tok == Tok::Punct('[') {
                        self.next();
                        if let Tok::Num(len) = self.tok {
                            self.next();
                            self.expect_punct(']');
                            self.expect_punct(';');
                            unsafe {
                                let nc = n.clone();
                                GLOBALS.push((nc.clone(), len as usize));
                                GLOBAL_KIND.push((nc, ty));
                            }
                            continue;
                        }
                    }
                    if self.tok == Tok::Punct(';') {
                        self.next();
                        unsafe {
                            let nc = n.clone();
                            GLOBALS.push((nc.clone(), 1));
                            GLOBAL_KIND.push((nc, ty));
                        }
                        continue;
                    }
                    self.expect_punct('(');
                    let mut params = Vec::new();
                    let mut param_tys = Vec::new();
                    if self.tok == Tok::Kw("ZhengShu".to_string())
                        || self.tok == Tok::Kw("ZiFu".to_string())
                        || self.tok == Tok::Kw("FuDian".to_string())
                        || self.tok == Tok::Kw("ShuangJing".to_string())
                    {
                        loop {
                            let pt = match &self.tok {
                                Tok::Kw(k) if k == "ZhengShu" => 0,
                                Tok::Kw(k) if k == "ZiFu" => 1,
                                Tok::Kw(k) if k == "FuDian" => 2,
                                Tok::Kw(k) if k == "ShuangJing" => 3,
                                _ => panic!("expected type"),
                            };
                            self.next();
                            match &self.tok {
                                Tok::Ident(p) => {
                                    params.push(p.clone());
                                    param_tys.push(pt);
                                }
                                _ => panic!("expected param name"),
                            }
                            self.next();
                            if self.tok == Tok::Punct(',') {
                                self.next();
                                continue;
                            }
                            break;
                        }
                    }
                    self.expect_punct(')');
                    self.expect_punct('{');
                    let body = self.parse_stmts();
                    self.expect_punct('}');
                    funcs.push(Func { name: n, params, param_tys, ret_ty: ty, body });
                }
                _ => panic!("expected function name"),
            }
        }
        Program { funcs }
    }
    fn parse_stmts(&mut self) -> Vec<Stmt> {
        let mut v = Vec::new();
        while self.tok != Tok::Punct('}') && self.tok != Tok::Eof {
            v.push(self.parse_stmt());
        }
        v
    }
    fn parse_stmt(&mut self) -> Stmt {
        if self.tok == Tok::Kw("HuiBian".to_string()) {
            self.next();
            self.expect_punct('(');
            let asm_text = match &self.tok {
                Tok::Str(s) => s.clone(),
                _ => panic!("expected string in HuiBian"),
            };
            self.next();
            self.expect_punct(')');
            self.expect_punct(';');
            return Stmt::Asm(asm_text);
        }
        if self.tok == Tok::Kw("FanHui".to_string()) {
            self.next();
            if self.tok == Tok::Punct(';') {
                self.next();
                return Stmt::Return(None);
            }
            let e = self.parse_expr();
            self.expect_punct(';');
            return Stmt::Return(Some(e));
        }
        if self.tok == Tok::Kw("RuGuo".to_string()) {
            self.next();
            self.expect_punct('(');
            let c = self.parse_expr();
            self.expect_punct(')');
            self.expect_punct('{');
            let then = self.parse_stmts();
            self.expect_punct('}');
            let mut els = Vec::new();
            if self.tok == Tok::Kw("FouZe".to_string()) {
                self.next();
                self.expect_punct('{');
                els = self.parse_stmts();
                self.expect_punct('}');
            }
            return Stmt::If(c, then, els);
        }
        if self.tok == Tok::Kw("Dang".to_string()) {
            self.next();
            self.expect_punct('(');
            let c = self.parse_expr();
            self.expect_punct(')');
            self.expect_punct('{');
            let body = self.parse_stmts();
            self.expect_punct('}');
            return Stmt::While(c, body);
        }
        if self.tok == Tok::Kw("XunHuan".to_string()) {
            self.next();
            self.expect_punct('(');
            let init = if self.tok == Tok::Punct(';') {
                None
            } else {
                Some(self.parse_expr())
            };
            self.expect_punct(';');
            let cond = if self.tok == Tok::Punct(';') {
                None
            } else {
                Some(self.parse_expr())
            };
            self.expect_punct(';');
            let step = if self.tok == Tok::Punct(')') {
                None
            } else {
                Some(self.parse_expr())
            };
            self.expect_punct(')');
            self.expect_punct('{');
            let body = self.parse_stmts();
            self.expect_punct('}');
            return Stmt::For(init, cond, step, body);
        }
        if self.tok == Tok::Kw("TiaoChu".to_string()) {
            self.next();
            self.expect_punct(';');
            return Stmt::Break;
        }
        if self.tok == Tok::Kw("JiXu".to_string()) {
            self.next();
            self.expect_punct(';');
            return Stmt::Continue;
        }
        let decl_ty: u8 = if self.tok == Tok::Kw("ZhengShu".to_string()) {
            0
        } else if self.tok == Tok::Kw("ZiFu".to_string()) {
            1
        } else if self.tok == Tok::Kw("FuDian".to_string()) {
            2
        } else if self.tok == Tok::Kw("ShuangJing".to_string()) {
            3
        } else {
            255
        };
        if decl_ty != 255 {
            self.next();
            if self.tok == Tok::Op("*".to_string()) {
                self.next();
                match &self.tok {
                    Tok::Ident(name) => {
                        let n = name.clone();
                        self.next();
                        if self.tok == Tok::Op("=".to_string()) {
                            self.next();
                            let e = self.parse_expr();
                            self.expect_punct(';');
                            return Stmt::Decl(n, 0, e);
                        }
                        panic!("expected =");
                    }
                    _ => panic!("expected pointer name"),
                }
            }
            match &self.tok {
                Tok::Ident(name) => {
                    let n = name.clone();
                    self.next();
                    if self.tok == Tok::Punct('[') {
                        self.next();
                        if let Tok::Num(len) = self.tok {
                            self.next();
                            self.expect_punct(']');
                            self.expect_punct(';');
                            return Stmt::ArrDecl(n, len as usize, decl_ty);
                        }
                    }
                    if self.tok == Tok::Op("=".to_string()) {
                        self.next();
                        let e = self.parse_expr();
                        self.expect_punct(';');
                        return Stmt::Decl(n, decl_ty, e);
                    }
                    self.expect_punct(';');
                    return Stmt::Decl(n, decl_ty, Expr::Num(0));
                }
                _ => {
                    let e = self.parse_expr();
                    self.expect_punct(';');
                    return Stmt::Expr(e);
                }
            }
        }
        let e = self.parse_expr();
        self.expect_punct(';');
        Stmt::Expr(e)
    }
    fn parse_expr(&mut self) -> Expr {
        self.parse_assign()
    }
    fn parse_assign(&mut self) -> Expr {
        let left = self.parse_or();
        if self.tok == Tok::Op("=".to_string()) {
            self.next();
            let rhs = self.parse_assign();
            if let Expr::Var(name) = left {
                return Expr::Assign(name, Box::new(rhs));
            }
            if let Expr::Index(name, idx) = left {
                return Expr::AssignIndex(name, idx, Box::new(rhs));
            }
            panic!("assign target must be variable");
        }
        if let Tok::Op(op) = &self.tok {
            if matches!(op.as_str(), "+=" | "-=" | "*=" | "/=" | "%=") {
                let o = op.clone();
                self.next();
                let rhs = self.parse_assign();
                if let Expr::Var(name) = left {
                    let bin = Expr::Bin(
                        o.trim_end_matches('=').to_string(),
                        Box::new(Expr::Var(name.clone())),
                        Box::new(rhs),
                    );
                    return Expr::Assign(name, Box::new(bin));
                }
            }
        }
        left
    }
    fn parse_or(&mut self) -> Expr {
        let mut left = self.parse_and();
        while self.tok == Tok::Op("||".to_string()) {
            self.next();
            let right = self.parse_and();
            left = Expr::Bin("||".to_string(), Box::new(left), Box::new(right));
        }
        left
    }
    fn parse_and(&mut self) -> Expr {
        let mut left = self.parse_bitor();
        while self.tok == Tok::Op("&&".to_string()) {
            self.next();
            let right = self.parse_bitor();
            left = Expr::Bin("&&".to_string(), Box::new(left), Box::new(right));
        }
        left
    }
    fn parse_bitor(&mut self) -> Expr {
        let mut left = self.parse_bitxor();
        while self.tok == Tok::Op("|".to_string()) {
            self.next();
            let right = self.parse_bitxor();
            left = Expr::Bin("|".to_string(), Box::new(left), Box::new(right));
        }
        left
    }
    fn parse_bitxor(&mut self) -> Expr {
        let mut left = self.parse_bitand();
        while self.tok == Tok::Op("^".to_string()) {
            self.next();
            let right = self.parse_bitand();
            left = Expr::Bin("^".to_string(), Box::new(left), Box::new(right));
        }
        left
    }
    fn parse_bitand(&mut self) -> Expr {
        let mut left = self.parse_shift();
        while self.tok == Tok::Op("&".to_string()) {
            self.next();
            let right = self.parse_shift();
            left = Expr::Bin("&".to_string(), Box::new(left), Box::new(right));
        }
        left
    }
    fn parse_shift(&mut self) -> Expr {
        let mut left = self.parse_eq();
        while self.tok == Tok::Op("<<".to_string()) || self.tok == Tok::Op(">>".to_string()) {
            let op = match &self.tok {
                Tok::Op(o) => o.clone(),
                _ => unreachable!(),
            };
            self.next();
            let right = self.parse_eq();
            left = Expr::Bin(op, Box::new(left), Box::new(right));
        }
        left
    }
    fn parse_eq(&mut self) -> Expr {
        let mut left = self.parse_rel();
        while self.tok == Tok::Op("==".to_string()) || self.tok == Tok::Op("!=".to_string()) {
            let op = match &self.tok {
                Tok::Op(o) => o.clone(),
                _ => unreachable!(),
            };
            self.next();
            let right = self.parse_rel();
            left = Expr::Bin(op, Box::new(left), Box::new(right));
        }
        left
    }
    fn parse_rel(&mut self) -> Expr {
        let mut left = self.parse_add();
        while matches!(
            &self.tok,
            Tok::Op(o) if o == "<" || o == ">" || o == "<=" || o == ">="
        ) {
            let op = match &self.tok {
                Tok::Op(o) => o.clone(),
                _ => unreachable!(),
            };
            self.next();
            let right = self.parse_add();
            left = Expr::Bin(op, Box::new(left), Box::new(right));
        }
        left
    }
    fn parse_add(&mut self) -> Expr {
        let mut left = self.parse_mul();
        while matches!(&self.tok, Tok::Op(o) if o == "+" || o == "-") {
            let op = match &self.tok {
                Tok::Op(o) => o.clone(),
                _ => unreachable!(),
            };
            self.next();
            let right = self.parse_mul();
            left = Expr::Bin(op, Box::new(left), Box::new(right));
        }
        left
    }
    fn parse_mul(&mut self) -> Expr {
        let mut left = self.parse_unary();
        while matches!(&self.tok, Tok::Op(o) if o == "*" || o == "/" || o == "%") {
            let op = match &self.tok {
                Tok::Op(o) => o.clone(),
                _ => unreachable!(),
            };
            self.next();
            let right = self.parse_unary();
            left = Expr::Bin(op, Box::new(left), Box::new(right));
        }
        left
    }
    fn parse_unary(&mut self) -> Expr {
        if self.tok == Tok::Op("+".to_string()) {
            self.next();
            return self.parse_unary();
        }
        if self.tok == Tok::Op("-".to_string()) {
            self.next();
            let e = self.parse_unary();
            return Expr::Bin("-".to_string(), Box::new(Expr::Num(0)), Box::new(e));
        }
        if self.tok == Tok::Op("!".to_string()) {
            self.next();
            let e = self.parse_unary();
            return Expr::Bin("==".to_string(), Box::new(e), Box::new(Expr::Num(0)));
        }
        if self.tok == Tok::Op("~".to_string()) {
            self.next();
            let e = self.parse_unary();
            return Expr::Bin("^".to_string(), Box::new(e), Box::new(Expr::Num(-1)));
        }
        if self.tok == Tok::Op("&".to_string()) {
            self.next();
            match self.parse_unary() {
                Expr::Var(name) => return Expr::Addr(name),
                Expr::Index(name, idx) => {
                    let base = Expr::Var(name);
                    return Expr::Bin("+".to_string(), Box::new(base), idx);
                }
                other => panic!("& requires variable"),
            }
        }
        if self.tok == Tok::Op("*".to_string()) {
            self.next();
            let e = self.parse_unary();
            return Expr::Deref(Box::new(e));
        }
        self.parse_primary()
    }
    fn parse_primary(&mut self) -> Expr {
        match &self.tok {
            Tok::Num(n) => {
                let v = *n;
                self.next();
                Expr::Num(v)
            }
            Tok::Flt(bits) => {
                let v = *bits as i64;
                self.next();
                Expr::Flt(v)
            }
            Tok::Dbl(bits) => {
                let v = *bits as i64;
                self.next();
                Expr::Dbl(v)
            }
            Tok::Str(s) => {
                let v = s.clone();
                self.next();
                Expr::Str(v)
            }
            Tok::Ident(name) => {
                let n = name.clone();
                self.next();
                if self.tok == Tok::Punct('(') {
                    self.next();
                    let mut args = Vec::new();
                    if self.tok != Tok::Punct(')') {
                        loop {
                            args.push(self.parse_expr());
                            if self.tok == Tok::Punct(',') {
                                self.next();
                                continue;
                            }
                            break;
                        }
                    }
                    self.expect_punct(')');
                    Expr::Call(n, args)
                } else if self.tok == Tok::Punct('[') {
                    self.next();
                    let idx = self.parse_expr();
                    self.expect_punct(']');
                    Expr::Index(n, Box::new(idx))
                } else {
                    Expr::Var(n)
                }
            }
            Tok::Punct('(') => {
                self.next();
                let e = self.parse_expr();
                self.expect_punct(')');
                e
            }
            _ => panic!("unexpected token: {:?}", self.tok),
        }
    }
}

static mut LABEL_ID: usize = 0;
static mut LOOP_STACK: Vec<(usize, bool)> = Vec::new();
static mut ARR_CUR: usize = 0;
static mut STR_LIST: Vec<String> = Vec::new();
static mut GLOBALS: Vec<(String, usize)> = Vec::new();
static mut GLOBAL_KIND: Vec<(String, u8)> = Vec::new();

fn collect_vars(
    f: &Func,
) -> (
    std::collections::HashMap<String, usize>,
    std::collections::HashMap<String, u8>,
) {
    let mut map = std::collections::HashMap::new();
    let mut types = std::collections::HashMap::new();
    for (i, p) in f.params.iter().enumerate() {
        map.insert(p.clone(), i + 1);
        types.insert(p.clone(), f.param_tys.get(i).copied().unwrap_or(0));
    }
    let mut next = f.params.len() + 1;
    fn walk(
        map: &mut std::collections::HashMap<String, usize>,
        types: &mut std::collections::HashMap<String, u8>,
        s: &Stmt,
        next: &mut usize,
    ) {
        match s {
            Stmt::Asm(_) => {}
            Stmt::Return(Some(e)) | Stmt::Expr(e) => walk_expr(map, types, e, next),
            Stmt::Return(None) => {}
            Stmt::If(c, t, els) => {
                walk_expr(map, types, c, next);
                for s2 in t {
                    walk(map, types, s2, next);
                }
                for s2 in els {
                    walk(map, types, s2, next);
                }
            }
            Stmt::While(c, b) => {
                walk_expr(map, types, c, next);
                for s2 in b {
                    walk(map, types, s2, next);
                }
            }
            Stmt::For(init, cond, step, b) => {
                if let Some(e) = init {
                    walk_expr(map, types, e, next);
                }
                if let Some(e) = cond {
                    walk_expr(map, types, e, next);
                }
                if let Some(e) = step {
                    walk_expr(map, types, e, next);
                }
                for s2 in b {
                    walk(map, types, s2, next);
                }
            }
            Stmt::Break | Stmt::Continue => {}
            Stmt::Decl(name, kind, init) => {
                if !map.contains_key(name) {
                    map.insert(name.clone(), *next);
                    *next += 1;
                }
                types.insert(name.clone(), *kind);
                walk_expr(map, types, init, next);
            }
            Stmt::ArrDecl(name, _len, kind) => {
                if !map.contains_key(name) {
                    map.insert(name.clone(), *next);
                    *next += 1;
                }
                types.insert(name.clone(), *kind);
            }
        }
    }
    fn walk_expr(
        map: &mut std::collections::HashMap<String, usize>,
        types: &mut std::collections::HashMap<String, u8>,
        e: &Expr,
        next: &mut usize,
    ) {
        match e {
            Expr::Var(name) | Expr::Assign(name, _) | Expr::Index(name, _) => {
                let is_global = unsafe { GLOBALS.iter().any(|(g, _)| *g == *name) };
                if !is_global && !map.contains_key(name) {
                    map.insert(name.clone(), *next);
                    *next += 1;
                }
                if let Expr::Assign(n, rhs) = e {
                    if !is_global && !types.contains_key(n) {
                        let k = match rhs.as_ref() {
                            Expr::Flt(_) => 2,
                            Expr::Dbl(_) => 3,
                            _ => 0,
                        };
                        types.insert(n.clone(), k);
                    }
                }
                match e {
                    Expr::Assign(_, rhs) => walk_expr(map, types, rhs, next),
                    Expr::Index(_, idx) => walk_expr(map, types, idx, next),
                    _ => {}
                }
            }
            Expr::AssignIndex(name, idx, rhs) => {
                let is_global = unsafe { GLOBALS.iter().any(|(g, _)| *g == *name) };
                if !is_global && !map.contains_key(name) {
                    map.insert(name.clone(), *next);
                    *next += 1;
                }
                walk_expr(map, types, idx, next);
                walk_expr(map, types, rhs, next);
            }
            Expr::Num(_) => {}
            Expr::Flt(_) => {}
            Expr::Dbl(_) => {}
            Expr::Str(_) => {}
            Expr::Addr(name) => {
                if !map.contains_key(name) {
                    map.insert(name.clone(), *next);
                    *next += 1;
                }
            }
            Expr::Deref(e) => walk_expr(map, types, e, next),
            Expr::Call(_, args) => {
                for a in args {
                    walk_expr(map, types, a, next);
                }
            }
            Expr::Bin(_, a, b) => {
                walk_expr(map, types, a, next);
                walk_expr(map, types, b, next);
            }
        }
    }
    for s in &f.body {
        walk(&mut map, &mut types, s, &mut next);
    }
    (map, types)
}

fn gen_asm(prog: &Program) -> String {
    let mut out = String::new();
    out.push_str(".intel_syntax noprefix\n");
    out.push_str(".text\n");
    let fn_ret: std::collections::HashMap<String, u8> = prog
        .funcs
        .iter()
        .map(|f| (f.name.clone(), f.ret_ty))
        .collect();
    let fn_params: std::collections::HashMap<String, Vec<u8>> = prog
        .funcs
        .iter()
        .map(|f| (f.name.clone(), f.param_tys.clone()))
        .collect();
    for f in &prog.funcs {
        let (slots, types) = collect_vars(f);
        let mut arr_total = 0;
        for s in &f.body {
            if let Stmt::ArrDecl(_, len, kind) = s {
                arr_total += len * if *kind == 3 { 8 } else if *kind == 1 { 1 } else { 4 };
            }
        }
        unsafe {
            ARR_CUR = 0;
        }
        let mut frame = slots.len() * 8 + arr_total + 16;
        frame = (frame + 15) & !15;
        out.push_str(&format!(".global {}\n{}:\n", f.name, f.name));
        out.push_str("  push rbp\n  mov rbp, rsp\n");
        out.push_str(&format!("  sub rsp, {}\n", frame));
        for (i, p) in f.params.iter().enumerate() {
            let slot = slots[p] * 8;
            let pt = f.param_tys.get(i).copied().unwrap_or(0);
            if i < 4 {
                let reg = ["rcx", "rdx", "r8", "r9"][i];
                if pt == 2 {
                    let r32 = match reg {
                        "rcx" => "ecx",
                        "rdx" => "edx",
                        "r8" => "r8d",
                        _ => "r9d",
                    };
                    out.push_str(&format!("  mov [rbp-{}], {}\n", slot, r32));
                } else {
                    out.push_str(&format!("  mov [rbp-{}], {}\n", slot, reg));
                }
            } else {
                out.push_str(&format!(
                    "  mov rax, [rbp+{}]\n  mov [rbp-{}], rax\n",
                    16 + (i - 4) * 8,
                    slot
                ));
            }
        }
        for s in &f.body {
            gen_stmt(&mut out, s, &slots, &types, &fn_ret, &fn_params, f.ret_ty);
        }
        out.push_str("  mov eax, 0\n  leave\n  ret\n");
    }
    out.push_str(".global DaYin\nDaYin:\n");
    out.push_str("  push rbp\n  mov rbp, rsp\n  sub rsp, 32\n");
    out.push_str("  call puts\n");
    out.push_str("  leave\n  ret\n");
    out.push_str(".data\n");
    let strs = unsafe { STR_LIST.clone() };
    for (i, s) in strs.iter().enumerate() {
        out.push_str(&format!(".Lstr{}:\n", i));
        out.push_str("  .ascii \"");
        for b in s.bytes() {
            match b {
                b'\\' => out.push_str("\\\\"),
                b'"' => out.push_str("\\\""),
                b'\n' => out.push_str("\\n"),
                _ => out.push((b as char)),
            }
        }
        out.push_str("\"\n  .byte 0\n");
    }
    out.push_str(".bss\n");
    let globs = unsafe { GLOBALS.clone() };
    let kinds = unsafe { GLOBAL_KIND.clone() };
    for (n, len) in globs.iter() {
        out.push_str(&format!(".global {}\n{}:\n", n, n));
        let k = kinds
            .iter()
            .find(|(g, _)| g == n)
            .map(|(_, k)| *k)
            .unwrap_or(0);
        let esz = if k == 3 { 8 } else if k == 1 { 1 } else { 4 };
        out.push_str(&format!("  .zero {}\n", len * esz));
    }
    out
}

fn global_kind(name: &str) -> u8 {
    unsafe {
        GLOBAL_KIND
            .iter()
            .find(|(g, _)| g == name)
            .map(|(_, k)| *k)
            .unwrap_or(0)
    }
}

fn expr_type(
    e: &Expr,
    types: &std::collections::HashMap<String, u8>,
    fn_ret: &std::collections::HashMap<String, u8>,
) -> u8 {
    match e {
        Expr::Num(_) | Expr::Str(_) | Expr::Addr(_) | Expr::Deref(_) => 0,
        Expr::Flt(_) => 2,
        Expr::Dbl(_) => 3,
        Expr::Var(name) | Expr::Index(name, _) => {
            let is_global = unsafe { GLOBALS.iter().any(|(g, _)| *g == *name) };
            if is_global {
                global_kind(name)
            } else {
                types.get(name).copied().unwrap_or(0)
            }
        }
        Expr::Assign(_, rhs) => expr_type(rhs, types, fn_ret),
        Expr::AssignIndex(_, _, rhs) => expr_type(rhs, types, fn_ret),
        Expr::Call(name, _) => fn_ret.get(name).copied().unwrap_or(0),
        Expr::Bin(op, a, b) => {
            let ta = expr_type(a, types, fn_ret);
            let tb = expr_type(b, types, fn_ret);
            match op.as_str() {
                "+" | "-" | "*" | "/" | "%" => {
                    if ta == 3 || tb == 3 {
                        3
                    } else if ta == 2 || tb == 2 {
                        2
                    } else {
                        0
                    }
                }
                _ => 0,
            }
        }
    }
}

fn conv_reg_xmm(out: &mut String, ty: u8, reg: &str, xmm: &str, fmt: &str) {
    let r32 = if reg == "rax" { "eax" } else { "ecx" };
    match ty {
        0 | 1 => out.push_str(&format!("  cvtsi2{} {}, {}\n", fmt, xmm, r32)),
        2 => {
            if fmt == "ss" {
                out.push_str(&format!("  movd {}, {}\n", xmm, r32));
            } else {
                out.push_str(&format!("  movd {}, {}\n  cvtss2sd {}, {}\n", xmm, r32, xmm, xmm));
            }
        }
        _ => {
            if fmt == "ss" {
                out.push_str(&format!("  movq {}, {}\n  cvtsd2ss {}, {}\n", xmm, reg, xmm, xmm));
            } else {
                out.push_str(&format!("  movq {}, {}\n", xmm, reg));
            }
        }
    }
}

fn emit_conv(out: &mut String, from: u8, to: u8) {
    let fi = from == 0 || from == 1;
    let ti = to == 0 || to == 1;
    if (fi && ti) || (from == to) {
        return;
    }
    match to {
        2 => {
            if fi {
                out.push_str("  cvtsi2ss xmm0, eax\n  movd eax, xmm0\n");
            } else {
                out.push_str("  movq xmm0, rax\n  cvtsd2ss xmm0, xmm0\n  movd eax, xmm0\n");
            }
        }
        3 => {
            if fi {
                out.push_str("  cvtsi2sd xmm0, eax\n  movq rax, xmm0\n");
            } else {
                out.push_str("  movd xmm0, eax\n  cvtss2sd xmm0, xmm0\n  movq rax, xmm0\n");
            }
        }
        _ => {
            if from == 2 {
                out.push_str("  movd xmm0, eax\n  cvttss2si rax, xmm0\n");
            } else {
                out.push_str("  movq xmm0, rax\n  cvttsd2si rax, xmm0\n");
            }
        }
    }
}

fn gen_expr(
    out: &mut String,
    e: &Expr,
    slots: &std::collections::HashMap<String, usize>,
    types: &std::collections::HashMap<String, u8>,
    fn_ret: &std::collections::HashMap<String, u8>,
    fn_params: &std::collections::HashMap<String, Vec<u8>>,
) {
    match e {
        Expr::Num(n) => out.push_str(&format!("  mov rax, {}\n", n)),
        Expr::Flt(n) => out.push_str(&format!("  mov rax, {}\n", n)),
        Expr::Dbl(n) => out.push_str(&format!("  mov rax, {}\n", n)),
        Expr::Str(s) => {
            let idx = unsafe {
                let mut found = None;
                for (i, x) in STR_LIST.iter().enumerate() {
                    if x == s {
                        found = Some(i);
                        break;
                    }
                }
                match found {
                    Some(i) => i,
                    None => {
                        STR_LIST.push(s.clone());
                        STR_LIST.len() - 1
                    }
                }
            };
            out.push_str(&format!("  lea rax, [rip + .Lstr{}]\n", idx));
        }
        Expr::Var(name) => {
            let is_global = unsafe { GLOBALS.iter().any(|(g, _)| *g == *name) };
            let k = if is_global {
                global_kind(name)
            } else {
                types.get(name).copied().unwrap_or(0)
            };
            if is_global {
                let g_len = unsafe {
                    GLOBALS
                        .iter()
                        .find(|(g, _)| *g == *name)
                        .map(|(_, l)| *l)
                        .unwrap_or(1)
                };
                if g_len == 1 {
                    if k == 2 {
                        out.push_str(&format!("  mov eax, [rip + {}]\n", name));
                    } else if k == 3 {
                        out.push_str(&format!("  mov rax, [rip + {}]\n", name));
                    } else {
                        out.push_str(&format!("  mov eax, [rip + {}]\n", name));
                        out.push_str("  movsxd rax, eax\n");
                    }
                } else {
                    out.push_str(&format!("  lea rax, [rip + {}]\n", name));
                }
            } else {
                let slot = slots[name] * 8;
                if k == 2 {
                    out.push_str(&format!("  mov eax, [rbp-{}]\n", slot));
                } else if k == 3 {
                    out.push_str(&format!("  mov rax, [rbp-{}]\n", slot));
                } else {
                    out.push_str(&format!("  mov rax, [rbp-{}]\n", slot));
                }
            }
        }
        Expr::Index(name, idx) => {
            gen_expr(out, idx, slots, types, fn_ret, fn_params);
            out.push_str("  mov rcx, rax\n");
            let is_global = unsafe { GLOBALS.iter().any(|(g, _)| *g == *name) };
            if is_global {
                out.push_str(&format!("  lea rax, [rip + {}]\n", name));
            } else {
                let slot = slots[name] * 8;
                out.push_str(&format!("  mov rax, [rbp-{}]\n", slot));
            }
            let k = if is_global {
                global_kind(name)
            } else {
                types.get(name).copied().unwrap_or(0)
            };
            if k == 1 {
                out.push_str("  movzx rax, byte ptr [rax+rcx]\n");
            } else if k == 3 {
                out.push_str("  mov rax, [rax+rcx*8]\n");
            } else {
                out.push_str("  mov eax, [rax+rcx*4]\n");
            }
        }
        Expr::AssignIndex(name, idx, rhs) => {
            gen_expr(out, rhs, slots, types, fn_ret, fn_params);
            out.push_str("  push rax\n");
            gen_expr(out, idx, slots, types, fn_ret, fn_params);
            out.push_str("  mov rcx, rax\n");
            let is_global = unsafe { GLOBALS.iter().any(|(g, _)| *g == *name) };
            if is_global {
                out.push_str(&format!("  lea rax, [rip + {}]\n", name));
            } else {
                let slot = slots[name] * 8;
                out.push_str(&format!("  mov rax, [rbp-{}]\n", slot));
            }
            out.push_str("  pop rdx\n");
            let k = if is_global {
                global_kind(name)
            } else {
                types.get(name).copied().unwrap_or(0)
            };
            if k == 1 {
                out.push_str("  mov byte ptr [rax+rcx], dl\n");
            } else if k == 3 {
                out.push_str("  mov [rax+rcx*8], rdx\n");
            } else {
                out.push_str("  mov [rax+rcx*4], edx\n");
            }
        }
        Expr::Addr(name) => {
            let slot = slots[name] * 8;
            out.push_str(&format!("  lea rax, [rbp-{}]\n", slot));
        }
        Expr::Deref(e) => {
            gen_expr(out, e, slots, types, fn_ret, fn_params);
            out.push_str("  mov eax, [rax]\n");
        }
        Expr::Call(name, args) => {
            if name == "DaYin" && args.len() == 1 {
                gen_expr(out, &args[0], slots, types, fn_ret, fn_params);
                out.push_str("  push rax\n");
                let is_num =
                    expr_type(&args[0], types, fn_ret) == 0 && !matches!(&args[0], Expr::Str(_));
                out.push_str(&format!(
                    "  mov rax, {}\n",
                    if is_num { 1 } else { 0 }
                ));
                out.push_str("  push rax\n");
                out.push_str("  pop rdx\n");
                out.push_str("  pop rcx\n");
                out.push_str("  call DaYin\n");
                return;
            }
            let pty = fn_params.get(name).cloned().unwrap_or_default();
            if args.len() > 4 {
                for (ai, a) in args.iter().skip(4).rev().enumerate() {
                    gen_expr(out, a, slots, types, fn_ret, fn_params);
                    let at = expr_type(a, types, fn_ret);
                    let pt = pty.get(args.len() - 1 - ai).copied().unwrap_or(0);
                    emit_conv(out, at, pt);
                    out.push_str("  push rax\n");
                }
            }
            for (i, a) in args.iter().take(4).enumerate() {
                gen_expr(out, a, slots, types, fn_ret, fn_params);
                let at = expr_type(a, types, fn_ret);
                let pt = pty.get(i).copied().unwrap_or(0);
                emit_conv(out, at, pt);
                out.push_str("  push rax\n");
            }
            for i in (0..args.len().min(4)).rev() {
                out.push_str(&format!(
                    "  pop {}\n",
                    ["rcx", "rdx", "r8", "r9"][i]
                ));
            }
            out.push_str(&format!("  call {}\n", name));
            if args.len() > 4 {
                out.push_str(&format!("  add rsp, {}\n", (args.len() - 4) * 8));
            }
        }
        Expr::Bin(op, a, b) => {
            let ta = expr_type(a, types, fn_ret);
            let tb = expr_type(b, types, fn_ret);
            gen_expr(out, a, slots, types, fn_ret, fn_params);
            out.push_str("  push rax\n");
            gen_expr(out, b, slots, types, fn_ret, fn_params);
            out.push_str("  mov rcx, rax\n  pop rax\n");
            match op.as_str() {
                "+" | "-" | "*" | "/" if ta == 3 || tb == 3 => {
                    conv_reg_xmm(out, ta, "rax", "xmm0", "sd");
                    conv_reg_xmm(out, tb, "rcx", "xmm1", "sd");
                    let inst = match op.as_str() {
                        "+" => "addsd",
                        "-" => "subsd",
                        "*" => "mulsd",
                        _ => "divsd",
                    };
                    out.push_str(&format!("  {} xmm0, xmm1\n  movq rax, xmm0\n", inst));
                }
                "+" | "-" | "*" | "/" if ta == 2 || tb == 2 => {
                    conv_reg_xmm(out, ta, "rax", "xmm0", "ss");
                    conv_reg_xmm(out, tb, "rcx", "xmm1", "ss");
                    let inst = match op.as_str() {
                        "+" => "addss",
                        "-" => "subss",
                        "*" => "mulss",
                        _ => "divss",
                    };
                    out.push_str(&format!("  {} xmm0, xmm1\n  movd eax, xmm0\n", inst));
                }
                "<" | ">" | "<=" | ">=" | "==" | "!=" if ta == 3 || tb == 3 => {
                    conv_reg_xmm(out, ta, "rax", "xmm0", "sd");
                    conv_reg_xmm(out, tb, "rcx", "xmm1", "sd");
                    out.push_str("  ucomisd xmm0, xmm1\n");
                    let set = match op.as_str() {
                        "<" => "setb",
                        ">" => "seta",
                        "<=" => "setbe",
                        ">=" => "setae",
                        "==" => "sete",
                        _ => "setne",
                    };
                    out.push_str(&format!("  {} al\n  movzx eax, al\n", set));
                }
                "<" | ">" | "<=" | ">=" | "==" | "!=" if ta == 2 || tb == 2 => {
                    conv_reg_xmm(out, ta, "rax", "xmm0", "ss");
                    conv_reg_xmm(out, tb, "rcx", "xmm1", "ss");
                    out.push_str("  ucomiss xmm0, xmm1\n");
                    let set = match op.as_str() {
                        "<" => "setb",
                        ">" => "seta",
                        "<=" => "setbe",
                        ">=" => "setae",
                        "==" => "sete",
                        _ => "setne",
                    };
                    out.push_str(&format!("  {} al\n  movzx eax, al\n", set));
                }
                "+" => out.push_str("  add rax, rcx\n"),
                "-" => out.push_str("  sub rax, rcx\n"),
                "*" => out.push_str("  imul rax, rcx\n"),
                "/" => out.push_str("  cqo\n  idiv rcx\n"),
                "%" => out.push_str("  cqo\n  idiv rcx\n  mov rax, rdx\n"),
                "<" | ">" | "<=" | ">=" | "==" | "!=" => {
                    out.push_str("  cmp eax, ecx\n");
                    let set = match op.as_str() {
                        "<" => "setl",
                        ">" => "setg",
                        "<=" => "setle",
                        ">=" => "setge",
                        "==" => "sete",
                        _ => "setne",
                    };
                    out.push_str(&format!("  {} al\n  movzx eax, al\n", set));
                }
                "&&" => {
                    out.push_str("  and eax, ecx\n  cmp eax, 0\n  setne al\n  movzx eax, al\n");
                }
                "||" => {
                    out.push_str("  or eax, ecx\n  cmp eax, 0\n  setne al\n  movzx eax, al\n");
                }
                "&" => out.push_str("  and eax, ecx\n"),
                "|" => out.push_str("  or eax, ecx\n"),
                "^" => out.push_str("  xor eax, ecx\n"),
                "<<" => out.push_str("  shl eax, cl\n"),
                ">>" => out.push_str("  sar eax, cl\n"),
                _ => panic!("unknown op"),
            }
        }
        Expr::Assign(name, rhs) => {
            let is_global = unsafe { GLOBALS.iter().any(|(g, _)| *g == *name) };
            let tk = if is_global {
                global_kind(name)
            } else {
                types.get(name).copied().unwrap_or(0)
            };
            gen_expr(out, rhs, slots, types, fn_ret, fn_params);
            let rt = expr_type(rhs, types, fn_ret);
            emit_conv(out, rt, tk);
            if is_global {
                out.push_str(&format!("  lea rcx, [rip + {}]\n", name));
                if tk == 3 {
                    out.push_str("  mov [rcx], rax\n");
                } else {
                    out.push_str("  mov [rcx], eax\n");
                }
            } else {
                let slot = slots[name] * 8;
                out.push_str(&format!("  mov [rbp-{}], rax\n", slot));
            }
        }
    }
}

fn gen_stmt(
    out: &mut String,
    s: &Stmt,
    slots: &std::collections::HashMap<String, usize>,
    types: &std::collections::HashMap<String, u8>,
    fn_ret: &std::collections::HashMap<String, u8>,
    fn_params: &std::collections::HashMap<String, Vec<u8>>,
    ret_ty: u8,
) {
    match s {
        Stmt::Asm(asm_text) => {
            out.push_str("  ");
            out.push_str(asm_text);
            out.push('\n');
        }
        Stmt::Return(Some(e)) => {
            gen_expr(out, e, slots, types, fn_ret, fn_params);
            emit_conv(out, expr_type(e, types, fn_ret), ret_ty);
            out.push_str("  leave\n  ret\n");
        }
        Stmt::Return(None) => {
            out.push_str("  mov eax, 0\n  leave\n  ret\n");
        }
        Stmt::Expr(e) => gen_expr(out, e, slots, types, fn_ret, fn_params),
        Stmt::If(c, then, els) => {
            unsafe {
                LABEL_ID += 1;
            }
            let id = unsafe { LABEL_ID };
            gen_expr(out, c, slots, types, fn_ret, fn_params);
            out.push_str("  cmp eax, 0\n");
            out.push_str(&format!("  je .Lelse{}\n", id));
            for s2 in then {
                gen_stmt(out, s2, slots, types, fn_ret, fn_params, ret_ty);
            }
            out.push_str(&format!("  jmp .Lend{}\n", id));
            out.push_str(&format!(".Lelse{}:\n", id));
            for s2 in els {
                gen_stmt(out, s2, slots, types, fn_ret, fn_params, ret_ty);
            }
            out.push_str(&format!(".Lend{}:\n", id));
        }
        Stmt::While(c, body) => {
            unsafe {
                LABEL_ID += 1;
            }
            let id = unsafe { LABEL_ID };
            unsafe {
                LOOP_STACK.push((id, false));
            }
            out.push_str(&format!(".Lbegin{}:\n", id));
            gen_expr(out, c, slots, types, fn_ret, fn_params);
            out.push_str("  cmp eax, 0\n");
            out.push_str(&format!("  je .Lend{}\n", id));
            for s2 in body {
                gen_stmt(out, s2, slots, types, fn_ret, fn_params, ret_ty);
            }
            out.push_str(&format!("  jmp .Lbegin{}\n", id));
            out.push_str(&format!(".Lend{}:\n", id));
            unsafe {
                LOOP_STACK.pop();
            }
        }
        Stmt::For(init, cond, step, body) => {
            if let Some(e) = init {
                gen_expr(out, e, slots, types, fn_ret, fn_params);
            }
            unsafe {
                LABEL_ID += 1;
            }
            let id = unsafe { LABEL_ID };
            unsafe {
                LOOP_STACK.push((id, true));
            }
            out.push_str(&format!(".LforBegin{}:\n", id));
            match cond {
                Some(c) => {
                    gen_expr(out, c, slots, types, fn_ret, fn_params);
                    out.push_str("  cmp eax, 0\n");
                    out.push_str(&format!("  je .Lend{}\n", id));
                }
                None => {}
            }
            for s2 in body {
                gen_stmt(out, s2, slots, types, fn_ret, fn_params, ret_ty);
            }
            out.push_str(&format!(".LforStep{}:\n", id));
            if let Some(e) = step {
                gen_expr(out, e, slots, types, fn_ret, fn_params);
            }
            out.push_str(&format!("  jmp .LforBegin{}\n", id));
            out.push_str(&format!(".Lend{}:\n", id));
            unsafe {
                LOOP_STACK.pop();
            }
        }
        Stmt::Decl(name, kind, init) => {
            gen_expr(out, init, slots, types, fn_ret, fn_params);
            emit_conv(out, expr_type(init, types, fn_ret), *kind);
            let slot = slots[name] * 8;
            out.push_str(&format!("  mov [rbp-{}], rax\n", slot));
        }
        Stmt::ArrDecl(name, len, kind) => {
            let slot = slots[name] * 8;
            let off = unsafe { ARR_CUR };
            let esz = if *kind == 3 { 8 } else if *kind == 1 { 1 } else { 4 };
            unsafe {
                ARR_CUR += len * esz;
            }
            out.push_str(&format!(
                "  lea rax, [rbp-{}]\n",
                slots.len() * 8 + off + len * esz
            ));
            out.push_str(&format!("  mov [rbp-{}], rax\n", slot));
        }
        Stmt::Break => {
            let id = unsafe { LOOP_STACK.last().map(|x| x.0).unwrap_or(0) };
            out.push_str(&format!("  jmp .Lend{}\n", id));
        }
        Stmt::Continue => {
            let last = unsafe { LOOP_STACK.last().cloned() };
            if let Some((id, is_for)) = last {
                if is_for {
                    out.push_str(&format!("  jmp .LforStep{}\n", id));
                } else {
                    out.push_str(&format!("  jmp .Lbegin{}\n", id));
                }
            }
        }
    }
}
