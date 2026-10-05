//! Regression for raft bug #12: `prefix people.customers` must resolve to the
//! file `people/customers.dlf` (only single-segment prefixes used to work).

use std::collections::HashMap;

#[test]
fn multi_segment_prefix_resolves() {
    let files: HashMap<String, String> = [
        (
            "package.dlf",
            "package <http://example.org/bikes>:\n  dolfin_version \"1\"\n  version \"0.1.0\"\n",
        ),
        ("people/customers.dlf", "concept Customer\n"),
        (
            "rentals.dlf",
            "prefix people.customers\n\nconcept Rental:\n  has customer: one customers.Customer\n",
        ),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();

    let pkg = rowl::package::load_package_from_memory(&files).expect("package should load");
    let rentals = pkg
        .ontologies
        .values()
        .find(|o| o.relative_path.ends_with("rentals.dlf"))
        .expect("rentals.dlf loaded");
    let customers_ns = rentals
        .resolved_prefixes
        .get("customers")
        .expect("alias 'customers' resolved");
    assert_eq!(customers_ns.parts.last().map(String::as_str), Some("customers"));
    assert_eq!(customers_ns.parts[customers_ns.parts.len() - 2], "people");
    assert!(pkg.ontologies.contains_key(customers_ns));
}
