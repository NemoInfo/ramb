use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::Rc;
use std::io::{self, Write};

use Token::*;
use Expression::*;
use print_flags::*;

pub enum Expression {
  Var(Rc<String>),
  Fun { arg: Rc<String>, body: Rc<Expression> },
  App { lhs: Rc<Expression>, rhs: Rc<Expression> },
}

pub fn var<S: Into<String>>(arg: S) -> Rc<Expression> {
  _var(&Rc::new(arg.into()))
}

pub fn fun<S: Into<String>>(arg: S, body: &Rc<Expression>) -> Rc<Expression> {
  _fun(&Rc::new(arg.into()), body)
}

pub fn app(lhs: &Rc<Expression>, rhs: &Rc<Expression>) -> Rc<Expression> {
  Rc::new(App {lhs: Rc::clone(lhs), rhs: Rc::clone(rhs)})
}

pub fn _var(arg: &Rc<String>) -> Rc<Expression> {
  Rc::new(Var(arg.clone()))
}

pub fn _fun(arg: &Rc<String>, body: &Rc<Expression>) -> Rc<Expression> {
  Rc::new(Fun {arg: arg.clone(), body: capture(arg, body)})
}

fn capture(pat: &Rc<String>, expr: &Rc<Expression>) -> Rc<Expression> {
  match expr.as_ref() {
    Var(var)       => if pat == var { _var(pat) } else { Rc::clone(expr) },
    Fun{arg, body} => if pat == arg { Rc::clone(expr) } 
                      // TODO: Maybe you do have to call _fun again? to basically capture arg on
                      // body as well but I am not sure?
                      else { Rc::new(Fun {arg: Rc::clone(&arg), body: capture(pat, body)}) },
    App{lhs, rhs}  => app(&capture(pat, lhs), &capture(pat, rhs))
  }
}

fn apply_binding(pat: &Rc<String>, expr: &Rc<Expression>, val: &Rc<Expression>) -> Rc<Expression> {
  match expr.as_ref() {
    Var(var)       => if var == pat { Rc::clone(val) } else { Rc::clone(expr) }
    Fun{arg, body} => if arg == pat { Rc::clone(body) } 
                      else { Rc::new(Fun {arg: Rc::clone(arg), body: apply(pat, body, val)}) }
    App{lhs, rhs}  => app(&apply_binding(pat, lhs, val), &apply_binding(pat, rhs, val))
  }
}

fn apply(pat: &Rc<String>, expr: &Rc<Expression>, val: &Rc<Expression>) -> Rc<Expression> {
  match expr.as_ref() {
    Var(var)       => if Rc::ptr_eq(var, pat) { Rc::clone(val) } else { Rc::clone(expr) }
    Fun{arg, body} => if Rc::ptr_eq(arg, pat) { Rc::clone(body) } 
                      else { Rc::new(Fun {arg: Rc::clone(arg), body: apply(pat, body, val)}) }
    App{lhs, rhs}  => app(&apply(pat, lhs, val), &apply(pat, rhs, val))
  }
}

fn beta1(expr: &Rc<Expression>) -> Rc<Expression> {
  match expr.as_ref() {
    Var(_) => Rc::clone(expr),
    Fun{arg, body} => {
      let beta_body = beta1(body);
      if Rc::ptr_eq(body, &beta_body) { return Rc::clone(expr) }
      Rc::new(Fun {arg: Rc::clone(arg), body: beta_body})
    }
    App{lhs,rhs} => {
      let beta_lhs = beta1(lhs);
      if !Rc::ptr_eq(lhs, &beta_lhs)        { return app(&beta_lhs, rhs)    }
      if let Fun {arg, body} = lhs.as_ref() { return apply(arg, body, rhs); }
      let beta_rhs = beta1(rhs);
      if !Rc::ptr_eq(rhs, &beta_rhs)        { return app(lhs, &beta_rhs)    }
      Rc::clone(expr)
    }
  }
}

fn beta(e: &Rc<Expression>) -> Rc<Expression> {
  let mut e = Rc::clone(e);
  let mut e1 = beta1(&e);
  while !Rc::ptr_eq(&e1, &e) {
    println!("    =ᵦ {e1}");
    e = e1; e1 = beta1(&e);
  }
  e1
}

impl Expression {
  pub fn display(&self, a: PrintArguments) -> String {
    let mut s = String::new();
    match self {
      Var(arg) => if a.v > 0 { format!("{arg}_{:x}", arg.as_ptr() as usize & 0xFFF) } else { format!("{arg}") }
      Fun { arg, body } => {
        if !a.f.any_set(FUN) { s += "𝞴" }
        s += arg;
        if a.v > 0 { s += &format!("_{:x}", arg.as_ptr() as usize & 0xFFF) }
        if !matches!(body.as_ref(), Fun { .. }) { s += "." }
        s = s + " " + &body.display(PrintArguments { f: FUN, ..a });
        if a.v >= 2 || a.f.any_set(APP_LHS | APP_RHS) { format!("({s})") } else { s }
      }
      App { lhs, rhs } => {
        s += &format!("{} {}", lhs.display(PrintArguments { f: APP_LHS, ..a }), 
                               rhs.display(PrintArguments { f: APP_RHS, ..a }));
        if a.v >= 2 || a.f.any_set(APP_RHS) { format!("({s})") } else { s }
      }
    }
  }
}

impl std::fmt::Display for Expression {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.display(PrintArguments { v: 0, f: TOP }))
  }
}

impl std::fmt::Debug for Expression {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "{}", self.display(PrintArguments { v: 2, f: TOP }))
  }
}

type FatToken<'a> =(&'a str, Token<'a>);

fn next_token(mut content: &str) -> (&str, Result<Token<'_>, ()>) {
  use Token::*;
  content = content.trim_start();
  let Some(c) = content.chars().next() else { return (content, Ok(End)); };
  match c {
    '('        => return (&content[1..],            Ok(OParen)),
    ')'        => return (&content[1..],            Ok(CParen)),
    '.'        => return (&content[1..],            Ok(Dot)),
    '?'        => return (&content[1..],            Ok(Print)),
    ':'        => if let Some(':') = content[1..].chars().next() { (&content[2..], Ok(Assign)) } 
                  else { (content, Err(())) }
    '\\' | '𝞴' => return (&content[c.len_utf8()..], Ok(Lambda)),
    c if c.is_alphanumeric() || c == '_' => {
      let end = content.chars().take_while(|&c| c.is_alphanumeric() || c == '_')
                               .fold(0, |acc, c| acc + c.len_utf8());
      let end = content[end..].chars().take_while(|&c| c == '\'')
                               .fold(end, |acc, c| acc + c.len_utf8());
      (&content[end..], Ok(Ident(&content[..end])))
    }
    _ => (content, Err(()))
  }
}

fn tokenize(mut content: &str) -> Result<Vec<(&str, Token<'_>)>, &str> {
  let mut tokens = vec![];
  loop { 
    let (new_content, res) = next_token(content);
    content = new_content;
    match res {
      Ok(token) => {
        tokens.push((content, token));
        if let Token::End = token { return Ok(tokens); }
      }
      Err(()) => return Err(content),
    }
  }
}

#[derive(Debug, Clone, Copy)]
pub enum Token<'a> {
  End,
  OParen,
  CParen,
  Lambda,
  Dot,
  Assign,
  Print,
  Ident(&'a str)
}

impl<'a> Token<'a> {
  fn kind(&self) -> std::mem::Discriminant<Self> {
    std::mem::discriminant(self)
  }
}

#[derive(Debug)]
pub enum ParseError<'a> {
  UnexpectedToken {
    wanted: Token<'a>,
    found: &'a Token<'a>,
    content: &'a str,
  },
  OutOfTokens {
    wanted: Token<'a>,
  }
}
use ParseError::*;

fn best_error<'a>(err1: ParseError<'a>, err2: ParseError<'a>) -> ParseError<'a> {
  use ParseError::*;
  match (&err1, &err2) {
    (OutOfTokens { .. }, _) => err1,
    (_, OutOfTokens { .. }) => err2,
    (UnexpectedToken { content: c1, .. }, UnexpectedToken { content: c2, .. }) => {
      if c1.as_ptr() >= c2.as_ptr() { err1 } else { err2 }
    }
  }
}

// GRAMMAR
//
// root  → var '=' expr
//       | expr
//
// expr  →  '\' var+ '.' expr
//       |  app
// 
// app   →  app atom
//       |  atom
// 
// atom  →  var
//       |  '(' expr ')'

fn parse_token<'a>(ts: &'a [FatToken<'a>], wanted: Token<'a>) -> Result<(&'a [FatToken<'a>], Token<'a>), ParseError<'a>> {
  match ts {
    [(content, found), rest @ ..] => 
      if found.kind() == wanted.kind() {Ok((rest, *found))} 
      else { Err(UnexpectedToken { wanted, found, content }) },
    [] => Err(OutOfTokens { wanted })
  }
}

fn parse_ident<'a>(ts: &'a [FatToken<'a>]) -> Result<(&'a [FatToken<'a>], &'a str), ParseError<'a>> {
  match ts {
    [(_, Ident(var)), rest @ ..] => { Ok((rest, *var)) } 
    [(content, found), ..] => { Err(UnexpectedToken { wanted: Ident(""), found, content }) },
    [] => Err(OutOfTokens { wanted: Ident("") })
  }
}

fn parse_var<'a>(ts: &'a [FatToken<'a>]) -> Result<(&'a [FatToken<'a>], Rc<Expression>), ParseError<'a>> {
  let (ts, v) = parse_ident(ts)?;
  return Ok((ts, var(v)));
}

fn parse_fun_after_lambda<'a>(ts: &'a [FatToken<'a>]) -> Result<(&'a [FatToken<'a>], Rc<Expression>), ParseError<'a>> {
  let (ts, arg) = parse_ident(ts)?;
  let (ts, body) = match parse_token(ts, Dot) {
    Ok((ts, _)) => parse_expr(ts),
    Err(_) => parse_fun_after_lambda(ts),
  }?;
  return Ok((ts, fun(arg, &body)))
}

fn parse_fun<'a>(ts: &'a [FatToken<'a>]) -> Result<(&'a [FatToken<'a>], Rc<Expression>), ParseError<'a>> {
  let (ts, _) = parse_token(ts, Lambda)?;
  parse_fun_after_lambda(ts)
}


fn parse_atom<'a>(ts: &'a [FatToken<'a>]) -> Result<(&'a [FatToken<'a>], Rc<Expression>), ParseError<'a>> {
  parse_var(ts).or_else(|err1| {
    let (ts, _) = parse_token(ts, OParen)?;
    let (ts, expr) = parse_expr(ts).map_err(|err2| best_error(err1, err2))?;
    let (ts, _) = parse_token(ts, CParen)?;
    Ok((ts, expr))
  })
}

fn parse_app<'a>(ts: &'a [FatToken<'a>]) -> Result<(&'a [FatToken<'a>], Rc<Expression>), ParseError<'a>> {
  let (mut ts, mut lhs) = parse_atom(ts)?;
  while let Ok((new_ts, rhs)) = parse_atom(ts) {
    ts = new_ts;
    lhs = app(&lhs, &rhs);
  }
  Ok((ts, lhs))
}

fn parse_expr<'a>(ts: &'a [FatToken<'a>]) -> Result<(&'a [FatToken<'a>], Rc<Expression>), ParseError<'a>> {
  parse_fun(ts).or_else(|err1|
    parse_app(ts).map_err(|err2| best_error(err1, err2)))
}

fn parse_assign<'a>(ts: &'a [FatToken<'a>]) -> Result<(&'a [FatToken<'a>], Rc<Expression>), ParseError<'a>> {
  let (ts, bind) = parse_ident(ts)?;
  let (ts, _)    = parse_token(ts, Assign)?;
  let (ts, expr) = parse_expr(ts)?;
  Ok((ts, fun(bind, &expr)))
}

fn parse_statement<'a>(ts: &'a [FatToken<'a>], bindings: &mut HashMap<Rc<String>, Rc<Expression>>) -> Result<(&'a [FatToken<'a>], Rc<Expression>), ParseError<'a>> {
  match parse_assign(ts) {
    Ok((ts, expr)) => {
      let Fun { arg, body } = expr.as_ref() else { unreachable!() };
      let mut body = body.clone();
      let mut new_body;
      for (pat, val) in bindings.iter() {
        new_body = apply_binding(pat, &body, val);
        body = new_body.clone();
      }
      bindings.insert(Rc::clone(&arg), Rc::clone(&body));
      Ok((ts, Rc::clone(&body)))
    }
    Err(err1) => { 
      let (ts, mut expr) = parse_expr(ts).map_err(|err2| best_error(err1, err2))?; 
      let Ok((ts, _)) = parse_token(ts, Print) else { return Ok((ts, expr)) };
      for (pat, val) in bindings.iter() {
        expr = apply_binding(pat, &expr, val);        
      }                                               
      println!("{expr}");
      beta(&expr);
      Ok((ts, expr))
    },
  }
}

fn main() -> std::io::Result<()> {
  let mut input = String::new();
  let mut bindings = HashMap::new();
  let args = std::env::args().collect::<Vec<_>>();
  if args.len() == 2 {
    let file = &args[1];
    let input = std::fs::read_to_string(file)?;
    for line in input.lines() {
      if let Err(e) = process_input(line, &mut bindings) { print_error(&e) }
    }
    Ok(())
  } else {
  loop {
    print!("𝞴> ");
    io::stdout().flush()?;
    input.clear();
    io::stdin().read_line(&mut input)?;

    if let Err(e) = process_input(&input, &mut bindings) { print_error(&e) }
  }
  }
}

fn process_input(input: &str, bindings: &mut HashMap<Rc<String>, Rc<Expression>>) -> Result<(), String> {
  let tokens = tokenize(input).map_err(|rest| format!("Unexpected character '{}'", 
                                               rest.chars().next().unwrap_or(' ')))?;
  if matches!(tokens.as_slice(), [(_, Token::End)]) { return Ok(()); }
  let (ts, mut expr) = parse_statement(&tokens, bindings).map_err(|err| format_parse_error(&err, input))?;
  let [(_, Token::End)] = ts else { 
    let (content, _) = ts.first().ok_or("No END token found".to_string())?;
    let (row, col) = get_line_col(input, content).ok_or("Could not get row:col of error".to_string())?;
    return Err(format!("Trailing characters at {row}:{col}: \"{}\"", 
                       content.replace("\n", "\\n").chars().take(20).collect::<String>(),
                       ));
  };
  for (pat, val) in bindings.iter() {
    expr = apply_binding(pat, &expr, val);
  }
  Ok(())
}

fn format_parse_error(err: &ParseError, input: &str) -> String {
  match err {
    UnexpectedToken { wanted, found, content } => {
      let wanted_str = format!("{wanted:?}")
        .strip_suffix("(\"\")")
        .unwrap_or(&format!("{wanted:?}"))
        .to_string();
      let (line, col) = get_line_col(input, content).unwrap();
      format!("Unexpected token, wanted {wanted_str} found {found:?} at ({line}:{col})")
    }
    OutOfTokens { wanted } => format!("Out of tokens, wanted {wanted:?}")
  }
}

fn print_error(msg: &str) { eprintln!("\x1b[1;91merror:\x1b[0m {msg}"); }

fn get_line_col(text: &str, substring: &str) -> Option<(usize, usize)> {
  let offset = substring.as_ptr() as usize - text.as_ptr() as usize;
  let mut row = 1;
  let mut col = 0;
  for (i, c) in text.char_indices() {
    if i == offset { return Some((row, col-1)) }
    if c == '\n' || c == '\r' {
      row += 1;
      col = 0;
    } 
    col += 1;
  }
  None
}

pub struct PrintArguments {
  v: u8,
  f: PrintFlags,
}

mod print_flags {
  #[derive(Clone, Copy)]
  pub struct PrintFlags(pub u8);
  pub const TOP     : PrintFlags = PrintFlags(1 << 0);
  pub const APP_LHS : PrintFlags = PrintFlags(1 << 1);
  pub const APP_RHS : PrintFlags = PrintFlags(1 << 2);
  pub const FUN     : PrintFlags = PrintFlags(1 << 3);
  
  impl PrintFlags {
    pub fn any_set(self, rhs: Self) -> bool {
      self.0 & rhs.0 != 0
    }
  }
  
  impl std::ops::BitOr for PrintFlags {
    type Output = PrintFlags;
    fn bitor(self, rhs: Self) -> Self::Output {
      PrintFlags(self.0 | rhs.0)
    }
  }
}
