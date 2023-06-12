extern crate proc_macro;
extern crate quick_xml;

use quick_xml::events::{BytesCData, BytesDecl, BytesEnd, BytesStart, Event};
use quick_xml::writer::Writer;
use std::collections::HashSet;
use std::io::Cursor;
use std::str;

use darling::{
    ast::{self},
    FromDeriveInput, FromField, FromMeta, ToTokens,
};
use proc_macro2::{self, TokenStream};
use quote::quote;
use syn::parse_macro_input;

/// A speaking volume. Deriving `FromMeta` will cause this to be usable
/// as a string value for a meta-item key.
#[derive(Debug, Clone, Copy, FromMeta, PartialEq)]
#[darling(default)]
enum Causality {
    Ignore,
    Output,
    Input,
    Parameter,
    Independent,
}
impl ToString for Causality {
    fn to_string(&self) -> String {
        match self {
            Causality::Ignore => "ignore",
            Causality::Output => "output",
            Causality::Input => "input",
            Causality::Parameter => "parameter",
            Causality::Independent => "independent",
        }
        .to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord, FromMeta)]
#[darling(default)]
struct VarRefId(u64);

#[derive(Debug, Clone, FromMeta)]
#[darling(default)]
struct Description(String);

#[derive(Debug, Clone, FromMeta)]
#[darling(default)]
struct Unit(String);

impl Default for Causality {
    fn default() -> Self {
        Causality::Ignore
    }
}

impl Default for VarRefId {
    fn default() -> VarRefId {
        VarRefId(0)
    }
}

impl Default for Description {
    fn default() -> Description {
        Description("".to_string())
    }
}

impl Default for Unit {
    fn default() -> Unit {
        Unit("".to_string())
    }
}

/// Support parsing from a full derive input. Unlike FromMeta, this isn't
/// composable; each darling-dependent crate should have its own struct to handle
/// when its trait is derived.
#[derive(Debug, FromDeriveInput)]
// This line says that we want to process all attributes declared with `my_trait`,
// and that darling should panic if this receiver is given an enum.
#[darling(attributes(fmi_model_struct), supports(struct_any))]
struct FmiModelStructReceiver {
    /// The struct ident.
    ident: syn::Ident,

    /// The type's generics. You'll need these any time your trait is expected
    /// to work with types that declare generics.
    generics: syn::Generics,

    /// Receives the body of the struct or enum. We don't care about
    /// struct fields because we previously told darling we only accept structs.
    data: ast::Data<(), FmiVariableReceiver>,
}

#[derive(Debug, FromField)]
#[darling(attributes(fmi_variable))]
struct FmiVariableReceiver {
    /// Get the ident of the field. For fields in tuple or newtype structs or
    /// enum bodies, this can be `None`.
    ident: Option<syn::Ident>,

    /// This magic field name pulls the type from the input.
    ty: syn::Type,

    /// We declare this as an `Option` so that during tokenization we can write
    /// `field.volume.unwrap_or(derive_input.volume)` to facilitate field-level
    /// overrides of struct-level settings.
    ///
    /// Because this field is an `Option`, we don't need to include `#[darling(default)]`
    #[darling(default)]
    causality: Causality,
    #[darling(default)]
    id: Option<VarRefId>,

    #[darling(default)]
    description: Description,

    #[darling(default)]
    unit: Unit,
}

impl ToTokens for FmiModelStructReceiver {
    fn to_tokens(&self, tokens: &mut TokenStream) {}
}

#[proc_macro_derive(FmiModelStructDerive, attributes(fmi_model_struct, fmi_variable))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input);
    let fmi_model = FmiModelStructReceiver::from_derive_input(&input).expect("Wrong options");

    let model_name = fmi_model.ident;

    // Get all fields
    let mut fields = fmi_model
        .data
        .take_struct()
        .expect("Should never be enum")
        .fields;

    let mut fields = fields
        .iter_mut()
        .filter(|x| x.causality != Causality::Ignore)
        .collect::<Vec<_>>();

    let enum_name: String = format!("{}Variables", model_name);
    let value_reference_enum = syn::Ident::new(&enum_name, model_name.span());

    // // Assign all value reference ids by finding the max id that occurs and making sure all enum fields are above that value.
    let max_value_ref_defined = fields.iter().max_by(|x, y| {
        x.id.unwrap_or(VarRefId(0))
            .cmp(&y.id.unwrap_or(VarRefId(0)))
    });

    let mut last_value_ref = match max_value_ref_defined {
        Some(f) => f.id.unwrap_or(VarRefId(0)).0 + 1,
        None => 0,
    };

    for field in fields.iter_mut() {
        let var_ref_id = field.id;
        match var_ref_id {
            Some(VarRefId(id)) => (),
            None => {
                field.id = Some(VarRefId(last_value_ref));
                last_value_ref += 1;
            }
        }
    }

    let enum_fields: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
            let name = f.ident.as_ref().unwrap();
            quote! { #name }
        })
        .collect::<Vec<_>>();

    let field_vrs: Vec<TokenStream> = fields
        .iter()
        .map(|f: &&mut FmiVariableReceiver| match f.id {
            Some(VarRefId(id)) => quote! { #id },
            None => quote! { 0 },
        })
        .collect::<Vec<_>>();

    let real_fields_idents = fields
        .iter()
        .filter(|x| x.ty == syn::parse_str::<syn::Type>("f64").unwrap())
        .map(|f| {
            let name = f.ident.as_ref().unwrap();
            quote! { #name }
        })
        .collect::<Vec<_>>();

    let bool_fields_idents: Vec<TokenStream> = fields
        .iter()
        .filter(|x| x.ty == syn::parse_str::<syn::Type>("bool").unwrap())
        .map(|f| {
            let name = f.ident.as_ref().unwrap();
            quote! { #name }
        })
        .collect::<Vec<_>>();

    let integer_fields_idents: Vec<TokenStream> = fields
        .iter()
        .filter(|x| x.ty == syn::parse_str::<syn::Type>("i64").unwrap())
        .map(|f| {
            let name = f.ident.as_ref().unwrap();
            quote! { #name }
        })
        .collect::<Vec<_>>();

    // Create XML writer code
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::<u8>::new()), ' ' as u8, 4);

    // Write XML Header
    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("ISO-8859-1"), None)));

    // Create the fmiModelDescription header. For example:
    //     <fmiModelDescription
    //   fmiVersion               = "2.0"
    //   modelName                = "Sine"
    //   guid                     = "{21d9f232-b090-4c79-933f-33da939b5934}"
    //   description              = "Model Sine Wave"
    //   variableNamingConvention = "structured"
    //   numberOfEventIndicators  = "0">
    writer.write_indent();

    let mut fmi_model_description = BytesStart::new("fmiModelDescription");
    // copy existing attributes, adds a new my-key="some value" attribute
    fmi_model_description.push_attribute(("fmiVersion", "2.0"));
    fmi_model_description.push_attribute(("modelName", model_name.to_string().as_str()));
    fmi_model_description.push_attribute(("guid", "{21d9f232-b090-4c79-933f-33da939b5934}"));
    fmi_model_description.push_attribute(("description", model_name.to_string().as_str()));
    // writes the event to the writer
    writer.write_event(Event::Start(fmi_model_description));

    // TODO(cw): Add all the required elements, and make them configurable using attributes
    let mut cosimulation_elements = BytesStart::new("CoSimulation");
    // copy existing attributes, adds a new my-key="some value" attribute
    cosimulation_elements.push_attribute(("modelIdentifier", model_name.to_string().as_str()));
    cosimulation_elements.push_attribute(("canHandleVariableCommunicationStepSize", "true"));
    cosimulation_elements.push_attribute(("canGetAndSetFMUstate", "true"));
    cosimulation_elements.push_attribute(("canSerializeFMUstate", "false"));
    cosimulation_elements.push_attribute(("providesDirectionalDerivative", "false"));
    cosimulation_elements.push_attribute(("canInterpolateInputs", "false"));
    writer.write_event(Event::Start(cosimulation_elements));
    let cosimulation_elements = BytesEnd::new("CoSimulation");
    writer.write_event(Event::End(cosimulation_elements));

    writer.write_indent();

    writer.write_event(Event::Start(BytesStart::new("UnitDefinitions")));

    // get all unique units and then add each unit to the model definition
    let unique_units = fields
        .iter()
        .map(|field| &field.unit.0)
        .collect::<HashSet<_>>()
        .into_iter();
    unique_units.for_each(|unit| {
        let mut elem = BytesStart::new("Unit");
        elem.push_attribute(("name", unit.as_str()));
        writer.write_event(Event::Empty(elem));
    });

    writer.write_event(Event::End(BytesEnd::new("UnitDefinitions")));


    // Add default experiment, but don't make it configurable
    // TODO(cw): Make this configurable through an attribute
    let mut default_experiment = BytesStart::new("DefaultExperiment");
    default_experiment.push_attribute(("startTime", "0.0"));
    default_experiment.push_attribute(("stopTime", "1.0"));
    default_experiment.push_attribute(("tolerance", "0.0001"));
    default_experiment.push_attribute(("stepSize", "0.1"));
    writer.write_event(Event::Empty(default_experiment));


    // Populate Model Variables
    writer.write_event(Event::Start(BytesStart::new("ModelVariables")));

    // Add all variables
    // Start with Scalar Variables

    // Add all real variables
    let real_fields: Vec<&&mut FmiVariableReceiver> = fields
        .iter()
        .filter(|x| x.ty == syn::parse_str::<syn::Type>("f64").unwrap())
        .collect::<Vec<_>>();

    for field in real_fields.iter() {
        let mut event = BytesStart::new("ScalarVariable");
        event.push_attribute(("name", field.ident.as_ref().unwrap().to_string().as_str()));
        event.push_attribute(("valueReference", field.id.unwrap().0.to_string().as_str()));
        event.push_attribute(("description", field.description.0.as_str()));
        event.push_attribute(("causality", field.causality.to_string().as_str()));
        if (field.causality == Causality::Parameter) {
            event.push_attribute(("variability", "fixed"));
        } else {
            event.push_attribute(("variability", "continuous"));
        }

        writer.write_event(Event::Start(event));
        
        let mut event = BytesStart::new("Real");
        event.push_attribute(("unit", field.unit.0.as_str()));
    
        writer.write_event(Event::Empty(event));
        writer.write_event(Event::End(BytesEnd::new("ScalarVariable")));
    }

    let integer_fields: Vec<&&mut FmiVariableReceiver> = fields
        .iter()
        .filter(|x| x.ty == syn::parse_str::<syn::Type>("i64").unwrap())
        .collect::<Vec<_>>();


    for field in integer_fields.iter() {
        let mut event = BytesStart::new("ScalarVariable");
        event.push_attribute(("name", field.ident.as_ref().unwrap().to_string().as_str()));
        event.push_attribute(("valueReference", field.id.unwrap().0.to_string().as_str()));
        event.push_attribute(("description", field.description.0.as_str()));
        event.push_attribute(("causality", field.causality.to_string().as_str()));
        if (field.causality == Causality::Parameter) {
            event.push_attribute(("variability", "fixed"));
        } else {
            event.push_attribute(("variability", "continuous"));
        }

        writer.write_event(Event::Start(event));
        
        let mut event = BytesStart::new("Integer");
        event.push_attribute(("unit", field.unit.0.as_str()));
    
        writer.write_event(Event::Empty(event));
        writer.write_event(Event::End(BytesEnd::new("ScalarVariable")));
    }

    let bool_fields: Vec<&&mut FmiVariableReceiver> = fields
        .iter()
        .filter(|x| x.ty == syn::parse_str::<syn::Type>("bool").unwrap())
        .collect::<Vec<_>>();


    for field in bool_fields.iter() {
        let mut event = BytesStart::new("ScalarVariable");
        event.push_attribute(("name", field.ident.as_ref().unwrap().to_string().as_str()));
        event.push_attribute(("valueReference", field.id.unwrap().0.to_string().as_str()));
        event.push_attribute(("description", field.description.0.as_str()));
        event.push_attribute(("causality", field.causality.to_string().as_str()));
        if (field.causality == Causality::Parameter) {
            event.push_attribute(("variability", "fixed"));
        } else {
            event.push_attribute(("variability", "continuous"));
        }

        writer.write_event(Event::Start(event));
        
        let mut event = BytesStart::new("Integer");
        event.push_attribute(("unit", field.unit.0.as_str()));
    
        writer.write_event(Event::Empty(event));
        writer.write_event(Event::End(BytesEnd::new("ScalarVariable")));
    }

    // TODO(cw): Support non scalar variables
    writer.write_event(Event::End(BytesEnd::new("ModelVariables")));
    
    writer.write_event(Event::Start(BytesStart::new("ModelStructure")));

    writer.write_event(Event::Start(BytesStart::new("Outputs")));

    for (index, field) in fields.iter().enumerate() {
        if field.causality ==Causality::Output {
            let mut event = BytesStart::new("Unknown");
            event.push_attribute(("index", (index + 1).to_string().as_str()));
            event.push_attribute(("dependencies", ""));
            writer.write_event(Event::Empty(event));    
        }        
    }

    writer.write_event(Event::End(BytesEnd::new("Outputs")));

    writer.write_event(Event::End(BytesEnd::new("ModelStructure")));

    let mut fmi_model_description = BytesEnd::new("fmiModelDescription");
    writer.write_event(Event::End(fmi_model_description));

    let result = writer.into_inner().into_inner();
    let val = String::from_utf8(result).expect("Found invalid UTF-8");

    // Create the output code
    let output = quote! {
        // Create the value reference enum
        #[derive(Copy, Clone)]
        enum #value_reference_enum {
            #(#enum_fields),*
        }

        impl #value_reference_enum {
            fn to_underlying(self: &Self) -> Option<u64> {
                match self {
                    #(#value_reference_enum::#enum_fields => Some(#field_vrs),)*
                    _ => None,
                }
            }

            fn from_underlying(x: u64) -> Option<Self> {
                match x {
                    #(#field_vrs => Some(#value_reference_enum::#enum_fields),)*
                    _ => None,
                }
            }
        }

        impl FmiModelStructDerive for #model_name {
            fn get_real_by_value_reference(self: &Self, value_reference: u64) -> Option<f64> {
                let vr = #value_reference_enum::from_underlying(value_reference)?;
                match vr {
                    #(#value_reference_enum::#real_fields_idents => Some(self.#real_fields_idents),)*
                    _ => None,
                }
            }

            fn set_real_by_value_reference(self: &mut Self, value_reference: u64, value: f64) -> Result<(),FMIErrors> {
                let vr = #value_reference_enum::from_underlying(value_reference).ok_or(FMIErrors::InvalidValueReference)?;
                match vr {
                    #(#value_reference_enum::#real_fields_idents => {self.#real_fields_idents = value; Ok(())},)*
                    _ => Err(FMIErrors::InvalidValueReference),
                }
            }

            fn get_bool_by_value_reference(self: &Self, value_reference: u64) -> Option<bool> {
                let vr = #value_reference_enum::from_underlying(value_reference)?;
                match vr {
                    #(#value_reference_enum::#bool_fields_idents => Some(self.#bool_fields_idents),)*
                    _ => None,
                }
            }

            fn set_bool_by_value_reference(self: &mut Self, value_reference: u64, value: bool) -> Result<(),FMIErrors> {
                let vr = #value_reference_enum::from_underlying(value_reference).ok_or(FMIErrors::InvalidValueReference)?;
                match vr {
                    #(#value_reference_enum::#bool_fields_idents => {self.#bool_fields_idents = value; Ok(())},)*
                    _ => Err(FMIErrors::InvalidValueReference),
                }
            }


            fn get_integer_by_value_reference(self: &Self, value_reference: u64) -> Option<i64> {
                let vr = #value_reference_enum::from_underlying(value_reference)?;
                match vr {
                    #(#value_reference_enum::#integer_fields_idents => Some(self.#integer_fields_idents),)*
                    _ => None,
                }
            }

            fn set_integer_by_value_reference(self: &mut Self, value_reference: u64, value: i64) -> Result<(),FMIErrors> {
                let vr = #value_reference_enum::from_underlying(value_reference).ok_or(FMIErrors::InvalidValueReference)?;
                match vr {
                    #(#value_reference_enum::#integer_fields_idents => {self.#integer_fields_idents = value; Ok(())},)*
                    _ => Err(FMIErrors::InvalidValueReference),
                }
            }

            fn to_model_description_xml() -> String {
                #val.to_string()
            }
        }
    };
    output.into()
}
