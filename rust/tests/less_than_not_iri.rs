//! A `<` comparison followed later in the file by a `>` must not lex as an IRI.
use rowl::error::Severity;
use rowl::parser::parse_ontology;

#[test]
fn less_than_before_greater_than_parses() {
    let src = "\
concept A:
  has x: float
concept B

rule r1:
  match:
    ?a x [ < 20.0 ]
  then:
    ?a a B

rule r2:
  match:
    ?a x [ > 5.0 ]
  then:
    ?a a B
";
    let errs: Vec<_> = parse_ontology(src)
        .diagnostics
        .into_iter()
        .filter(|d| d.severity == Severity::HardError)
        .map(|d| d.message)
        .collect();
    assert!(errs.is_empty(), "comparison lexed as IRI: {errs:?}");
}
