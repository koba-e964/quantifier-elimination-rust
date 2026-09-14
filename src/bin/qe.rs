use quantifier_elimination::{
    eliminate_with_stats, parse_formula, EliminationStats, Formula, Relation,
};
use std::io::Read;

fn main() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments
        .iter()
        .any(|argument| argument == "--help" || argument == "-h")
    {
        println!("usage: qe [--stats] [FORMULA]\n       printf '%s' FORMULA | qe [--stats]");
        return;
    }

    let show_stats = arguments.iter().any(|argument| argument == "--stats");
    let arguments = arguments
        .into_iter()
        .filter(|argument| argument != "--stats")
        .collect::<Vec<_>>();

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
    match eliminate_with_stats(&formula) {
        Ok((result, stats)) => {
            println!("{}", format_formula(&result));
            if show_stats {
                eprintln!("{}", format_stats(&stats));
            }
        }
        Err(error) => {
            eprintln!("qe: elimination error: {error:?}");
            std::process::exit(1);
        }
    }
}

fn format_stats(stats: &EliminationStats) -> String {
    format!(
        "stats:\n  quantifier calls: {}\n  cells constructed: {}\n  leaf cells: {}\n  sector cells: {}\n  section cells: {}\n  projection levels: {}\n  projection polynomials: {}",
        stats.quantifier_calls,
        stats.cells_constructed,
        stats.leaf_cells,
        stats.sector_cells,
        stats.section_cells,
        stats.projection_levels,
        stats.projection_polynomials,
    )
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
