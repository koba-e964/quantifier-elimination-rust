use quantifier_elimination::{eliminate, parse_formula, Formula, Relation};
use std::io::Read;

fn main() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments
        .iter()
        .any(|argument| argument == "--help" || argument == "-h")
    {
        println!("usage: qe [FORMULA]\n       printf '%s' FORMULA | qe");
        return;
    }

    let input = if arguments.is_empty() {
        let mut input = String::new();
        if let Err(error) = std::io::stdin().read_to_string(&mut input) {
            eprintln!("qe: failed to read standard input: {error}");
            std::process::exit(2);
        }
        input
    } else {
        arguments.join(" ")
    };

    let formula = match parse_formula(&input) {
        Ok(formula) => formula,
        Err(error) => {
            eprintln!("qe: parse error {error}");
            std::process::exit(2);
        }
    };
    match eliminate(&formula) {
        Ok(result) => println!("{}", format_formula(&result)),
        Err(error) => {
            eprintln!("qe: elimination error: {error:?}");
            std::process::exit(1);
        }
    }
}

fn format_formula(formula: &Formula) -> String {
    match formula {
        Formula::True => "true".to_owned(),
        Formula::False => "false".to_owned(),
        Formula::Atom(atom) => {
            format!("{} {} 0", atom.polynomial, format_relation(atom.relation))
        }
        Formula::Not(body) => format!("!({})", format_formula(body)),
        Formula::And(formulas) => formulas
            .iter()
            .map(format_formula)
            .map(|formula| format!("({formula})"))
            .collect::<Vec<_>>()
            .join(" && "),
        Formula::Or(formulas) => formulas
            .iter()
            .map(format_formula)
            .map(|formula| format!("({formula})"))
            .collect::<Vec<_>>()
            .join(" || "),
        Formula::Quantified { .. } => "<quantified>".to_owned(),
    }
}

fn format_relation(relation: Relation) -> &'static str {
    match relation {
        Relation::Equal => "=",
        Relation::NotEqual => "!=",
        Relation::Less => "<",
        Relation::LessOrEqual => "<=",
        Relation::Greater => ">",
        Relation::GreaterOrEqual => ">=",
    }
}
