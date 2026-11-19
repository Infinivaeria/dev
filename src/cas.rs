use nalgebra::{DMatrix, DVector};
use serde::{Deserialize, Serialize};
use std::{cell::Cell, fs, path::Path, sync::Mutex};
use symbolica::{
    atom::{Atom, AtomCore, Indeterminate},
    domains::{integer::Z, rational::Q, rational_polynomial::RationalPolynomial},
    parser::ParseSettings,
    poly::series::SeriesDepth,
    symbol,
};

const CAS_NAMESPACE: &str = "selenite::cas";
static SYMBOLICA_LOCK: Mutex<()> = Mutex::new(());

thread_local! {
    static SYMBOLICA_DEPTH: Cell<usize> = const { Cell::new(0) };
}

fn with_symbolica<R: Send>(operation: impl FnOnce() -> R + Send) -> R {
    struct ReentrantDepth;
    impl Drop for ReentrantDepth {
        fn drop(&mut self) {
            SYMBOLICA_DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
        }
    }

    if SYMBOLICA_DEPTH.with(Cell::get) != 0 {
        return operation();
    }
    let _guard = SYMBOLICA_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    std::thread::scope(|scope| {
        scope
            .spawn(move || {
                SYMBOLICA_DEPTH.with(|depth| depth.set(1));
                let _depth = ReentrantDepth;
                operation()
            })
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
    })
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CasResult {
    pub operation: String,
    pub input: String,
    pub output: String,
    pub steps: Vec<String>,
    pub error: Option<String>,
}

impl CasResult {
    fn ok(operation: &str, input: &str, output: String, steps: Vec<String>) -> Self {
        Self {
            operation: operation.to_owned(),
            input: input.to_owned(),
            output,
            steps,
            error: None,
        }
    }

    fn error(operation: &str, input: &str, error: impl ToString) -> Self {
        Self {
            operation: operation.to_owned(),
            input: input.to_owned(),
            output: String::new(),
            steps: Vec::new(),
            error: Some(error.to_string()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NotebookCell {
    pub id: String,
    pub input: String,
    pub result: CasResult,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Notebook {
    pub cells: Vec<NotebookCell>,
}

impl Notebook {
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("cannot serialize notebook: {e}"))?;
        fs::write(path, json).map_err(|e| format!("cannot save notebook: {e}"))
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let json = fs::read_to_string(path).map_err(|e| format!("cannot load notebook: {e}"))?;
        serde_json::from_str(&json).map_err(|e| format!("cannot parse notebook: {e}"))
    }
}

pub fn parse_expression(input: &str) -> Result<Atom, String> {
    with_symbolica(|| {
        Atom::parse(input.trim(), CAS_NAMESPACE, ParseSettings::symbolica())
            .map_err(|e| format!("cannot parse expression `{input}`: {e}"))
    })
}

fn variable(input: &str) -> Result<Atom, String> {
    let name = input.trim();
    if name.is_empty()
        || name.chars().enumerate().any(|(index, c)| {
            if index == 0 {
                !(c == '_' || c.is_ascii_alphabetic())
            } else {
                !(c == '_' || c.is_ascii_alphanumeric())
            }
        })
    {
        return Err(format!("invalid variable `{input}`"));
    }
    parse_expression(name)
}

pub fn simplify(input: &str) -> Result<Atom, String> {
    with_symbolica(|| Ok(parse_expression(input)?.cancel().factor()))
}

pub fn expand_expression(input: &str) -> Result<Atom, String> {
    with_symbolica(|| Ok(parse_expression(input)?.expand()))
}

pub fn factor_expression(input: &str) -> Result<Atom, String> {
    with_symbolica(|| Ok(parse_expression(input)?.factor()))
}

pub fn together_expression(input: &str) -> Result<Atom, String> {
    with_symbolica(|| Ok(parse_expression(input)?.together()))
}

pub fn cancel_expression(input: &str) -> Result<Atom, String> {
    with_symbolica(|| Ok(parse_expression(input)?.cancel()))
}

pub fn apart_expression(input: &str, wrt: &str) -> Result<Atom, String> {
    with_symbolica(|| {
        let atom = parse_expression(input)?;
        let x = variable(wrt)?;
        let indeterminate: Indeterminate = x
            .clone()
            .try_into()
            .map_err(|_| format!("`{wrt}` is not a symbolic variable"))?;
        Ok(atom.apart(indeterminate))
    })
}

pub fn differentiate(input: &str, wrt: &str) -> Result<Atom, String> {
    with_symbolica(|| {
        let atom = parse_expression(input)?;
        let x = variable(wrt)?;
        let indeterminate: Indeterminate = x
            .try_into()
            .map_err(|_| format!("`{wrt}` is not a symbolic variable"))?;
        Ok(atom.derivative(indeterminate))
    })
}

pub fn integrate(input: &str, wrt: &str) -> Result<Atom, String> {
    with_symbolica(|| {
        let atom = parse_expression(input)?;
        let x = variable(wrt)?;
        let variables = [x.clone()];
        let rational: RationalPolynomial<_, u32> = atom
            .try_to_rational_polynomial(&Q, &Z, &variables)
            .map_err(|e| {
                format!(
                    "symbolic integration is unavailable for this expression in this Symbolica \
                         build: {e}; no external integration backend or license key is configured"
                )
            })?;
        if rational.get_variables().len() > variables.len() {
            return Err(
                "symbolic integration is unavailable for non-rational expressions in this \
                 Symbolica build; no external integration backend or license key is configured"
                    .to_owned(),
            );
        }
        let integral = rational.integrate(0);
        if !integral.logarithmic_parts.is_empty()
            && integral
                .logarithmic_parts
                .iter()
                .any(|term| term.coefficient.get_variables().len() > variables.len())
        {
            return Err(
                "symbolic integration produced an algebraic root sum, which this public CAS \
                 interface cannot represent; no external Symbolica integration backend is linked"
                    .to_owned(),
            );
        }

        let mut result = Atom::num(0);
        for part in integral.rational_parts {
            result = result + part.to_expression();
        }
        for part in integral.logarithmic_parts {
            let log_term = symbolica::function!(symbol!("log"), part.argument.to_expression());
            result = result + part.coefficient.to_expression() * log_term;
        }
        Ok(result.cancel())
    })
}

pub fn taylor_series(input: &str, wrt: &str, point: &str, order: u32) -> Result<Atom, String> {
    with_symbolica(|| {
        let atom = parse_expression(input)?;
        let x = variable(wrt)?;
        let indeterminate: Indeterminate = x
            .try_into()
            .map_err(|_| format!("`{wrt}` is not a symbolic variable"))?;
        let point = parse_expression(point)?;
        atom.series(
            indeterminate,
            point,
            SeriesDepth::absolute(i64::from(order)),
        )
        .map(|series| series.to_atom())
        .map_err(|e| format!("cannot compute series: {e}"))
    })
}

fn equation_to_atom(input: &str) -> Result<Atom, String> {
    let equation = input.trim();
    if let Some(index) = find_top_level(equation, '=') {
        let lhs = parse_expression(&equation[..index])?;
        let rhs = parse_expression(&equation[index + 1..])?;
        Ok(lhs - rhs)
    } else {
        parse_expression(equation)
    }
}

pub fn solve_equations(equations: &str, unknowns: &str) -> Result<String, String> {
    with_symbolica(|| {
        let equations = split_top_level(equations, ',')
            .into_iter()
            .map(equation_to_atom)
            .collect::<Result<Vec<_>, _>>()?;
        let variables = split_top_level(unknowns, ',')
            .into_iter()
            .map(variable)
            .collect::<Result<Vec<_>, _>>()?;
        if equations.is_empty() || variables.is_empty() {
            return Err("solve requires at least one equation and one variable".into());
        }
        let solutions = Atom::solve(&equations)
            .wrt(&variables)
            .map_err(|e| format!("cannot solve system: {e}"))?;
        if solutions.len() == 0 {
            return Ok("[]".to_owned());
        }
        Ok(format!(
            "[{}]",
            solutions
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ))
    })
}

pub fn evaluate_at(input: &str, substitutions: &str) -> Result<Atom, String> {
    with_symbolica(|| {
        let mut atom = parse_expression(input)?;
        for assignment in split_top_level(substitutions, ',') {
            let Some(index) = find_top_level(assignment, '=') else {
                return Err(format!(
                    "invalid substitution `{assignment}`, expected VAR=NUM"
                ));
            };
            let name = assignment[..index].trim();
            let raw_value = assignment[index + 1..].trim();
            let numeric = raw_value
                .parse::<f64>()
                .map_err(|_| format!("substitution for `{name}` is not a number: `{raw_value}`"))?;
            if !numeric.is_finite() {
                return Err(format!("substitution for `{name}` must be finite"));
            }
            atom = atom
                .replace(variable(name)?)
                .with(parse_expression(raw_value)?);
        }
        Ok(atom.cancel())
    })
}

pub fn backend_capabilities() -> &'static [&'static str] {
    &[
        "Symbolica exact parsing and algebraic transformations",
        "Symbolica exact differentiation, rational/polynomial integration, series and solving",
        "No external Symbolica integration extension or license key is configured",
        "nalgebra numerical linear algebra",
    ]
}

fn find_top_level(input: &str, needle: char) -> Option<usize> {
    let mut round = 0i32;
    let mut square = 0i32;
    let mut curly = 0i32;
    for (index, c) in input.char_indices() {
        match c {
            '(' => round += 1,
            ')' => round -= 1,
            '[' => square += 1,
            ']' => square -= 1,
            '{' => curly += 1,
            '}' => curly -= 1,
            found if found == needle && round == 0 && square == 0 && curly == 0 => {
                return Some(index);
            }
            _ => {}
        }
    }
    None
}

fn split_top_level(input: &str, separator: char) -> Vec<&str> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut round = 0i32;
    let mut square = 0i32;
    let mut curly = 0i32;
    for (index, c) in input.char_indices() {
        match c {
            '(' => round += 1,
            ')' => round -= 1,
            '[' => square += 1,
            ']' => square -= 1,
            '{' => curly += 1,
            '}' => curly -= 1,
            found if found == separator && round == 0 && square == 0 && curly == 0 => {
                result.push(input[start..index].trim());
                start = index + c.len_utf8();
            }
            _ => {}
        }
    }
    let tail = input[start..].trim();
    if !tail.is_empty() {
        result.push(tail);
    }
    result
}

fn split_semicolon<'a>(input: &'a str, expected: &str) -> Result<Vec<&'a str>, String> {
    let parts = split_top_level(input, ';');
    if parts.len() != expected.matches(';').count() + 1 || parts.iter().any(|p| p.is_empty()) {
        return Err(format!("invalid arguments; expected {expected}"));
    }
    Ok(parts)
}

pub fn execute_command(command: &str) -> CasResult {
    with_symbolica(|| execute_command_unlocked(command))
}

fn execute_command_unlocked(command: &str) -> CasResult {
    let input = command.trim();
    let mut parts = input.splitn(2, char::is_whitespace);
    let operation = parts.next().unwrap_or("").trim().to_ascii_lowercase();
    let arguments = parts.next().unwrap_or("").trim();
    let run = |output: Result<String, String>| match output {
        Ok(output) => CasResult::ok(&operation, arguments, output, vec!["parsed command".into()]),
        Err(error) => CasResult::error(&operation, arguments, error),
    };

    match operation.as_str() {
        "eval" | "simplify" => run(simplify(arguments).map(|a| a.to_string())),
        "expand" => run(expand_expression(arguments).map(|a| a.to_string())),
        "factor" => run(factor_expression(arguments).map(|a| a.to_string())),
        "together" => run(together_expression(arguments).map(|a| a.to_string())),
        "cancel" => run(cancel_expression(arguments).map(|a| a.to_string())),
        "diff" => run(split_semicolon(arguments, "EXPR; VAR")
            .and_then(|parts| differentiate(parts[0], parts[1]).map(|a| a.to_string()))),
        "integrate" => run(split_semicolon(arguments, "EXPR; VAR")
            .and_then(|parts| integrate(parts[0], parts[1]).map(|a| a.to_string()))),
        "series" => run(
            split_semicolon(arguments, "EXPR; VAR; POINT; ORDER").and_then(|parts| {
                let order = parts[3]
                    .parse::<u32>()
                    .map_err(|_| format!("invalid series order `{}`", parts[3]))?;
                taylor_series(parts[0], parts[1], parts[2], order).map(|a| a.to_string())
            }),
        ),
        "solve" => run(split_semicolon(arguments, "EQ1,EQ2; VARS")
            .and_then(|parts| solve_equations(parts[0], parts[1]))),
        "apart" => run(if find_top_level(arguments, ';').is_some() {
            split_semicolon(arguments, "EXPR; VAR")
                .and_then(|parts| apart_expression(parts[0], parts[1]).map(|a| a.to_string()))
        } else {
            let atom = parse_expression(arguments);
            atom.and_then(|atom| {
                let variables = atom.get_all_indeterminates(false);
                let indeterminates = variables
                    .iter()
                    .map(|view| {
                        Indeterminate::try_from(view.to_owned())
                            .map_err(|_| "apart expression contains a non-symbol".to_owned())
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(atom.apart_multivariate(&indeterminates).to_string())
            })
        }),
        "eval-at" => run(split_semicolon(arguments, "EXPR; VAR=NUM")
            .and_then(|parts| evaluate_at(parts[0], parts[1]).map(|a| a.to_string()))),
        "determinant" => run(parse_matrix(arguments)
            .and_then(matrix_determinant)
            .map(|x| x.to_string())),
        "inverse" => run(parse_matrix(arguments)
            .and_then(matrix_inverse)
            .and_then(matrix_json)),
        "matrix-solve" => run(
            split_semicolon(arguments, "MATRIX; VECTOR").and_then(|parts| {
                let matrix = parse_matrix(parts[0])?;
                let vector = parse_vector(parts[1])?;
                matrix_solve(&matrix, &vector)
                    .map(|v| serde_json::to_string(&v.iter().copied().collect::<Vec<_>>()).unwrap())
            }),
        ),
        "rank" => run(parse_matrix(arguments).map(|m| matrix_rank(&m).to_string())),
        "eigenvalues" => run(parse_matrix(arguments)
            .and_then(matrix_eigenvalues)
            .and_then(|values| {
                serde_json::to_string(&values)
                    .map_err(|e| format!("cannot encode eigenvalues: {e}"))
            })),
        "" => CasResult::error("", "", "empty command"),
        _ => CasResult::error(
            &operation,
            arguments,
            format!("unknown CAS operation `{operation}`"),
        ),
    }
}

pub fn parse_matrix(input: &str) -> Result<DMatrix<f64>, String> {
    let rows: Vec<Vec<f64>> =
        serde_json::from_str(input.trim()).map_err(|e| format!("invalid matrix JSON: {e}"))?;
    if rows.is_empty() || rows[0].is_empty() {
        return Err("matrix must not be empty".into());
    }
    let columns = rows[0].len();
    if rows.iter().any(|row| row.len() != columns) {
        return Err("matrix rows must all have the same length".into());
    }
    if rows.iter().flatten().any(|value| !value.is_finite()) {
        return Err("matrix entries must be finite".into());
    }
    Ok(DMatrix::from_row_slice(
        rows.len(),
        columns,
        &rows.into_iter().flatten().collect::<Vec<_>>(),
    ))
}

pub fn parse_vector(input: &str) -> Result<DVector<f64>, String> {
    let values: Vec<f64> =
        serde_json::from_str(input.trim()).map_err(|e| format!("invalid vector JSON: {e}"))?;
    if values.is_empty() {
        return Err("vector must not be empty".into());
    }
    if values.iter().any(|value| !value.is_finite()) {
        return Err("vector entries must be finite".into());
    }
    Ok(DVector::from_vec(values))
}

pub fn matrix_determinant(matrix: DMatrix<f64>) -> Result<f64, String> {
    if !matrix.is_square() {
        return Err("determinant requires a square matrix".into());
    }
    Ok(matrix.determinant())
}

pub fn matrix_inverse(matrix: DMatrix<f64>) -> Result<DMatrix<f64>, String> {
    if !matrix.is_square() {
        return Err("inverse requires a square matrix".into());
    }
    matrix
        .try_inverse()
        .ok_or_else(|| "matrix is singular; inverse does not exist".to_owned())
}

pub fn matrix_solve(matrix: &DMatrix<f64>, vector: &DVector<f64>) -> Result<DVector<f64>, String> {
    if !matrix.is_square() {
        return Err("Ax=b currently requires a square matrix".into());
    }
    if matrix.nrows() != vector.len() {
        return Err(format!(
            "matrix has {} rows but vector has {} entries",
            matrix.nrows(),
            vector.len()
        ));
    }
    matrix
        .clone()
        .lu()
        .solve(vector)
        .ok_or_else(|| "matrix is singular; Ax=b has no unique solution".to_owned())
}

pub fn matrix_rank(matrix: &DMatrix<f64>) -> usize {
    matrix.clone().svd(false, false).rank(1e-12)
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Eigenvalue {
    pub real: f64,
    pub imaginary: f64,
}

pub fn matrix_eigenvalues(matrix: DMatrix<f64>) -> Result<Vec<Eigenvalue>, String> {
    if !matrix.is_square() {
        return Err("eigenvalues require a square matrix".into());
    }
    let values = nalgebra::linalg::Schur::new(matrix).complex_eigenvalues();
    Ok(values
        .iter()
        .map(|value| Eigenvalue {
            real: value.re,
            imaginary: value.im,
        })
        .collect())
}

fn matrix_json(matrix: DMatrix<f64>) -> Result<String, String> {
    let rows = matrix
        .row_iter()
        .map(|row| row.iter().copied().collect::<Vec<_>>())
        .collect::<Vec<_>>();
    serde_json::to_string(&rows).map_err(|e| format!("cannot encode matrix: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expression_operations_work() {
        assert_eq!(execute_command("expand (x+1)^3").output, "1+3*x+3*x^2+x^3");
        assert_eq!(execute_command("diff x^3+2*x; x").output, "2+3*x^2");
        assert_eq!(execute_command("integrate x^2; x").output, "1/3*x^3");
        assert_eq!(execute_command("factor x^2-1").output, "(-1+x)*(1+x)");
        for command in [
            "simplify (x^2-1)/(x-1)",
            "together 1/x+1/y",
            "cancel (x^2-1)/(x-1)",
            "apart 1/(x*(x+1)); x",
            "series exp(x); x; 0; 4",
        ] {
            let result = execute_command(command);
            assert_eq!(result.error, None, "{command}: {result:?}");
        }
        let unavailable = execute_command("integrate sin(x); x");
        assert!(unavailable
            .error
            .as_deref()
            .is_some_and(|error| error.contains("no external integration backend")));
    }

    #[test]
    fn solve_and_numeric_evaluation_work() {
        let solved = execute_command("solve x+y=3,x-y=1; x,y").output;
        assert!(
            solved.contains("x = 2") && solved.contains("y = 1"),
            "{solved}"
        );
        assert_eq!(execute_command("eval-at x^2+y; x=3,y=4").output, "13");
    }

    #[test]
    fn matrix_operations_work() {
        let matrix = parse_matrix("[[1,2],[3,4]]").unwrap();
        let inverse = matrix_inverse(matrix.clone()).unwrap();
        assert!((inverse[(0, 0)] + 2.0).abs() < 1e-12);
        let vector = parse_vector("[5,11]").unwrap();
        let solution = matrix_solve(&matrix, &vector).unwrap();
        assert!((solution - DVector::from_vec(vec![1.0, 2.0])).norm() < 1e-12);
        assert_eq!(matrix_rank(&parse_matrix("[[1,2],[2,4]]").unwrap()), 1);
    }

    #[test]
    fn notebook_roundtrips() {
        let path = std::env::current_dir().unwrap().join(format!(
            ".cas-notebook-unit-test-{}.json",
            std::process::id()
        ));
        let notebook = Notebook {
            cells: vec![NotebookCell {
                id: "one".into(),
                input: "eval 1+2".into(),
                result: execute_command("eval 1+2"),
            }],
        };
        notebook.save(&path).unwrap();
        assert_eq!(Notebook::load(&path).unwrap(), notebook);
        fs::remove_file(path).unwrap();
    }
}
