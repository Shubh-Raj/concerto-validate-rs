//! Concerto Validator RS
//!
//! A Rust library that validates Accord Project Concerto data models in their JSON AST format
//! against the [Concerto Metamodel](https://models.accordproject.org/concerto/metamodel@1.0.0.html)
//!
//! Currently, the library only exposes a single function to validate Concerto ASTs,
//! although internally there are primitive implementations of structures that would sound familiar
//! to the JS classes, like [`ModelManager`](crate::model_manager::ModelManager). But they are not
//! ready for public consumption yet.

pub mod error;
mod model_manager;
mod validator;

use std::sync::OnceLock;

pub use error::{ValidationError, ValidationResult};
use validator::Validator;

// Reference to hold singleton instance of Validator
static GLOBAL_VALIDATOR: OnceLock<Option<Validator>> = OnceLock::new();

/// Validates a Concerto model JSON AST against the system metamodel
pub fn validate_metamodel(json_ast: &str) -> ValidationResult<()> {
    let validator = GLOBAL_VALIDATOR
        .get_or_init(|| Validator::new().ok())
        .as_ref()
        .ok_or(ValidationError::ValidatorInitializationError)?;
    validator.validate(json_ast)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_metamodel_validation() {
        let metamodel_json = include_str!("../metamodel.json");
        let result = validate_metamodel(metamodel_json);
        assert!(
            result.is_ok(),
            "Metamodel validation should succeed: {:?}",
            result
        );
    }

    #[test]
    fn test_invalid_json() {
        let invalid_json = r#"{ invalid json }"#;
        let result = validate_metamodel(invalid_json);
        assert!(result.is_err(), "Invalid JSON should fail validation");
    }

    #[test]
    fn test_invalid_namespace() {
        let invalid_json = r#"{ 
            "$class": "concerto.metamodel@1.0.0.Model",
            "namespace": 123
        }"#;
        let result = validate_metamodel(invalid_json);
        assert!(result.is_err(), "Invalid JSON should fail validation");
    }

    #[test]
    fn test_missing_class_property() {
        let json_without_class = r#"{
            "namespace": "test.namespace",
            "imports": [],
            "declarations": []
        }"#;
        let result = validate_metamodel(json_without_class);
        assert!(
            result.is_err(),
            "JSON without $class should fail validation"
        );
    }

    #[test]
    fn test_simple_model_validation() {
        let simple_model = r#"{
            "$class": "concerto.metamodel@1.0.0.Model",
            "namespace": "test.namespace@1.0.0",
            "imports": [],
            "declarations": [
                {
                    "$class": "concerto.metamodel@1.0.0.ConceptDeclaration",
                    "name": "TestConcept",
                    "isAbstract": false,
                    "properties": [
                        {
                            "$class": "concerto.metamodel@1.0.0.StringProperty",
                            "name": "testField",
                            "isArray": false,
                            "isOptional": false
                        }
                    ]
                }
            ]
        }"#;

        let result = validate_metamodel(simple_model);
        assert!(
            result.is_ok(),
            "Simple valid model should pass validation: {:?}",
            result
        );
    }

    #[test]
    fn test_extra_properties() {
        let simple_model = r#"{
            "$class": "concerto.metamodel@1.0.0.Model",
            "namespace": "test.namespace@1.0.0",
            "imports": [],
            "declarations": [
                {
                    "$class": "concerto.metamodel@1.0.0.ConceptDeclaration",
                    "name": "TestConcept",
                    "isAbstract": false,
                    "isOptional": false,
                    "properties": [
                        {
                            "$class": "concerto.metamodel@1.0.0.StringProperty",
                            "name": "testField",
                            "isArray": false,
                            "isOptional": false,
                            "propertyType": "String"
                        }
                    ]
                }
            ]
        }"#;

        let result = validate_metamodel(simple_model);
        assert!(
            result.is_err(),
            "Extra properties should fail validation: {:?}",
            result
        );
    }

    #[test]
    fn test_boolean_property_type_mismatch() {
        // isAbstract should be a boolean, not a string
        let model_with_wrong_type = r#"{
            "$class": "concerto.metamodel@1.0.0.Model",
            "namespace": "test.namespace@1.0.0",
            "imports": [],
            "declarations": [
                {
                    "$class": "concerto.metamodel@1.0.0.ConceptDeclaration",
                    "name": "TestConcept",
                    "isAbstract": "false",
                    "properties": []
                }
            ]
        }"#;

        let result = validate_metamodel(model_with_wrong_type);
        assert!(
            result.is_err(),
            "String where boolean expected should fail validation"
        );
    }

    #[test]
    fn test_integer_property_type_mismatch() {
        // isOptional should be a boolean, not a string — exercises validate_boolean_property()
        // on a field within an IntegerProperty definition
        let model_with_wrong_type = r#"{
            "$class": "concerto.metamodel@1.0.0.Model",
            "namespace": "test.namespace@1.0.0",
            "imports": [],
            "declarations": [
                {
                    "$class": "concerto.metamodel@1.0.0.ConceptDeclaration",
                    "name": "TestConcept",
                    "isAbstract": false,
                    "properties": [
                        {
                            "$class": "concerto.metamodel@1.0.0.IntegerProperty",
                            "name": "count",
                            "isArray": false,
                            "isOptional": "false"
                        }
                    ]
                }
            ]
        }"#;

        let result = validate_metamodel(model_with_wrong_type);
        assert!(
            result.is_err(),
            "String where boolean expected in IntegerProperty should fail validation"
        );
    }

    #[test]
    fn test_array_property_expects_array() {
        // "properties" is defined as isArray: true in the metamodel,
        // so passing a non-array value should fail validation
        let model_with_non_array = r#"{
            "$class": "concerto.metamodel@1.0.0.Model",
            "namespace": "test.namespace@1.0.0",
            "imports": [],
            "declarations": [
                {
                    "$class": "concerto.metamodel@1.0.0.ConceptDeclaration",
                    "name": "TestConcept",
                    "isAbstract": false,
                    "properties": "not-an-array"
                }
            ]
        }"#;

        let result = validate_metamodel(model_with_non_array);
        assert!(
            result.is_err(),
            "Non-array value for an array property should fail validation"
        );
    }

    #[test]
    fn test_optional_property_can_be_missing() {
        // ConceptDeclaration has an optional "superType" property.
        // Omitting it should NOT cause validation failure.
        // This tests validate_required_properties() correctly skips optional fields.
        let model_without_optional = r#"{
            "$class": "concerto.metamodel@1.0.0.Model",
            "namespace": "test.namespace@1.0.0",
            "imports": [],
            "declarations": [
                {
                    "$class": "concerto.metamodel@1.0.0.ConceptDeclaration",
                    "name": "TestConcept",
                    "isAbstract": false,
                    "properties": []
                }
            ]
        }"#;

        let result = validate_metamodel(model_without_optional);
        assert!(
            result.is_ok(),
            "Missing optional property (superType) should not fail validation: {:?}",
            result
        );
    }
}
