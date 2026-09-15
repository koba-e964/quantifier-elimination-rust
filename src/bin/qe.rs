use quantifier_elimination::{
    eliminate_with_options, parse_formula_with_names, EliminationOptions, EliminationStats,
    Formula, ParsedFormula, Relation, SpecialHandlingConfig, VariableNames,
};
use std::io::Read;

fn main() {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if arguments
        .iter()
        .any(|argument| argument == "--help" || argument == "-h")
    {
        println!("usage: qe [--stats] [--max-cells=N] [--special-handling=true|false] [--special-rules=PATH] [--variable-order=xN,xN,...] [FORMULA]\n       printf '%s' FORMULA | qe [--stats]");
        return;
    }

    let show_stats = arguments.iter().any(|argument| argument == "--stats");
    let max_cells = match parse_max_cells(&arguments) {
        Ok(limit) => limit,
        Err(error) => {
            eprintln!("qe: {error}");
            std::process::exit(2);
        }
    };
    let special_handling = !arguments
        .iter()
        .any(|argument| argument == "--special-handling=false");
    let variable_order_names = match parse_variable_order(&arguments) {
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
                && !argument.starts_with("--max-cells=")
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

    let parsed = match parse_formula_with_names(&input) {
        Ok(parsed) => parsed,
        Err(error) => {
            eprintln!("qe: parse error {error}");
            std::process::exit(2);
        }
    };
    let variable_order = match resolve_variable_order(variable_order_names.as_ref(), &parsed) {
        Ok(order) => order,
        Err(error) => {
            eprintln!("qe: {error}");
            std::process::exit(2);
        }
    };
    if let Err(error) =
        validate_variable_order(variable_order.as_ref(), &parsed.formula, &parsed.names)
    {
        eprintln!("qe: {error}");
        std::process::exit(2);
    }
    match eliminate_with_options(
        &parsed.formula,
        EliminationOptions {
            special_handling,
            special_rules,
            variable_order,
            max_cells,
        },
    ) {
        Ok((result, stats)) => {
            println!("{}", format_formula(&result, &parsed.names));
            if show_stats {
                eprintln!("{}", format_stats(&stats, &parsed.names));
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

fn parse_max_cells(arguments: &[String]) -> Result<Option<usize>, String> {
    let Some(argument) = arguments
        .iter()
        .find(|argument| argument.starts_with("--max-cells="))
    else {
        return Ok(Some(100));
    };
    let value = argument.trim_start_matches("--max-cells=");
    let limit = value
        .parse::<usize>()
        .map_err(|_| "--max-cells requires a positive integer".to_owned())?;
    if limit == 0 {
        return Err("--max-cells requires a positive integer".to_owned());
    }
    Ok(Some(limit))
}

fn parse_variable_order(arguments: &[String]) -> Result<Option<Vec<String>>, String> {
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
            if item.is_empty() {
                Err("invalid empty variable in --variable-order".to_owned())
            } else {
                Ok(item.to_owned())
            }
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn resolve_variable_order(
    order: Option<&Vec<String>>,
    parsed: &ParsedFormula,
) -> Result<Option<Vec<usize>>, String> {
    let Some(order) = order else {
        return Ok(None);
    };
    let free_variables = parsed.formula.free_variables();
    let mut resolved = Vec::with_capacity(order.len());
    for name in order {
        let matches = free_variables
            .iter()
            .copied()
            .filter(|variable| parsed.names.name(*variable) == *name)
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [variable] => resolved.push(*variable),
            [] => {
                let expected = free_variables
                    .iter()
                    .map(|variable| parsed.names.name(*variable))
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(format!(
                    "--variable-order must contain exactly the free variables: {{{expected}}}; unknown or bound variable: {name}"
                ));
            }
            _ => return Err(format!("ambiguous variable in --variable-order: {name}")),
        }
    }
    Ok(Some(resolved))
}

fn validate_variable_order(
    order: Option<&Vec<usize>>,
    formula: &Formula,
    names: &VariableNames,
) -> Result<(), String> {
    let Some(order) = order else {
        return Ok(());
    };
    let free_variables = formula.free_variables();
    let actual = order
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    if order.len() != free_variables.len() {
        let expected = free_variables
            .iter()
            .map(|variable| names.name(*variable))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "--variable-order must list every free variable exactly once; expected {{{expected}}}, got {} entries",
            order.len()
        ));
    }
    if actual.len() != order.len() {
        let duplicate = order
            .iter()
            .find(|variable| {
                order
                    .iter()
                    .filter(|candidate| candidate == variable)
                    .count()
                    > 1
            })
            .expect("duplicate order entry exists");
        return Err(format!(
            "--variable-order contains duplicate variable: {}",
            names.name(*duplicate)
        ));
    }
    if actual != free_variables {
        let expected = free_variables
            .iter()
            .map(|variable| names.name(*variable))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "--variable-order must contain exactly the free variables: {{{expected}}}"
        ));
    }
    Ok(())
}

fn format_stats(stats: &EliminationStats, names: &VariableNames) -> String {
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
                        .map(|variable| names.name(*variable))
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

fn format_formula(formula: &Formula, names: &VariableNames) -> String {
    match formula {
        Formula::True => "true".to_owned(),
        Formula::False => "false".to_owned(),
        Formula::Atom(atom) => {
            format!(
                "{} {} 0",
                atom.polynomial.to_string_with(names),
                format_relation(atom.relation)
            )
        }
        Formula::Not(body) => format!("!({})", format_formula(body, names)),
        Formula::And(formulas) => formulas
            .iter()
            .map(|formula| format_formula(formula, names))
            .map(|formula| format!("({formula})"))
            .collect::<Vec<_>>()
            .join(" && "),
        Formula::Or(formulas) => formulas
            .iter()
            .map(|formula| format_formula(formula, names))
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
