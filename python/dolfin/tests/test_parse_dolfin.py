import pytest

import rowl

import os
this_file_path = os.path.dirname(os.path.realpath(__file__))
DLF_PATH = os.path.join(this_file_path, "resources")


class TestParseEmpty:
    """Tests for empty and minimal inputs."""

    def test_parse_empty(self):
        tree = rowl.parse("")
        print(dir(tree))
        assert tree.concepts == []
        assert tree.declarations == []
        assert tree.iri_name is None
        assert tree.prefixes == []

    def test_parse_only_whitespace(self):
        tree = rowl.parse("   \n\n   \n")
        assert tree.concepts == []
        assert tree.declarations == []
        assert tree.iri_name is None
        assert tree.prefixes == []

    def test_parse_only_comment(self):
        tree = rowl.parse_file(f"{DLF_PATH}/only_comment.dlf")
        assert tree.concepts == []
        assert tree.declarations == []
        assert tree.iri_name is None
        assert tree.prefixes == []

    def test_parse_multiple_comments(self):
        tree = rowl.parse_file(f"{DLF_PATH}/multiple_comments.dlf")
        assert tree.concepts == []
        assert tree.declarations == []
        assert tree.iri_name is None
        assert tree.prefixes == []


class TestParseImport:
    """Tests for import statements."""

    def test_parse_simple_import(self):
        result = rowl.parse("prefix Person\n")
        assert result is not None

    def test_parse_qualified_import(self):
        result = rowl.parse("prefix com.example.Person\n")
        assert result is not None

    def test_parse_import_with_alias(self):
        result = rowl.parse("prefix com.example.Person as P\n")
        assert result is not None

    def test_parse_hierarchical_import(self):
        code = """prefix com.example:
  Person
  Organization
"""
        result = rowl.parse(code)
        assert result is not None

    def test_parse_hierarchical_import_with_aliases(self):
        code = """prefix com.example:
  Person as P
  Organization as Org
"""
        result = rowl.parse(code)
        assert result is not None

    def test_parse_nested_hierarchical_import(self):
        code = """prefix com.example:
  Person
  organization:
    Federal
    National
"""
        result = rowl.parse(code)
        assert result is not None

    def test_parse_import_missing_target_raises(self):
        with pytest.raises(ValueError):
            rowl.parse("prefix\n")


class TestParseOntology:
    """Tests for ontology definitions."""

    def test_parse_ontology_with_enum(self):
        code = """enum Status:
    active | inactive | pending
"""
        result = rowl.parse(code)
        assert result is not None

    def test_parse_ontology_with_concept(self):
        code = """concept Person:
    has name: string
"""
        result = rowl.parse(code)
        assert result is not None

    def test_parse_ontology_with_property(self):
        code = """property worksFor: Person -> Organization
"""
        result = rowl.parse(code)
        assert result is not None

    def test_parse_ontology_with_multiple_declarations(self):
        result = rowl.parse_file(
            f"{DLF_PATH}/ontology_with_multiple_declarations.dlf"
        )
        assert result is not None


class TestParseEnum:
    """Tests for enumeration definitions."""

    def test_parse_enum_single_value(self):
        code = """enum SingleValue:
    only
"""
        result = rowl.parse(code)
        assert result is not None

    def test_parse_enum_inline_values(self):
        code = """enum Status:
    active | inactive | pending
"""
        result = rowl.parse(code)
        assert result is not None

    def test_parse_enum_multiline_values(self):
        code = """enum Status:
    active
    inactive
    pending
"""
        result = rowl.parse(code)
        assert result is not None

    def test_parse_enum_mixed_format(self):
        code = """enum Priority:
    low | medium
    high | critical
"""
        result = rowl.parse(code)
        assert result is not None


class TestParseConcept:
    """Tests for concept definitions."""

    def test_parse_concept_with_single_property(self):
        code = """concept Person:
    has name: string
"""
        result = rowl.parse(code)
        assert result is not None

    def test_parse_concept_with_multiple_properties(self):
        code = """concept Person:
    has name: string
    has age: int
    has salary: float
    has active: boolean
"""
        result = rowl.parse(code)
        assert result is not None

    def test_parse_concept_with_inheritance(self):
        code = """concept Employee:
    sub Person
    has employeeId: string
"""
        result = rowl.parse(code)
        assert result is not None

    def test_parse_concept_with_multiple_inheritance(self):
        code = """concept Manager:
    sub Person, Employee
    has teamSize: int
"""
        result = rowl.parse(code)
        assert result is not None

    def test_parse_concept_with_qualified_type_reference(self):
        code = """concept Employee:
    has department: com.example.Department
"""
        result = rowl.parse(code)
        assert result is not None


class TestParseProperty:
    """Tests for property definitions."""

    def test_parse_simple_property(self):
        code = """property worksFor: Person -> Organization
"""
        result = rowl.parse(code)
        assert result is not None

    def test_parse_property_with_cardinality_one(self):
        code = """property hasManager: one Employee -> one Manager
"""
        result = rowl.parse(code)
        assert result is not None

    def test_parse_property_with_cardinality_any(self):
        code = """property hasChild: Person -> any Person
"""
        result = rowl.parse(code)
        assert result is not None

    def test_parse_property_with_cardinality_some(self):
        code = """property hasEmployee: Organization -> some Employee
"""
        result = rowl.parse(code)
        assert result is not None

    def test_parse_property_with_cardinality_optional(self):
        code = """property hasSpouse: Person -> optional Person
"""
        result = rowl.parse(code)
        assert result is not None

    def test_parse_property_with_cardinality_range(self):
        code = """property hasWheels: Vehicle -> 2..6 Wheel
"""
        result = rowl.parse(code)
        assert result is not None

    def test_parse_property_with_cardinality_min_range(self):
        code = """property hasMembers: Team -> 1..* Person
"""
        result = rowl.parse(code)
        assert result is not None

    def test_parse_property_with_both_cardinalities(self):
        code = """property manages: optional Manager -> some Employee
"""
        result = rowl.parse(code)
        assert result is not None


class TestParsePrimitiveTypes:
    """Tests for primitive type references."""

    def test_parse_string_type(self):
        code = """concept Entity:
    has name: string
"""
        result = rowl.parse(code)
        assert result is not None

    def test_parse_int_type(self):
        code = """concept Entity:
    has count: int
"""
        result = rowl.parse(code)
        assert result is not None

    def test_parse_float_type(self):
        code = """concept Entity:
    has value: float
"""
        result = rowl.parse(code)
        assert result is not None

    def test_parse_boolean_type(self):
        code = """concept Entity:
    has active: boolean
"""
        result = rowl.parse(code)
        assert result is not None


class TestParseComplexOntology:
    """Tests for complex, realistic ontology definitions."""

    def test_parse_complete_ontology(self):
        result = rowl.parse_file(f"{DLF_PATH}/complete_ontology.dlf")
        assert result is not None

    def test_parse_multiple_ontologies_in_file(self):
        result = rowl.parse_file(f"{DLF_PATH}/multiple_ontologies_in_file.dlf")
        assert result is not None


# class TestQualifiedName:
#     """Tests for QualifiedName class."""

#     def test_qualified_name_from_single_component(self):
#         qn = QualifiedName.formList(["Person"])
#         assert qn.prefix == ""
#         assert qn.last == "Person"
#         assert qn.full == "Person"

#     def test_qualified_name_from_two_components(self):
#         qn = QualifiedName.formList(["com", "Person"])
#         assert qn.prefix == "com"
#         assert qn.last == "Person"
#         assert qn.full == "com.Person"

#     def test_qualified_name_from_multiple_components(self):
#         qn = QualifiedName.formList(["com", "example", "domain", "Person"])
#         assert qn.prefix == "com.example.domain"
#         assert qn.last == "Person"
#         assert qn.full == "com.example.domain.Person"

#     def test_qualified_name_division_simple(self):
#         base = QualifiedName("com", "example")
#         sub = QualifiedName("", "Person")
#         combined = base / sub
#         assert combined.full == "com.example.Person"

#     def test_qualified_name_division_with_prefix(self):
#         base = QualifiedName("com", "example")
#         sub = QualifiedName("models", "Person")
#         combined = base / sub
#         assert combined.full == "com.example.models.Person"

#     def test_qualified_name_repr(self):
#         qn = QualifiedName("com.example", "Person")
#         assert repr(qn) == "Q(com.example, Person)"


# class TestImportStatement:
#     """Tests for ImportStatement class."""

#     def test_import_statement_creation(self):
#         qn = QualifiedName("com.example", "Person")
#         imp = ImportStatement(qn, "Person")
#         assert imp.expanded == qn
#         assert imp.alias == "Person"

#     def test_import_statement_with_alias(self):
#         qn = QualifiedName("com.example", "Person")
#         imp = ImportStatement(qn, "P")
#         assert imp.expanded == qn
#         assert imp.alias == "P"

#     def test_import_statement_repr(self):
#         qn = QualifiedName("com.example", "Person")
#         imp = ImportStatement(qn, "P")
#         assert "com.example.Person" in repr(imp) or "Q(com.example, Person)" in repr(imp)
