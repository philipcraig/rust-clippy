use clippy_utils::diagnostics::{span_lint, span_lint_and_then};
use clippy_utils::macros::{root_macro_call_first_node, FormatArgsExpn, MacroCall};
use clippy_utils::source::{expand_past_previous_comma, snippet_opt};
use rustc_ast::LitKind;
use rustc_errors::Applicability;
use rustc_hir::{Expr, ExprKind, HirIdMap, Impl, Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::{sym, BytePos};

declare_clippy_lint! {
    /// ### What it does
    /// This lint warns when you use `println!("")` to
    /// print a newline.
    ///
    /// ### Why is this bad?
    /// You should use `println!()`, which is simpler.
    ///
    /// ### Example
    /// ```rust
    /// println!("");
    /// ```
    ///
    /// Use instead:
    /// ```rust
    /// println!();
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub PRINTLN_EMPTY_STRING,
    style,
    "using `println!(\"\")` with an empty string"
}

declare_clippy_lint! {
    /// ### What it does
    /// This lint warns when you use `print!()` with a format
    /// string that ends in a newline.
    ///
    /// ### Why is this bad?
    /// You should use `println!()` instead, which appends the
    /// newline.
    ///
    /// ### Example
    /// ```rust
    /// # let name = "World";
    /// print!("Hello {}!\n", name);
    /// ```
    /// use println!() instead
    /// ```rust
    /// # let name = "World";
    /// println!("Hello {}!", name);
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub PRINT_WITH_NEWLINE,
    style,
    "using `print!()` with a format string that ends in a single newline"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for printing on *stdout*. The purpose of this lint
    /// is to catch debugging remnants.
    ///
    /// ### Why is this bad?
    /// People often print on *stdout* while debugging an
    /// application and might forget to remove those prints afterward.
    ///
    /// ### Known problems
    /// Only catches `print!` and `println!` calls.
    ///
    /// ### Example
    /// ```rust
    /// println!("Hello world!");
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub PRINT_STDOUT,
    restriction,
    "printing on stdout"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for printing on *stderr*. The purpose of this lint
    /// is to catch debugging remnants.
    ///
    /// ### Why is this bad?
    /// People often print on *stderr* while debugging an
    /// application and might forget to remove those prints afterward.
    ///
    /// ### Known problems
    /// Only catches `eprint!` and `eprintln!` calls.
    ///
    /// ### Example
    /// ```rust
    /// eprintln!("Hello world!");
    /// ```
    #[clippy::version = "1.50.0"]
    pub PRINT_STDERR,
    restriction,
    "printing on stderr"
}

declare_clippy_lint! {
    /// ### What it does
    /// Checks for use of `Debug` formatting. The purpose of this
    /// lint is to catch debugging remnants.
    ///
    /// ### Why is this bad?
    /// The purpose of the `Debug` trait is to facilitate
    /// debugging Rust code. It should not be used in user-facing output.
    ///
    /// ### Example
    /// ```rust
    /// # let foo = "bar";
    /// println!("{:?}", foo);
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub USE_DEBUG,
    restriction,
    "use of `Debug`-based formatting"
}

declare_clippy_lint! {
    /// ### What it does
    /// This lint warns about the use of literals as `print!`/`println!` args.
    ///
    /// ### Why is this bad?
    /// Using literals as `println!` args is inefficient
    /// (c.f., https://github.com/matthiaskrgr/rust-str-bench) and unnecessary
    /// (i.e., just put the literal in the format string)
    ///
    /// ### Example
    /// ```rust
    /// println!("{}", "foo");
    /// ```
    /// use the literal without formatting:
    /// ```rust
    /// println!("foo");
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub PRINT_LITERAL,
    style,
    "printing a literal with a format string"
}

declare_clippy_lint! {
    /// ### What it does
    /// This lint warns when you use `writeln!(buf, "")` to
    /// print a newline.
    ///
    /// ### Why is this bad?
    /// You should use `writeln!(buf)`, which is simpler.
    ///
    /// ### Example
    /// ```rust
    /// # use std::fmt::Write;
    /// # let mut buf = String::new();
    /// writeln!(buf, "");
    /// ```
    ///
    /// Use instead:
    /// ```rust
    /// # use std::fmt::Write;
    /// # let mut buf = String::new();
    /// writeln!(buf);
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub WRITELN_EMPTY_STRING,
    style,
    "using `writeln!(buf, \"\")` with an empty string"
}

declare_clippy_lint! {
    /// ### What it does
    /// This lint warns when you use `write!()` with a format
    /// string that
    /// ends in a newline.
    ///
    /// ### Why is this bad?
    /// You should use `writeln!()` instead, which appends the
    /// newline.
    ///
    /// ### Example
    /// ```rust
    /// # use std::fmt::Write;
    /// # let mut buf = String::new();
    /// # let name = "World";
    /// write!(buf, "Hello {}!\n", name);
    /// ```
    ///
    /// Use instead:
    /// ```rust
    /// # use std::fmt::Write;
    /// # let mut buf = String::new();
    /// # let name = "World";
    /// writeln!(buf, "Hello {}!", name);
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub WRITE_WITH_NEWLINE,
    style,
    "using `write!()` with a format string that ends in a single newline"
}

declare_clippy_lint! {
    /// ### What it does
    /// This lint warns about the use of literals as `write!`/`writeln!` args.
    ///
    /// ### Why is this bad?
    /// Using literals as `writeln!` args is inefficient
    /// (c.f., https://github.com/matthiaskrgr/rust-str-bench) and unnecessary
    /// (i.e., just put the literal in the format string)
    ///
    /// ### Example
    /// ```rust
    /// # use std::fmt::Write;
    /// # let mut buf = String::new();
    /// writeln!(buf, "{}", "foo");
    /// ```
    ///
    /// Use instead:
    /// ```rust
    /// # use std::fmt::Write;
    /// # let mut buf = String::new();
    /// writeln!(buf, "foo");
    /// ```
    #[clippy::version = "pre 1.29.0"]
    pub WRITE_LITERAL,
    style,
    "writing a literal with a format string"
}

#[derive(Default)]
pub struct Write {
    in_debug_impl: bool,
}

impl_lint_pass!(Write => [
    PRINT_WITH_NEWLINE,
    PRINTLN_EMPTY_STRING,
    PRINT_STDOUT,
    PRINT_STDERR,
    USE_DEBUG,
    PRINT_LITERAL,
    WRITE_WITH_NEWLINE,
    WRITELN_EMPTY_STRING,
    WRITE_LITERAL,
]);

impl<'tcx> LateLintPass<'tcx> for Write {
    fn check_item(&mut self, cx: &LateContext<'_>, item: &Item<'_>) {
        if is_debug_impl(cx, item) {
            self.in_debug_impl = true;
        }
    }

    fn check_item_post(&mut self, cx: &LateContext<'_>, item: &Item<'_>) {
        if is_debug_impl(cx, item) {
            self.in_debug_impl = false;
        }
    }

    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'_>) {
        let Some(macro_call) = root_macro_call_first_node(cx, expr) else { return };
        let Some(diag_name) = cx.tcx.get_diagnostic_name(macro_call.def_id) else { return };
        let Some(name) = diag_name.as_str().strip_suffix("_macro") else { return };

        let is_build_script = cx
            .sess()
            .opts
            .crate_name
            .as_ref()
            .map_or(false, |crate_name| crate_name == "build_script_build");

        match diag_name {
            sym::print_macro | sym::println_macro => {
                if !is_build_script {
                    span_lint(cx, PRINT_STDOUT, macro_call.span, &format!("use of `{name}!`"));
                }
            },
            sym::eprint_macro | sym::eprintln_macro => {
                span_lint(cx, PRINT_STDERR, macro_call.span, &format!("use of `{name}!`"));
            },
            sym::write_macro | sym::writeln_macro => {},
            _ => return,
        }

        let Some(format_args) = FormatArgsExpn::find_nested(cx, expr, macro_call.expn) else { return };

        // ignore `writeln!(w)` and `write!(v, some_macro!())`
        if format_args.format_string.span.from_expansion() {
            return;
        }

        match diag_name {
            sym::print_macro | sym::eprint_macro | sym::write_macro => {
                check_newline(cx, &format_args, &macro_call, name);
            },
            sym::println_macro | sym::eprintln_macro | sym::writeln_macro => {
                check_empty_string(cx, &format_args, &macro_call, name);
            },
            _ => {},
        }

        check_literal(cx, &format_args, name);

        if !self.in_debug_impl {
            for arg in &format_args.args {
                if arg.format.r#trait == sym::Debug {
                    span_lint(cx, USE_DEBUG, arg.span, "use of `Debug`-based formatting");
                }
            }
        }
    }
}
fn is_debug_impl(cx: &LateContext<'_>, item: &Item<'_>) -> bool {
    if let ItemKind::Impl(Impl { of_trait: Some(trait_ref), .. }) = &item.kind
        && let Some(trait_id) = trait_ref.trait_def_id()
    {
        cx.tcx.is_diagnostic_item(sym::Debug, trait_id)
    } else {
        false
    }
}

fn check_newline(cx: &LateContext<'_>, format_args: &FormatArgsExpn<'_>, macro_call: &MacroCall, name: &str) {
    let format_string_parts = &format_args.format_string.parts;
    let mut format_string_span = format_args.format_string.span;

    let Some(last) = format_string_parts.last() else { return };

    let count_vertical_whitespace = || {
        format_string_parts
            .iter()
            .flat_map(|part| part.as_str().chars())
            .filter(|ch| matches!(ch, '\r' | '\n'))
            .count()
    };

    if last.as_str().ends_with('\n')
        // ignore format strings with other internal vertical whitespace
        && count_vertical_whitespace() == 1

        // ignore trailing arguments: `print!("Issue\n{}", 1265);`
        && format_string_parts.len() > format_args.args.len()
    {
        let lint = if name == "write" {
            format_string_span = expand_past_previous_comma(cx, format_string_span);

            WRITE_WITH_NEWLINE
        } else {
            PRINT_WITH_NEWLINE
        };

        span_lint_and_then(
            cx,
            lint,
            macro_call.span,
            &format!("using `{name}!()` with a format string that ends in a single newline"),
            |diag| {
                let name_span = cx.sess().source_map().span_until_char(macro_call.span, '!');
                let Some(format_snippet) = snippet_opt(cx, format_string_span) else { return };

                if format_string_parts.len() == 1 && last.as_str() == "\n" {
                    // print!("\n"), write!(f, "\n")

                    diag.multipart_suggestion(
                        &format!("use `{name}ln!` instead"),
                        vec![(name_span, format!("{name}ln")), (format_string_span, String::new())],
                        Applicability::MachineApplicable,
                    );
                } else if format_snippet.ends_with("\\n\"") {
                    // print!("...\n"), write!(f, "...\n")

                    let hi = format_string_span.hi();
                    let newline_span = format_string_span.with_lo(hi - BytePos(3)).with_hi(hi - BytePos(1));

                    diag.multipart_suggestion(
                        &format!("use `{name}ln!` instead"),
                        vec![(name_span, format!("{name}ln")), (newline_span, String::new())],
                        Applicability::MachineApplicable,
                    );
                }
            },
        );
    }
}

fn check_empty_string(cx: &LateContext<'_>, format_args: &FormatArgsExpn<'_>, macro_call: &MacroCall, name: &str) {
    if let [part] = &format_args.format_string.parts[..]
        && let mut span = format_args.format_string.span
        && part.as_str() == "\n"
    {
        let lint = if name == "writeln" {
            span = expand_past_previous_comma(cx, span);

            WRITELN_EMPTY_STRING
        } else {
            PRINTLN_EMPTY_STRING
        };

        span_lint_and_then(
            cx,
            lint,
            macro_call.span,
            &format!("empty string literal in `{name}!`"),
            |diag| {
                diag.span_suggestion(
                    span,
                    "remove the empty string",
                    String::new(),
                    Applicability::MachineApplicable,
                );
            },
        );
    }
}

fn check_literal(cx: &LateContext<'_>, format_args: &FormatArgsExpn<'_>, name: &str) {
    let mut counts = HirIdMap::<usize>::default();
    for param in format_args.params() {
        *counts.entry(param.value.hir_id).or_default() += 1;
    }

    for arg in &format_args.args {
        let value = arg.param.value;

        if counts[&value.hir_id] == 1
            && arg.format.is_default()
            && let ExprKind::Lit(lit) = &value.kind
            && !value.span.from_expansion()
            && let Some(value_string) = snippet_opt(cx, value.span)
        {
            let (replacement, replace_raw) = match lit.node {
                LitKind::Str(..) => extract_str_literal(&value_string),
                LitKind::Char(ch) => (
                    match ch {
                        '"' => "\\\"",
                        '\'' => "'",
                        _ => &value_string[1..value_string.len() - 1],
                    }
                    .to_string(),
                    false,
                ),
                LitKind::Bool(b) => (b.to_string(), false),
                _ => continue,
            };

            let lint = if name.starts_with("write") {
                WRITE_LITERAL
            } else {
                PRINT_LITERAL
            };

            let format_string_is_raw = format_args.format_string.style.is_some();
            let replacement = match (format_string_is_raw, replace_raw) {
                (false, false) => Some(replacement),
                (false, true) => Some(replacement.replace('"', "\\\"").replace('\\', "\\\\")),
                (true, false) => match conservative_unescape(&replacement) {
                    Ok(unescaped) => Some(unescaped),
                    Err(UnescapeErr::Lint) => None,
                    Err(UnescapeErr::Ignore) => continue,
                },
                (true, true) => {
                    if replacement.contains(['#', '"']) {
                        None
                    } else {
                        Some(replacement)
                    }
                },
            };

            span_lint_and_then(
                cx,
                lint,
                value.span,
                "literal with an empty format string",
                |diag| {
                    if let Some(replacement) = replacement {
                        // `format!("{}", "a")`, `format!("{named}", named = "b")
                        //              ~~~~~                      ~~~~~~~~~~~~~
                        let value_span = expand_past_previous_comma(cx, value.span);

                        let replacement = replacement.replace('{', "{{").replace('}', "}}");
                        diag.multipart_suggestion(
                            "try this",
                            vec![(arg.span, replacement), (value_span, String::new())],
                            Applicability::MachineApplicable,
                        );
                    }
                },
            );
        }
    }
}

/// Removes the raw marker, `#`s and quotes from a str, and returns if the literal is raw
///
/// `r#"a"#` -> (`a`, true)
///
/// `"b"` -> (`b`, false)
fn extract_str_literal(literal: &str) -> (String, bool) {
    let (literal, raw) = match literal.strip_prefix('r') {
        Some(stripped) => (stripped.trim_matches('#'), true),
        None => (literal, false),
    };

    (literal[1..literal.len() - 1].to_string(), raw)
}

enum UnescapeErr {
    /// Should still be linted, can be manually resolved by author, e.g.
    ///
    /// ```ignore
    /// print!(r"{}", '"');
    /// ```
    Lint,
    /// Should not be linted, e.g.
    ///
    /// ```ignore
    /// print!(r"{}", '\r');
    /// ```
    Ignore,
}

/// Unescape a normal string into a raw string
fn conservative_unescape(literal: &str) -> Result<String, UnescapeErr> {
    let mut unescaped = String::with_capacity(literal.len());
    let mut chars = literal.chars();
    let mut err = false;

    while let Some(ch) = chars.next() {
        match ch {
            '#' => err = true,
            '\\' => match chars.next() {
                Some('\\') => unescaped.push('\\'),
                Some('"') => err = true,
                _ => return Err(UnescapeErr::Ignore),
            },
            _ => unescaped.push(ch),
        }
    }

    if err { Err(UnescapeErr::Lint) } else { Ok(unescaped) }
}
