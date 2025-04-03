use lazy_static::lazy_static;
use rand::Rng;
use std::cmp::Ordering;
use std::io;

lazy_static! {
    static ref K: Number = generate_number();
}

#[derive(Debug, Clone)]
struct Number {
    is_negative: bool,
    digits: Vec<u8>,
}

impl Number {
    fn from_str(s: &str) -> Option<Self> {
        let s = s.trim();
        if s.is_empty() {
            return None;
        }
        let (is_negative, num_part) = match s.chars().next().unwrap() {
            '-' => (true, &s[1..]),
            '+' => (false, &s[1..]),
            _ => (false, s),
        };
        let num_part = num_part.trim_start_matches('0');
        if num_part.is_empty() {
            return Some(Number {
                is_negative: false,
                digits: vec![],
            });
        }
        let mut digits = Vec::with_capacity(num_part.len());
        for c in num_part.chars() {
            if let Some(d) = c.to_digit(10) {
                digits.push(d as u8);
            } else {
                return None;
            }
        }
        Some(Number {
            is_negative,
            digits,
        })
    }
}

fn generate_number() -> Number {
    let mut rng = rand::thread_rng();
    let m = rng.gen_range(0..=1_000_000);
    if m == 0 {
        return Number {
            is_negative: false,
            digits: vec![],
        };
    }
    let is_negative = rng.gen_bool(0.5);
    let mut digits = Vec::with_capacity(m);
    digits.push(rng.gen_range(1..=9) as u8);
    for _ in 1..m {
        digits.push(rng.gen_range(0..=9) as u8);
    }
    Number { is_negative, digits }
}

fn compare(a: &Number, b: &Number) -> Ordering {
    if a.digits.is_empty() && b.digits.is_empty() {
        return Ordering::Equal;
    }
    if a.is_negative && !b.is_negative {
        Ordering::Less
    } else if !a.is_negative && b.is_negative {
        Ordering::Greater
    } else {
        let a_len = a.digits.len();
        let b_len = b.digits.len();
        if a.is_negative {
            match a_len.cmp(&b_len) {
                Ordering::Greater => Ordering::Less,
                Ordering::Less => Ordering::Greater,
                Ordering::Equal => a.digits.iter().rev().cmp(b.digits.iter().rev()).reverse(),
            }
        } else {
            match a_len.cmp(&b_len) {
                Ordering::Greater => Ordering::Greater,
                Ordering::Less => Ordering::Less,
                Ordering::Equal => a.digits.cmp(&b.digits),
            }
        }
    }
}

fn check(n: &str) -> Result<(), String> {
    let n = Number::from_str(n).ok_or_else(|| "Error".to_string())?;
    match compare(&n, &K) {
        Ordering::Equal => Ok(()),
        cmp => {
            if rand::thread_rng().gen_ratio(1, 10000) {
                Err(if cmp == Ordering::Greater {
                    "Too Big".to_string()
                } else {
                    "Too Small".to_string()
                })
            } else {
                Err("NaN".to_string())
            }
        }
    }
}

fn main() {
    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        match check(&input) {
            Ok(()) => {
                println!("NumberFound");
                break;
            }
            Err(msg) => println!("{}", msg),
        }
    }
}