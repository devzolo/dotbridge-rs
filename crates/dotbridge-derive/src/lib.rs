use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields};

/// Derive macro for marshaling Rust structs to/from .NET objects.
///
/// Generates `ToClr` and `FromClr` implementations that convert
/// the struct to/from a key-value dictionary representation used
/// by the CLR interop layer.
///
/// # Example
///
/// Use via the `dotbridge` crate (which re-exports this macro):
///
/// ```ignore
/// use dotbridge::{DotNetMarshal, ToClrValue, FromClrValue};
///
/// #[derive(DotNetMarshal)]
/// struct MyData {
///     name: String,
///     value: i32,
///     items: Vec<String>,
/// }
/// ```
#[proc_macro_derive(DotNetMarshal, attributes(dotnet))]
pub fn derive_dotnet_marshal(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("DotNetMarshal only supports structs with named fields"),
        },
        _ => panic!("DotNetMarshal only supports structs"),
    };

    let to_clr_fields = fields.iter().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        let field_name_str = field_name.to_string();
        quote! {
            map.insert(#field_name_str.to_string(), self.#field_name.to_clr_value());
        }
    });

    let from_clr_fields = fields.iter().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        let field_name_str = field_name.to_string();
        let ty = &f.ty;
        quote! {
            #field_name: <#ty as dotbridge::marshal::FromClrValue>::from_clr_value(
                map.get(#field_name_str)
                    .ok_or_else(|| dotbridge::DotBridgeError::MarshalError(
                        format!("missing field '{}'", #field_name_str)
                    ))?
            )?,
        }
    });

    let expanded = quote! {
        impl dotbridge::marshal::ToClrValue for #name {
            fn to_clr_value(&self) -> dotbridge::marshal::ClrValue {
                let mut map = std::collections::HashMap::new();
                #(#to_clr_fields)*
                dotbridge::marshal::ClrValue::Object(map)
            }
        }

        impl dotbridge::marshal::FromClrValue for #name {
            fn from_clr_value(value: &dotbridge::marshal::ClrValue) -> Result<Self, dotbridge::DotBridgeError> {
                match value {
                    dotbridge::marshal::ClrValue::Object(map) => {
                        Ok(Self {
                            #(#from_clr_fields)*
                        })
                    }
                    _ => Err(dotbridge::DotBridgeError::MarshalError(
                        format!("expected Object for {}, got {:?}", stringify!(#name), value)
                    )),
                }
            }
        }
    };

    TokenStream::from(expanded)
}
