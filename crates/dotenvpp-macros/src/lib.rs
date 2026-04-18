//! Procedural macros for DotenvPP.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, Data, DeriveInput, Expr, ExprArray, Fields, GenericArgument, Lit,
    PathArguments, Type,
};

/// Derive a DotenvPP schema from a Rust struct.
///
/// Supported field metadata:
///
/// ```ignore
/// #[env(name = "PORT", required, default = 8080, description = "HTTP port")]
/// #[env(secret, min_length = 32)]
/// #[env(values = ["debug", "info", "warn"])]
/// #[env(range = [1024, 65535])]
/// ```
#[proc_macro_derive(Schema, attributes(env))]
pub fn derive_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident;
    let Data::Struct(data) = input.data else {
        return syn::Error::new_spanned(ident, "Schema can only be derived for structs")
            .to_compile_error()
            .into();
    };
    let Fields::Named(fields) = data.fields else {
        return syn::Error::new_spanned(ident, "Schema requires named struct fields")
            .to_compile_error()
            .into();
    };

    let mut toml = String::new();
    toml.push_str("[meta]\n");
    toml.push_str(&format!("name = \"{}\"\n", ident));
    toml.push_str("version = \"1.0\"\n\n");

    for field in fields.named {
        let Some(field_ident) = field.ident else {
            continue;
        };
        let mut attrs = FieldAttrs {
            name: to_env_key(&field_ident.to_string()),
            required: false,
            default: None,
            secret: false,
            description: None,
            values: None,
            range: None,
            min_length: None,
            max_length: None,
        };

        for attr in field.attrs.iter().filter(|attr| attr.path().is_ident("env")) {
            if let Err(err) = parse_env_attr(attr, &mut attrs) {
                return err.to_compile_error().into();
            }
        }

        let Some(ty) = schema_type_for(&field.ty) else {
            return syn::Error::new_spanned(
                field.ty,
                "unsupported field type for DotenvPP schema derive",
            )
            .to_compile_error()
            .into();
        };

        toml.push_str("[vars.");
        toml.push_str(&attrs.name);
        toml.push_str("]\n");
        toml.push_str("type = \"");
        toml.push_str(ty);
        toml.push_str("\"\n");
        if attrs.required {
            toml.push_str("required = true\n");
        }
        if let Some(default) = attrs.default {
            toml.push_str("default = ");
            toml.push_str(&default);
            toml.push('\n');
        }
        if attrs.secret {
            toml.push_str("secret = true\n");
        }
        if let Some(description) = attrs.description {
            toml.push_str("description = \"");
            toml.push_str(&toml_escape(&description));
            toml.push_str("\"\n");
        }
        if let Some(values) = attrs.values {
            toml.push_str("values = [");
            toml.push_str(&values.join(", "));
            toml.push_str("]\n");
        }
        if let Some(range) = attrs.range {
            toml.push_str("range = [");
            toml.push_str(&range.join(", "));
            toml.push_str("]\n");
        }
        if let Some(min_length) = attrs.min_length {
            toml.push_str("min_length = ");
            toml.push_str(&min_length);
            toml.push('\n');
        }
        if let Some(max_length) = attrs.max_length {
            toml.push_str("max_length = ");
            toml.push_str(&max_length);
            toml.push('\n');
        }
        toml.push('\n');
    }

    let expanded = quote! {
        impl ::dotenvpp::ConfigSchema for #ident {
            fn schema() -> ::dotenvpp::schema::SchemaDocument {
                ::dotenvpp::schema::SchemaDocument::from_toml_str(#toml)
                    .unwrap_or_else(|err| panic!("generated DotenvPP schema is invalid: {err}"))
            }
        }
    };

    expanded.into()
}

struct FieldAttrs {
    name: String,
    required: bool,
    default: Option<String>,
    secret: bool,
    description: Option<String>,
    values: Option<Vec<String>>,
    range: Option<Vec<String>>,
    min_length: Option<String>,
    max_length: Option<String>,
}

fn parse_env_attr(attr: &syn::Attribute, attrs: &mut FieldAttrs) -> syn::Result<()> {
    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("required") {
            attrs.required = true;
            return Ok(());
        }
        if meta.path.is_ident("secret") {
            attrs.secret = true;
            return Ok(());
        }

        let value = meta.value()?;
        if meta.path.is_ident("name") {
            attrs.name = parse_string_lit(value.parse()?)?;
        } else if meta.path.is_ident("description") {
            attrs.description = Some(parse_string_lit(value.parse()?)?);
        } else if meta.path.is_ident("default") {
            attrs.default = Some(parse_toml_literal(value.parse()?)?);
        } else if meta.path.is_ident("values") {
            let array: ExprArray = value.parse()?;
            attrs.values = Some(parse_toml_array(array)?);
        } else if meta.path.is_ident("range") {
            let array: ExprArray = value.parse()?;
            attrs.range = Some(parse_toml_array(array)?);
        } else if meta.path.is_ident("min_length") {
            attrs.min_length = Some(parse_numeric_literal(value.parse()?)?);
        } else if meta.path.is_ident("max_length") {
            attrs.max_length = Some(parse_numeric_literal(value.parse()?)?);
        } else {
            return Err(meta.error("unsupported env attribute"));
        }
        Ok(())
    })
}

fn parse_string_lit(lit: Lit) -> syn::Result<String> {
    match lit {
        Lit::Str(value) => Ok(value.value()),
        other => Err(syn::Error::new_spanned(other, "expected string literal")),
    }
}

fn parse_toml_literal(expr: Expr) -> syn::Result<String> {
    match expr {
        Expr::Lit(lit) => match lit.lit {
            Lit::Str(value) => Ok(format!("\"{}\"", toml_escape(&value.value()))),
            Lit::Int(value) => Ok(value.base10_digits().to_owned()),
            Lit::Float(value) => Ok(value.base10_digits().to_owned()),
            Lit::Bool(value) => Ok(value.value.to_string()),
            other => Err(syn::Error::new_spanned(other, "unsupported literal")),
        },
        other => Err(syn::Error::new_spanned(other, "expected literal")),
    }
}

fn parse_numeric_literal(expr: Expr) -> syn::Result<String> {
    match expr {
        Expr::Lit(lit) => match lit.lit {
            Lit::Int(value) => Ok(value.base10_digits().to_owned()),
            other => Err(syn::Error::new_spanned(other, "expected integer literal")),
        },
        other => Err(syn::Error::new_spanned(other, "expected integer literal")),
    }
}

fn parse_toml_array(array: ExprArray) -> syn::Result<Vec<String>> {
    array.elems.into_iter().map(parse_toml_literal).collect::<syn::Result<Vec<_>>>()
}

fn schema_type_for(ty: &Type) -> Option<&'static str> {
    let Type::Path(path) = ty else {
        return None;
    };
    let segment = path.path.segments.last()?;
    let ident = segment.ident.to_string();

    match ident.as_str() {
        "String" | "str" => Some("string"),
        "bool" => Some("bool"),
        "i32" => Some("i32"),
        "i64" => Some("i64"),
        "u16" => Some("u16"),
        "u32" => Some("u32"),
        "u64" => Some("u64"),
        "f64" => Some("f64"),
        "Url" => Some("url"),
        "IpAddr" | "Ipv4Addr" | "Ipv6Addr" => Some("ip"),
        "PathBuf" => Some("path"),
        "Vec" => vec_type_for(&segment.arguments),
        _ => None,
    }
}

fn vec_type_for(args: &PathArguments) -> Option<&'static str> {
    let PathArguments::AngleBracketed(args) = args else {
        return None;
    };
    let Some(GenericArgument::Type(Type::Path(inner))) = args.args.first() else {
        return None;
    };
    let ident = inner.path.segments.last()?.ident.to_string();
    match ident.as_str() {
        "String" => Some("string[]"),
        "i32" => Some("i32[]"),
        _ => None,
    }
}

fn to_env_key(field: &str) -> String {
    field.to_ascii_uppercase()
}

fn toml_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
