"""
# Dolfin Ontology Language

**Dolfin** is a human-readable, indentation-based language for defining ontologies.
It provides a clean and expressive syntax for modeling concepts, properties,
enumerations, and their relationships.

## Overview

Dolfin is designed to make ontology definition accessible and maintainable.
Its Python-like indentation syntax eliminates the need for verbose XML or
complex RDF notations while maintaining full expressiveness for ontology modeling.

### Key Features

- **Indentation-based syntax**: Clean, readable structure similar to Python
- **Namespace management**: Hierarchical organization of ontology definitions
- **Rich type system**: Primitive types (`string`, `int`, `float`, `boolean`) and user-defined types
- **Concept hierarchies**: Define concepts with inheritance using `sub` declarations
- **Property definitions**: Model relationships with domain and range specifications
- **Cardinality constraints**: Express multiplicity with `one`, `any`, `some`, `optional`, or ranges
- **Enumeration types**: Define finite sets of named values

## Quick Start

### Installation

```bash
pip install dolfin
```

### Basic Usage

```python
from dolfin import parser

# Parse a Dolfin string
result = parser.parse('''
namespace com.example

ontology MyOntology:
  concept Person:
    has name: string
    has age: int
''')

# Parse a Dolfin file
result = parser.parse_file("my_ontology.dlf")
```

## Language Syntax

### Namespace Declaration

Every Dolfin file should declare its namespace:

```dolfin
namespace com.example.myproject
```

### Import Statements

Import definitions from other modules:

```dolfin
# Simple import
import com.example.Person

# Import with alias
import com.example.Organization as Org

# Hierarchical imports
import com.example:
  Person
  Organization as Org
  address:
    Street
    City
```

### Ontology Definition

Define an ontology containing concepts, properties, and enumerations:

```dolfin
ontology CompanyModel:

  enum EmploymentStatus:
    active | inactive | onLeave | terminated

  concept Person:
    has firstName: string
    has lastName: string
    has age: optional int

  concept Employee:
    sub Person
    has employeeId: string
    has status: EmploymentStatus
    has manager: optional Employee

  property worksFor: Employee -> Company
  property manages: optional Employee -> any Employee
```

### Concepts

Concepts are analogous to classes. They support inheritance and property definitions:

```dolfin
concept Animal:
  has name: string
  has age: int

concept Dog:
  sub Animal
  has breed: string
  has owner: optional Person
```

### Properties

Properties define relationships between concepts with optional cardinality:

```dolfin
# Simple property
property owns: Person -> Pet

# Property with cardinality constraints
property hasChild: one Person -> any Person
property employs: Company -> some Employee
```

### Cardinality Constraints

Dolfin supports flexible cardinality specifications:

| Constraint | Meaning |
|------------|---------|
| `one` | Exactly one (default) |
| `optional` | Zero or one |
| `any` | Zero or more |
| `some` | One or more |
| `N..M` | Between N and M |
| `N..*` | At least N |

### Enumerations

Define fixed sets of values:

```dolfin
enum Priority:
  low | medium | high | critical

enum DayOfWeek:
  monday
  | tuesday
  | wednesday
  | thursday
  | friday
  | saturday
  | sunday
```

### Primitive Types

Dolfin provides four primitive types:

- `string`: Text data
- `int`: Integer numbers
- `float`: Floating-point numbers
- `boolean`: True/false values

## Module Structure

The `dolfin` package provides the following components:

- `dolfin.parser`: Parser module for processing Dolfin source code
- `dolfin.dolfin`: AST classes and transformers

### Parser Module

The parser module exposes a singleton `parser` instance:

```python
from dolfin.parser import parser

# Parse string
ast = parser.parse("ontology Example:")

# Parse file
ast = parser.parse_file("example.dlf")
```

### AST Classes

Key AST classes for working with parsed Dolfin code:

- `QualifiedName`: Represents dot-separated identifiers
- `ImportStatement`: Represents import declarations

## Examples

### Complete Ontology Example

```dolfin
namespace org.example.library

import org.example.common:
  Address
  ContactInfo

ontology LibrarySystem:

  enum BookStatus:
    available | checkedOut | reserved | lost

  enum MembershipType:
    basic | premium | student | senior

  concept Person:
    has name: string
    has email: optional string
    has address: optional Address

  concept Author:
    sub Person
    has biography: optional string
    has nationality: optional string

  concept Member:
    sub Person
    has memberId: string
    has membershipType: MembershipType
    has joinDate: string

  concept Book:
    has isbn: string
    has title: string
    has publicationYear: int
    has status: BookStatus
    has authors: some Author

  concept Loan:
    has loanDate: string
    has dueDate: string
    has returnDate: optional string

  property wrote: Author -> any Book
  property borrowed: Member -> any Loan
  property loanedBook: one Loan -> one Book
```

## File Extension

Dolfin source files conventionally use the `.dlf` extension.

## Comments

Single-line comments start with `#`:

```dolfin
# This is a comment
namespace com.example  # Inline comment

ontology Example:
  # Define a simple concept
  concept Thing:
    has id: string  # Unique identifier
```
"""
