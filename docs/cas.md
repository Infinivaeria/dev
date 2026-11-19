# Selenite CAS Notebook

Selenite 18 includes a native Computer Algebra System backed by Symbolica for
exact symbolic mathematics and nalgebra for numerical linear algebra. Launch it
from the **CAS** organizer button, **Suite > CAS Notebook**, a managed CAS desktop
shortcut, or directly:

```sh
./selenite --cas
./selenite --cas workbook.cas.json
./selenite --cas-run 'diff x^3+2*x; x'
```

The packaged executable runs CAS workflows directly; no Python, Ruby, shell
wrapper or external computer-algebra process is required.

## Notebook workflow

The scaled CAS Notebook starts as a 1100 x 760 logical canvas and letterboxes
without changing control proportions. It supports:

- New, delete, duplicate and select cells.
- Run, Save and Load.
- Scrollable input/result/error/step display.
- Ctrl+Enter run, Ctrl+S save, Ctrl+N new, Ctrl+D duplicate, Up/Down selection,
  Delete removal and Escape close.
- JSON persistence at `$XDG_CONFIG_HOME/selenite/cas/notebook.json`, or the
  notebook path supplied with `--cas`.

Save notebooks are ordinary JSON containing each cell input and its structured
`CasResult`. Evaluate all saved cells headlessly with:

```sh
./selenite --cas-notebook workbook.cas.json
```

## Command language

Expressions use Symbolica syntax. Separate operation arguments with semicolons.
Top-level commas separate equations, variables or matrix rows. Examples:

| Goal | Command |
|---|---|
| Simplify | `simplify (x^2-1)/(x-1)` |
| Expand | `expand (x+1)^4` |
| Factor | `factor x^2-1` |
| Combine fractions | `together 1/x+1/y` |
| Cancel | `cancel (x^2-1)/(x-1)` |
| Partial fractions | `apart 1/(x*(x+1)); x` |
| Differentiate | `diff x^3+2*x; x` |
| Integrate | `integrate x^2; x` |
| Taylor series | `series exp(x); x; 0; 5` |
| Solve | `solve x+y=3,x-y=1; x,y` |
| Substitute/evaluate | `eval-at x^2+y; x=3,y=4` |
| Determinant | `determinant [[1,2],[3,4]]` |
| Inverse | `inverse [[1,2],[3,4]]` |
| Solve Ax=b | `matrix-solve [[2,1],[1,3]]; [5,7]` |
| Rank | `rank [[1,2],[2,4]]` |
| Eigenvalues | `eigenvalues [[2,1],[1,2]]` |

Matrices and vectors use finite JSON numbers. Selenite reports malformed shapes,
singular matrices and unsupported operations explicitly rather than returning a
success-shaped result.

## Symbolic backend scope and licensing

Symbolica provides parsing, expansion, factorization, rational transformations,
differentiation, available exact integration, series, solving and numeric
evaluation. nalgebra provides numerical matrices, LU solving, SVD rank, inverse
and complex Schur eigenvalues.

The unrestricted distributable configuration is **non-commercial** under
Symbolica's public restricted license terms and runs on one Symbolica thread per
user. It may print Symbolica's license notice on first use. Commercial use,
additional cores or advanced licensed integration features require a valid
Symbolica key and are not implied by Selenite. Selenite does not bundle a key.

Unsupported integrations report an error naming the limitation. The public CAS
does not route expressions to a network service or claim that a full paid
Symbolica backend is present.

## Release verification

CAS tests cover exact transformations, derivatives, available integration,
series, solving, substitution, matrix operations, notebook JSON, command
parsing, scaled GUI geometry, no-selection safety and packaged CLI smoke.
