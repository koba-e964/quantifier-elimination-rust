# Formula syntax

The parser accepts the grammar below. Whitespace may appear between tokens
and is ignored. `<digit>` denotes an ASCII decimal digit.

```bnf
<formula> ::= <or-formula>

<or-formula> ::= <and-formula>
               | <or-formula> "||" <and-formula>

<and-formula> ::= <unary-formula>
                | <and-formula> "&&" <unary-formula>

<unary-formula> ::= "!" <unary-formula>
                  | <quantified-formula>
                  | "(" <formula> ")"
                  | "true"
                  | "false"
                  | <polynomial> <relation> <polynomial>

<quantified-formula> ::= <quantifier> <variable> "." <formula>

<quantifier> ::= "exists"
               | "forall"

<relation> ::= "="
             | "!="
             | "<"
             | "<="
             | ">"
             | ">="

<polynomial> ::= <sum>

<sum> ::= <product>
        | <sum> "+" <product>
        | <sum> "-" <product>

<product> ::= <power>
            | <product> "*" <power>

<power> ::= <signed-atom>
          | <signed-atom> "^" <exponent>

<signed-atom> ::= <polynomial-atom>
                | "+" <signed-atom>
                | "-" <signed-atom>

<polynomial-atom> ::= <integer>
                    | <variable>
                    | "(" <polynomial> ")"

<exponent> ::= <digits>

<variable> ::= "x" <digits>
              | <identifier>

<identifier> ::= <identifier-start> <identifier-continue>*

<identifier-start> ::= <letter> | "_"

<identifier-continue> ::= <letter> | <digit> | "_"

<integer> ::= <digits>

<digits> ::= <digit>
           | <digits> <digit>

<digit> ::= "0" | "1" | "2" | "3" | "4"
          | "5" | "6" | "7" | "8" | "9"
```

## Precedence and associativity

From tightest to loosest, operators are parsed as follows:

1. Polynomial parentheses and unary `+`/`-`.
2. Polynomial exponentiation `^`.
3. Polynomial multiplication `*`.
4. Polynomial addition and subtraction `+`, `-`.
5. Formula comparisons.
6. Negation `!` and quantifiers.
7. Conjunction `&&`.
8. Disjunction `||`.

Addition, subtraction, multiplication, conjunction, and disjunction are
left-associative. Quantifiers extend to the end of the following formula, so
parentheses are needed to limit their scope.

Examples:

```text
exists x0. x0^2 + 1 = 0
forall x0. (x0 >= 0) || !(x0 < 0)
exists x0. exists x1. x2 = x0 + x1 && x3 = x0 * x1
exists x. x^2 + st + y = 0
```

Variables may use the compatibility form `x` followed by a non-negative
decimal index, such as `x0`, `x1`, or `x42`, or ergonomic identifiers such as
`x`, `y`, `st`, and `tmp`. Identifiers may contain letters, digits, and
underscores and must not be `exists`, `forall`, `true`, or `false`. Integer
literals are decimal and non-negative; negative constants use unary `-`, for
example `-3`. Floating-point literals and implicit multiplication are not
supported.
