use quantifier_elimination::{
    eliminate_with_options, parse_formula, EliminationOptions, EliminationStats, Formula, Relation,
    SpecialHandlingConfig,
};
use std::io::Read;

fn main() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments
        .iter()
        .any(|argument| argument == "--help" || argument == "-h")
    {
        println!("usage: qe [--stats] [--special-handling=true|false] [--special-rules=PATH] [--variable-order=xN,xN,...] [FORMULA]\n       printf '%s' FORMULA | qe [--stats]");
        return;
    }

    let show_stats = arguments.iter().any(|argument| argument == "--stats");
    let special_handling = !arguments
        .iter()
        .any(|argument| argument == "--special-handling=false");
    let variable_order = match parse_variable_order(&arguments) {
        Ok(order) => order,
        Err(error) => {
            eprintln!("qe: {error}");
            std::process::exit(2);
        }
    };
    let special_rules = match parse_special_rules(&arguments) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("qe: {error}");
            std::process::exit(2);
        }
    };
    let arguments = arguments
        .into_iter()
        .filter(|argument| {
            argument != "--stats"
                && argument != "--special-handling=true"
                && argument != "--special-handling=false"
                && !argument.starts_with("--variable-order=")
                && !argument.starts_with("--special-rules=")
        })
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
    if let Err(error) = validate_variable_order(variable_order.as_ref(), &formula) {
        eprintln!("qe: {error}");
        std::process::exit(2);
    }
    match eliminate_with_options(
        &formula,
        EliminationOptions {
            special_handling,
            special_rules,
            variable_order,
        },
    ) {
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

fn parse_special_rules(arguments: &[String]) -> Result<SpecialHandlingConfig, String> {
    let Some(argument) = arguments
        .iter()
        .find(|argument| argument.starts_with("--special-rules="))
    else {
        return Ok(SpecialHandlingConfig::default());
    };
    let path = argument.trim_start_matches("--special-rules=");
    if path.is_empty() {
        return Err("--special-rules requires a file path".to_owned());
    }
    SpecialHandlingConfig::from_file(path)
}

fn parse_variable_order(arguments: &[String]) -> Result<Option<Vec<usize>>, String> {
    let Some(argument) = arguments
        .iter()
        .find(|argument| argument.starts_with("--variable-order="))
    else {
        return Ok(None);
    };
    let value = argument.trim_start_matches("--variable-order=");
    if value.is_empty() {
        return Err("--variable-order requires a comma-separated list".to_owned());
    }
    value
        .split(',')
        .map(|item| {
            item.strip_prefix('x')
                .unwrap_or(item)
                .parse::<usize>()
                .map_err(|_| format!("invalid variable in --variable-order: {item}"))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn validate_variable_order(order: Option<&Vec<usize>>, formula: &Formula) -> Result<(), String> {
    let Some(order) = order else {
        return Ok(());
    };
    let free_variables = formula.free_variables();
    let actual = order
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    if order.len() != free_variables.len() {
        return Err(format!(
            "--variable-order must list every free variable exactly once; expected {}, got {}",
            free_variables.len(),
            order.len()
        ));
    }
    if actual != free_variables {
        let expected = free_variables
            .iter()
            .map(|variable| format!("x{variable}"))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "--variable-order must contain exactly the free variables: {{{expected}}}"
        ));
    }
    Ok(())
}

fn format_stats(stats: &EliminationStats) -> String {
    let lifting_orders = if stats.lifting_orders.is_empty() {
        String::new()
    } else {
        let lifting_order_lines = stats
            .lifting_orders
            .iter()
            .enumerate()
            .map(|(index, order)| {
                format!(
                    "\n  lifting order {}: {}",
                    index,
                    order
                        .iter()
                        .map(|variable| format!("x{variable}"))
                        .collect::<Vec<_>>()
                        .join(" -> ")
                )
            })
            .collect::<Vec<_>>()
            .join("");
        format!(
            "\n  lifting order count: {}{}",
            stats.lifting_orders.len(),
            lifting_order_lines
        )
    };
    format!(
        "stats:\n  quantifier calls: {}\n  cells constructed: {}\n  leaf cells: {}\n  sector cells: {}\n  section cells: {}\n  projection levels: {}\n  projection polynomials: {}{}",
        stats.quantifier_calls,
        stats.cells_constructed,
        stats.leaf_cells,
        stats.sector_cells,
        stats.section_cells,
        stats.projection_levels,
        stats.projection_polynomials,
        lifting_orders,
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
