// liquid_derive::FilterReflection violates this lint
#![allow(clippy::box_default)]

use heck::{ToKebabCase, ToSnakeCase, ToUpperCamelCase};
use liquid_core::{Filter, ParseFilter, Runtime, ValueView};
use liquid_derive::FilterReflection;

// Filters that are added to the Liquid parser and allow templates to specify
// transformations of input strings

#[derive(Clone, ParseFilter, FilterReflection)]
#[filter(
    name = "kebab_case",
    description = "Change text to kebab-case.",
    parsed(KebabCaseFilter)
)]
pub(crate) struct KebabCaseFilterParser;

#[derive(Debug, Default, liquid_derive::Display_filter)]
#[name = "kebab_case"]
struct KebabCaseFilter;

impl Filter for KebabCaseFilter {
    fn evaluate(
        &self,
        input: &dyn ValueView,
        _runtime: &dyn Runtime,
    ) -> Result<liquid_core::model::Value, liquid_core::error::Error> {
        let input = input
            .as_scalar()
            .ok_or_else(|| liquid_core::error::Error::with_msg("String expected"))?;

        let input = input.into_string().to_string().to_kebab_case();
        Ok(liquid_core::model::Value::scalar(input))
    }
}

#[derive(Clone, liquid_derive::ParseFilter, liquid_derive::FilterReflection)]
#[filter(
    name = "pascal_case",
    description = "Change text to PascalCase.",
    parsed(PascalCaseFilter)
)]
pub(crate) struct PascalCaseFilterParser;

#[derive(Debug, Default, liquid_derive::Display_filter)]
#[name = "pascal_case"]
struct PascalCaseFilter;

impl Filter for PascalCaseFilter {
    fn evaluate(
        &self,
        input: &dyn ValueView,
        _runtime: &dyn Runtime,
    ) -> Result<liquid::model::Value, liquid_core::error::Error> {
        let input = input
            .as_scalar()
            .ok_or_else(|| liquid_core::error::Error::with_msg("String expected"))?;

        let input = input.into_string().to_string().to_upper_camel_case();
        Ok(liquid::model::Value::scalar(input))
    }
}

#[derive(Clone, liquid_derive::ParseFilter, liquid_derive::FilterReflection)]
#[filter(
    name = "snake_case",
    description = "Change text to snake_case.",
    parsed(SnakeCaseFilter)
)]
pub(crate) struct SnakeCaseFilterParser;

#[derive(Debug, Default, liquid_derive::Display_filter)]
#[name = "snake_case"]
struct SnakeCaseFilter;

impl Filter for SnakeCaseFilter {
    fn evaluate(
        &self,
        input: &dyn ValueView,
        _runtime: &dyn Runtime,
    ) -> Result<liquid::model::Value, liquid_core::error::Error> {
        let input = input
            .as_scalar()
            .ok_or_else(|| liquid_core::error::Error::with_msg("String expected"))?;

        let input = input.into_string().to_string().to_snake_case();
        Ok(input.to_value())
    }
}

#[derive(Clone, liquid_derive::ParseFilter, liquid_derive::FilterReflection)]
#[filter(
    name = "http_wildcard",
    description = "Add Spin HTTP wildcard suffix (/...) if needed.",
    parsed(HttpWildcardFilter)
)]
pub(crate) struct HttpWildcardFilterParser;

#[derive(Debug, Default, liquid_derive::Display_filter)]
#[name = "http_wildcard"]
struct HttpWildcardFilter;

impl Filter for HttpWildcardFilter {
    fn evaluate(
        &self,
        input: &dyn ValueView,
        _runtime: &dyn Runtime,
    ) -> Result<liquid::model::Value, liquid_core::error::Error> {
        let input = input
            .as_scalar()
            .ok_or_else(|| liquid_core::error::Error::with_msg("String expected"))?;

        let route = input.into_string().to_string();
        let wildcard_route = if route.ends_with("/...") {
            route
        } else if route.ends_with('/') {
            format!("{route}...")
        } else {
            format!("{route}/...")
        };

        Ok(wildcard_route.to_value())
    }
}
