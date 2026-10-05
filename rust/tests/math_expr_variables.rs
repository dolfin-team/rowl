//! Variable operands in math expressions. `PrimaryExpr` accepts a `?var`, so a
//! variable can appear anywhere an `Expr` is expected (rule then/match objects,
//! constraint comparison values). A *bare* variable object must still collapse
//! back to `Object::Variable` for downstream consumers (grammar fold in
//! `Object`), while a compound expression stays an `Object::Literal`.

use rowl::ast::{
    BinaryOp, ComparisonOp, Constraint, Declaration, Expr, Object, Pattern, ThenItem,
};
use rowl::parser::{parse_ontology, parse_result_strict};

fn parse(src: &str) -> rowl::ast::OntologyFile {
    parse_result_strict(parse_ontology(src))
        .unwrap_or_else(|e| panic!("parse failed: {e:?}\n---\n{src}"))
}

fn only_rule(file: &rowl::ast::OntologyFile) -> &rowl::ast::RuleDef {
    file.declarations
        .iter()
        .find_map(|d| match d {
            Declaration::Rule(r) => Some(r),
            _ => None,
        })
        .expect("expected a rule declaration")
}

#[test]
fn then_object_is_variable_operand_expression() {
    let src = "\
rule compute:
  match:
    ?a value ?x
  then:
    ?a total ?x + 1
";
    let file = parse(src);
    let rule = only_rule(&file);
    let item = &rule.then_block.items[0];
    let ThenItem::AssertionTriple { assertion, .. } = item else {
        panic!("expected assertion, got {item:?}");
    };
    let Object::Literal { value, .. } = &assertion.object else {
        panic!("expected Object::Literal, got {:?}", assertion.object);
    };
    let Expr::BinaryOp { op, left, right, .. } = value else {
        panic!("expected BinaryOp, got {value:?}");
    };
    assert_eq!(*op, BinaryOp::Add);
    assert!(
        matches!(left.as_ref(), Expr::Variable { name, .. } if name == "?x"),
        "left operand should be variable ?x, got {left:?}"
    );
    assert!(matches!(right.as_ref(), Expr::Literal { .. }));
}

#[test]
fn bare_variable_object_still_collapses_to_object_variable() {
    let src = "\
rule identity:
  match:
    ?a value ?x
  then:
    ?a copy ?x
";
    let file = parse(src);
    let rule = only_rule(&file);
    let ThenItem::AssertionTriple { assertion, .. } = &rule.then_block.items[0] else {
        panic!("expected assertion");
    };
    assert!(
        matches!(&assertion.object, Object::Variable { name, .. } if name == "?x"),
        "bare ?x must stay Object::Variable, got {:?}",
        assertion.object
    );
}

#[test]
fn comparison_value_can_be_a_variable() {
    // `?x >= ?y` in a match-pattern constraint: the comparison RHS is an `Expr`,
    // now permitted to be a variable.
    let src = "\
rule guard:
  match:
    ?a value [ ?x >= ?y ]
  then:
    ?a ok true
";
    let file = parse(src);
    let rule = only_rule(&file);
    let pattern = &rule.match_block.patterns[0];
    let Pattern::Triple { object, .. } = pattern else {
        panic!("expected triple pattern, got {pattern:?}");
    };
    let Object::Constraint { block } = object else {
        panic!("expected constraint object, got {object:?}");
    };
    let has_var_cmp = block.constraints.iter().any(|c| {
        matches!(
            c,
            Constraint::Comparison { operator: ComparisonOp::GreaterEqual, value: Expr::Variable { name, .. }, .. }
            if name == "?y"
        )
    });
    assert!(has_var_cmp, "expected `?x >= ?y` comparison, got {:?}", block.constraints);
}
