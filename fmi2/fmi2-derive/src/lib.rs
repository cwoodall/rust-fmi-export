extern crate proc_macro;
extern crate quick_xml;
extern crate handlebars;
extern crate serde_json;
extern crate serde;
use serde::{Serialize, Deserialize};

use handlebars::Handlebars;

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
struct GUID(String);

#[derive(Debug, Clone, FromMeta)]
#[darling(default)]
struct Unit(String);


#[derive(Debug, Clone, FromMeta)]
#[darling(default)]
struct ExportFMI(bool);

impl Default for ExportFMI {
    fn default() -> Self {
        ExportFMI(false)
    }
}

impl Default for Causality {
    fn default() -> Self {
        Causality::Ignore
    }
}

impl Default for GUID {
    fn default() -> Self {
        GUID(format!("{{{}}}", uuid::Uuid::new_v4().to_string()))
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
// This line says that we want to process all attributes declared with `fmi_model_struct`,
// and that darling should panic if this receiver is given an enum.
#[darling(attributes(fmi_model), supports(struct_any))]
struct FmiModelStructReceiver {
    /// The struct ident.
    ident: syn::Ident,

    /// The type's generics. You'll need these any time your trait is expected
    /// to work with types that declare generics.
    generics: syn::Generics,

    /// Receives the body of the struct or enum. We don't care about
    /// struct fields because we previously told darling we only accept structs.
    data: ast::Data<(), FmiVariableReceiver>,

    #[darling(default)]
    description: Description,

    #[darling(default)]
    guid: GUID,

    #[darling(default)]
    export: ExportFMI,
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

    starting_value: Option<f64>,
}

impl ToTokens for FmiModelStructReceiver {
    fn to_tokens(&self, tokens: &mut TokenStream) {}
}

#[proc_macro_derive(FmiModelStructDerive, attributes(fmi_model, fmi_variable))]
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
    fmi_model_description.push_attribute(("guid", fmi_model.guid.0.as_str()));
    fmi_model_description.push_attribute(("description", fmi_model.description.0.as_str()));
    // writes the event to the writer
    writer.write_event(Event::Start(fmi_model_description));

    writer.write_indent();

    // TODO(cw): Add all the required elements, and make them configurable using attributes
    let mut cosimulation_elements = BytesStart::new("CoSimulation");
    // copy existing attributes, adds a new my-key="some value" attribute
    cosimulation_elements.push_attribute(("modelIdentifier", model_name.to_string().as_str()));
    cosimulation_elements.push_attribute(("canHandleVariableCommunicationStepSize", "false"));
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
    writer.write_indent();

    // Add default experiment, but don't make it configurable
    // TODO(cw): Make this configurable through an attribute
    let mut default_experiment = BytesStart::new("DefaultExperiment");
    default_experiment.push_attribute(("startTime", "0.0"));
    default_experiment.push_attribute(("stopTime", "1.0"));
    default_experiment.push_attribute(("tolerance", "0.0001"));
    default_experiment.push_attribute(("stepSize", "0.01"));
    writer.write_event(Event::Empty(default_experiment));

    writer.write_indent();

    // Populate Model Variables
    writer.write_event(Event::Start(BytesStart::new("ModelVariables")));

    for field in fields.iter() {
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

        let type_string = match &field.ty {
            syn::Type::Path(t) => {
                if t.path.is_ident("f64") {
                    "Real"
                } else if t.path.is_ident("i64") {
                    "Integer"
                } else if t.path.is_ident("bool") {
                    "Boolean"
                } else {
                    panic!("Unsupported type");
                }
            }
            _ => {
                panic!("Unsupported type");
            }
        };

        let mut event = BytesStart::new(type_string);

        if type_string == "Real" {
            event.push_attribute(("unit", field.unit.0.as_str()));
        }

        // TODO: Fix start value settings
        if field.causality == Causality::Parameter || field.causality == Causality::Input {
            let mut start_value_tag = String::from("{{");
            start_value_tag.push_str(field.ident.as_ref().unwrap().to_string().as_str());
            start_value_tag.push_str("}}");
            event.push_attribute(("start", start_value_tag.as_str()));
        }

        writer.write_event(Event::Empty(event)).expect("could not write event");
        writer.write_event(Event::End(BytesEnd::new("ScalarVariable"))).expect("could not write scalar variable");
        writer.write_indent().expect("Could not write indent");
    }

    // TODO(cw): Support non scalar variables
    writer.write_event(Event::End(BytesEnd::new("ModelVariables")));

    writer.write_event(Event::Start(BytesStart::new("ModelStructure")));

    writer.write_event(Event::Start(BytesStart::new("Outputs")));

    for (index, field) in fields.iter().enumerate() {
        if field.causality == Causality::Output {
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
    let xml_model_description = String::from_utf8(result).expect("Found invalid UTF-8");

    // Create the output code
    let guid = fmi_model.guid.0;
    let description = fmi_model.description.0;
    let model_name_str = model_name.to_string();
    let mut output = quote! {
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
                let default = #model_name::default();
                let json = serde_json::to_value(default);
                Handlebars::new().render_template(
                    #xml_model_description,
                    &json.expect("Could not render template")
                ).expect("Could not render template")
            }

            fn guid() -> &'static str {
                #guid
            }

            fn description() -> &'static str {
                #description
            }

            fn model_name() -> &'static str {
                #model_name_str
            }
        }
        
    };

    if fmi_model.export.0 {
        output.extend(quote!{
        #[no_mangle]
        pub extern "C" fn fmi2GetTypesPlatform() -> *const c_char {
            "default".as_ptr() as *const c_char
        }

        #[no_mangle]
        pub extern "C" fn fmi2GetVersion() -> *const c_char {
            "2.0".as_ptr() as *const c_char
        }


        #[no_mangle]
        pub extern "C" fn fmi2SetDebugLogging(
            c: *mut c_void,
            loggingOn: fmi2Boolean,
            _nCategories: usize,
            _categories: *const fmi2String,
        ) -> fmi2Status {
            assert!(
                std::ptr::null() != c as *mut c_void,
                "fmi2FreeInstance: Null pointer passed"
            );

            let x: &mut #model_name = unsafe { &mut *(c as *mut #model_name) };

            // https://stackoverflow.com/questions/26117197/create-interface-to-c-function-pointers-in-rust
            // Create a c-string from a rust string and report it

            // match x.functions.logger {
            //     Some(f) => {
            //         // TODO(cw): Create a wrapper around calling this function (and other callback functions)
            //         let category = CString::new("log").unwrap();
            //         let message = CString::new("fmi2SetDebugLogging: loggingOn = %d").unwrap();
            //         unsafe {
            //             f(
            //                 c,
            //                 x.instanceName,
            //                 fmi2Status_fmi2OK,
            //                 category.into_raw(),
            //                 message.into_raw(),
            //                 loggingOn,
            //             );
            //         }
            //     }
            //     None => {
            //         ();
            //     }
            // }

            match x.state {
                ModelState::Error => fmi2Status_fmi2Error,
                _ => {
                    x.loggingOn = loggingOn;
                    fmi2Status_fmi2OK
                }
            }
        }

        #[no_mangle]
        pub extern "C" fn fmi2Instantiate(
            instanceName: fmi2String,
            fmuType: fmi2Type,
            fmuGUID: fmi2String,
            _fmuResourceLocation: fmi2String,
            functions: fmi2CallbackFunctions,
            _visible: fmi2Boolean,
            loggingOn: fmi2Boolean,
        ) -> *mut #model_name {
            assert!(
                std::ptr::null() != instanceName as *mut c_void,
                "fmi2Instantiate: Null pointer passed"
            );

            assert!(
                std::ptr::null() != fmuGUID as *mut c_void,
                "fmi2Instantiate: Null pointer passed"
            );

            let guid = unsafe { CStr::from_ptr(fmuGUID as *mut c_char) };

            println!("fmi2Instantiate: GUID = {}", guid.to_str().unwrap());
            assert!(
                guid.to_str().unwrap() == #model_name::guid(),
                "fmi2Instantiate: Invalid GUID"
            );

            let mut model: Box<#model_name> = Box::new(
                #model_name::instantiate()
                    .expect("fmi2Instantiate: Failed to instantiate model")
            );

            if model.loggingOn == fmi2True as fmi2Boolean {
                // // https://stackoverflow.com/questions/26117197/create-interface-to-c-function-pointers-in-rust
                // // Create a c-string from a rust string and report it
                let category = CString::new("log").unwrap();
                let message = CString::new("fmi2Instantiate: instanceName = %s").unwrap();
                // unsafe {
                //     // model.functions.logger.unwrap()(
                //     //     model.as_mut() as *mut #model_name as *mut c_void,
                //     //     instanceName,
                //     //     fmi2Status_fmi2OK,
                //     //     category.into_raw(),
                //     //     message.into_raw(),
                //     //     instanceName,
                //     // );
                // }
            }

            Box::into_raw(model) as *mut #model_name
        }

        #[no_mangle]
        pub extern "C" fn fmi2FreeInstance(c: fmi2Component) -> () {
            // let model = #model_name::from_c_ptr(c as *mut c_void)
            //     .expect("fmi2FreeInstance: Failed to get model from c_ptr");

            // https://stackoverflow.com/questions/26117197/create-interface-to-c-function-pointers-in-rust
            // Create a c-string from a rust string and report it
            // if model.loggingOn == fmi2True as fmi2Boolean {
            //     let category = CString::new("log").unwrap();
            //     let message = CString::new("fmi2FreeInstance: instanceName = %s").unwrap();
            //     unsafe {
            //         // x.functions.logger.unwrap()(
            //         //     c,
            //         //     x.instanceName,
            //         //     fmi2Status_fmi2OK,
            //         //     category.into_raw(),
            //         //     message.into_raw(),
            //         //     x.instanceName,
            //         // );
            //     }
            // }

        //     match x.functions.freeMemory {
        //         None => {
        //             ();
        //         }
        //         Some(f) => unsafe {
        //             f(c);
        //         },
        //     }
        // }
            // TODO: Figure out a way to free this.  It's a Box, so it should be freed automatically, but it's not.
            let model = unsafe { Box::from_raw(c as *mut #model_name) };
        }

        #[no_mangle]
        pub extern "C" fn fmi2SetupExperiment(
            c: fmi2Component,
            _toleranceDefined: fmi2Boolean,
            _tolerance: fmi2Real,
            _startTime: fmi2Real,
            _stopTimeDefined: fmi2Boolean,
            _stopTime: fmi2Real,
        ) -> fmi2Status {
            assert!(
                std::ptr::null() != c as *mut c_void,
                "fmi2FreeInstance: Null pointer passed"
            );

            // let x: &mut #model_name = unsafe { &mut *(c as *mut #model_name) };
            // println!("{:?}", x);

            // x.startTime = startTime;
            // x.stopTime = if stopTimeDefined == fmi2True as fmi2Boolean {
            //     stopTime
            // } else {
            //     0.0
            // };

            // x.stopTimeDefined = stopTimeDefined;

            fmi2Status_fmi2OK
        }

        #[no_mangle]
        pub extern "C" fn fmi2EnterInitializationMode(c: fmi2Component) -> fmi2Status {
            let model = #model_name::from_c_ptr(c as *mut c_void);

            match model {
                Ok(x) => {
                    x.state = ModelState::Initialized;
                    fmi2Status_fmi2OK
                }
                Err(_) => fmi2Status_fmi2Error,
            }
        }

        #[no_mangle]
        pub extern "C" fn fmi2ExitInitializationMode(c: fmi2Component) -> fmi2Status {
            match #model_name::from_c_ptr(c as *mut c_void) {
                Ok(x) => {
                    x.state = ModelState::Instantiated;
                    fmi2Status_fmi2OK
                }
                Err(_) => fmi2Status_fmi2Error,
            }

        }


        #[no_mangle]
        pub extern "C" fn fmi2Terminate(c: fmi2Component) -> fmi2Status {
            match #model_name::from_c_ptr(c as *mut c_void) {
                Ok(x) => {
                    x.state = ModelState::Terminated;
                    fmi2Status_fmi2OK
                }
                Err(_) => fmi2Status_fmi2Error,
            }
        }

        #[no_mangle]
        pub extern "C" fn fmi2Reset(c: fmi2Component) -> fmi2Status {
            match #model_name::from_c_ptr(c as *mut c_void) {
                Ok(x) => {
                    *x = #model_name::default();
                    fmi2Status_fmi2OK
                }
                Err(_) => fmi2Status_fmi2Error,
            }
        }

        #[no_mangle]
        pub extern "C" fn fmi2GetReal(
            c: fmi2Component,
            vr: *const fmi2ValueReference,
            nvr: usize,
            value: *mut fmi2Real,
        ) -> fmi2Status {
            assert!(
                std::ptr::null() != c as *mut c_void,
                "fmi2GetReal: Null pointer passed"
            );

            assert!(
                value != std::ptr::null_mut(),
                "fmi2GetReal: Null pointer passed"
            );

            assert!(
                vr != std::ptr::null_mut(),
                "fmi2GetReal: Null pointer passed"
            );

            match #model_name::from_c_ptr(c as *mut c_void) {
                Ok(x) => {
                    if nvr > 0 {
                        let value_slice: &mut [f64] = unsafe { std::slice::from_raw_parts_mut(value, nvr) };
                        let reference_slice: &[fmi2ValueReference] = unsafe { std::slice::from_raw_parts(vr, nvr) };

                        for i in 0..nvr {
                            value_slice[i] = x.get_real_by_value_reference(reference_slice[i] as u64).expect("fmi2GetReal: Unknown value reference");
                        }

                        fmi2Status_fmi2OK
                    } else {
                        fmi2Status_fmi2Error
                    }
                }
                Err(_) => fmi2Status_fmi2Error,
            }
        }

        #[no_mangle]
        pub extern "C" fn fmi2GetInteger(
            c: fmi2Component,
            vr: *const fmi2ValueReference,
            nvr: usize,
            value: *mut fmi2Integer,
        ) -> fmi2Status {
            assert!(
                std::ptr::null() != c as *mut c_void,
                "fmi2GetInteger: Null pointer passed"
            );

            assert!(
                value != std::ptr::null_mut(),
                "fmi2GetInteger: Null pointer passed"
            );

            assert!(
                vr != std::ptr::null_mut(),
                "fmi2GetInteger: Null pointer passed"
            );

            match #model_name::from_c_ptr(c as *mut c_void) {
                Ok(x) => {
                    if nvr > 0 {
                        let value_slice: &mut [i32] = unsafe { std::slice::from_raw_parts_mut(value, nvr) };
                        let reference_slice: &[fmi2ValueReference] = unsafe { std::slice::from_raw_parts(vr, nvr) };

                        for i in 0..nvr {
                            value_slice[i] = x.get_integer_by_value_reference(reference_slice[i] as u64).expect("fmi2GetInteger: Unknown value reference") as i32;
                        }

                        fmi2Status_fmi2OK
                    } else {
                        fmi2Status_fmi2Error
                    }
                }
                Err(_) => fmi2Status_fmi2Error,
            }
        }

        #[no_mangle]
        pub extern "C" fn fmi2GetBoolean(
            c: fmi2Component,
            vr: *const fmi2ValueReference,
            nvr: usize,
            value: *mut fmi2Boolean,
        ) -> fmi2Status {
            assert!(
                std::ptr::null() != c as *mut c_void,
                "fmi2GetInteger: Null pointer passed"
            );

            assert!(
                value != std::ptr::null_mut(),
                "fmi2GetInteger: Null pointer passed"
            );

            assert!(
                vr != std::ptr::null_mut(),
                "fmi2GetInteger: Null pointer passed"
            );

            match #model_name::from_c_ptr(c as *mut c_void) {
                Ok(x) => {
                    if nvr > 0 {
                        let value_slice: &mut [fmi2Boolean] = unsafe { std::slice::from_raw_parts_mut(value, nvr) };
                        let reference_slice: &[fmi2ValueReference] = unsafe { std::slice::from_raw_parts(vr, nvr) };

                        for i in 0..nvr {
                            value_slice[i] = x.get_bool_by_value_reference(reference_slice[i] as u64).expect("fmi2GetInteger: Unknown value reference") as fmi2Boolean;
                        }

                        fmi2Status_fmi2OK
                    } else {
                        fmi2Status_fmi2Error
                    }
                }
                Err(_) => fmi2Status_fmi2Error,
            }
        }

        #[no_mangle]
        pub extern "C" fn fmi2GetString(
            _c: fmi2Component,
            _vr: *const fmi2ValueReference,
            _nvr: usize,
            _value: *mut fmi2String,
        ) -> fmi2Status {
            fmi2Status_fmi2Error
        }

        #[no_mangle]
        pub extern "C" fn fmi2SetReal(
            c: fmi2Component,
            vr: *const fmi2ValueReference,
            nvr: usize,
            value: *const fmi2Real,
        ) -> fmi2Status {
            assert!(
                std::ptr::null() != c as *mut c_void,
                "fmi2SetReal: Null pointer passed"
            );

            assert!(
                value != std::ptr::null_mut(),
                "fmi2SetReal: Null pointer passed"
            );

            assert!(
                vr != std::ptr::null_mut(),
                "fmi2SetReal: Null pointer passed"
            );

            match #model_name::from_c_ptr(c as *mut c_void) {
                Ok(x) => {
                    if nvr > 0 {
                        let value_slice: &[f64] = unsafe { std::slice::from_raw_parts(value, nvr) };
                        let reference_slice: &[fmi2ValueReference] = unsafe { std::slice::from_raw_parts(vr, nvr) };
                
                        for i in 0..nvr {
                            if x.set_real_by_value_reference(reference_slice[i] as u64, value_slice[i]).is_err() {
                                println!(
                                    "fmi2GetReal: Unknown value reference: {}",
                                    reference_slice[i]
                                );
                                return fmi2Status_fmi2Error;

                            }
                        }
                        fmi2Status_fmi2OK
                    } else {
                        fmi2Status_fmi2Error
                    }
                }
                Err(_) => fmi2Status_fmi2Error,
            }
            // print!("fmi2SetReal: ");
            // println!("{:?}", x);
            // println!("{:?}, {:?}, {:?}", vr, nvr, value);

        }

        #[no_mangle]
        pub extern "C" fn fmi2SetInteger(
            c: fmi2Component,
            vr: *const fmi2ValueReference,
            nvr: usize,
            value: *const fmi2Integer,
        ) -> fmi2Status {
            assert!(
                std::ptr::null() != c as *mut c_void,
                "fmi2SetInteger: Null pointer passed"
            );

            assert!(
                value != std::ptr::null_mut(),
                "fmi2SetInteger: Null pointer passed"
            );

            assert!(
                vr != std::ptr::null_mut(),
                "fmi2SetInteger: Null pointer passed"
            );

            match #model_name::from_c_ptr(c as *mut c_void) {
                Ok(x) => {
                    if nvr > 0 {
                        let value_slice: &[fmi2Integer] = unsafe { std::slice::from_raw_parts(value, nvr) };
                        let reference_slice: &[fmi2ValueReference] = unsafe { std::slice::from_raw_parts(vr, nvr) };
                
                        for i in 0..nvr {
                            if x.set_integer_by_value_reference(reference_slice[i] as u64, value_slice[i] as i64).is_err() {
                                println!(
                                    "fmi2GetReal: Unknown value reference: {}",
                                    reference_slice[i]
                                );
                                return fmi2Status_fmi2Error;

                            }
                        }
                        fmi2Status_fmi2OK
                    } else {
                        fmi2Status_fmi2Error
                    }
                }
                Err(_) => fmi2Status_fmi2Error,
            }
            // print!("fmi2SetReal: ");
            // println!("{:?}", x);
            // println!("{:?}, {:?}, {:?}", vr, nvr, value);

        }

        #[no_mangle]
        pub extern "C" fn fmi2SetBoolean(
            c: fmi2Component,
            vr: *const fmi2ValueReference,
            nvr: usize,
            value: *const fmi2Boolean,
        ) -> fmi2Status {
            assert!(
                std::ptr::null() != c as *mut c_void,
                "fmi2SetBoolean: Null pointer passed"
            );

            assert!(
                value != std::ptr::null_mut(),
                "fmi2SetBoolean: Null pointer passed"
            );

            assert!(
                vr != std::ptr::null_mut(),
                "fmi2SetBoolean: Null pointer passed"
            );

            match #model_name::from_c_ptr(c as *mut c_void) {
                Ok(x) => {
                    if nvr > 0 {
                        let value_slice: &[fmi2Integer] = unsafe { std::slice::from_raw_parts(value, nvr) };
                        let reference_slice: &[fmi2ValueReference] = unsafe { std::slice::from_raw_parts(vr, nvr) };
                
                        for i in 0..nvr {
                            let val = value_slice[i] == fmi2True as fmi2Boolean;
                            if x.set_bool_by_value_reference(reference_slice[i] as u64, val).is_err() {
                                println!(
                                    "fmi2GetReal: Unknown value reference: {}",
                                    reference_slice[i]
                                );
                                return fmi2Status_fmi2Error;

                            }
                        }
                        fmi2Status_fmi2OK
                    } else {
                        fmi2Status_fmi2Error
                    }
                }
                Err(_) => fmi2Status_fmi2Error,
            }
            // print!("fmi2SetReal: ");
            // println!("{:?}", x);
            // println!("{:?}, {:?}, {:?}", vr, nvr, value);

        }

        #[no_mangle]
        pub extern "C" fn fmi2SetString(
            _c: fmi2Component,
            _vr: *const fmi2ValueReference,
            _nvr: usize,
            _value: *const fmi2String,
        ) -> fmi2Status {
            fmi2Status_fmi2Error
        }

        #[no_mangle]
        pub extern "C" fn fmi2GetFMUstate(_c: fmi2Component, _FMUstate: *mut fmi2FMUstate) -> fmi2Status {
            fmi2Status_fmi2Error
        }
        #[no_mangle]
        pub extern "C" fn fmi2SetFMUstate(_c: fmi2Component, _FMUstate: fmi2FMUstate) -> fmi2Status {
            fmi2Status_fmi2Error
        }
        #[no_mangle]
        pub extern "C" fn fmi2FreeFMUstate(_c: fmi2Component, _FMUstate: *mut fmi2FMUstate) -> fmi2Status {
            fmi2Status_fmi2Error
        }
        #[no_mangle]
        pub extern "C" fn fmi2SerializedFMUstateSize(
            _c: fmi2Component,
            _FMUstate: fmi2FMUstate,
            _size: *mut usize,
        ) -> fmi2Status {
            fmi2Status_fmi2Error
        }
        #[no_mangle]
        pub extern "C" fn fmi2SerializeFMUstate(
            _c: fmi2Component,
            _FMUstate: fmi2FMUstate,
            _serializedState: *mut fmi2Byte,
            _size: usize,
        ) -> fmi2Status {
            fmi2Status_fmi2Error
        }
        #[no_mangle]
        pub extern "C" fn fmi2DeSerializeFMUstate(
            _c: fmi2Component,
            _serializedState: *const fmi2Byte,
            _size: usize,
            _FMUstate: *mut fmi2FMUstate,
        ) -> fmi2Status {
            fmi2Status_fmi2Error
        }
        #[no_mangle]
        pub extern "C" fn fmi2GetDirectionalDerivative(
            _c: fmi2Component,
            _vUnknown_ref: *const fmi2ValueReference,
            _nUnknown: usize,
            _vKnown_ref: *const fmi2ValueReference,
            _nKnown: usize,
            _dvKnown: *const fmi2Real,
            _dvUnknown: *mut fmi2Real,
        ) -> fmi2Status {
            fmi2Status_fmi2Error
        }
        #[no_mangle]
        pub extern "C" fn fmi2EnterEventMode(_c: fmi2Component) -> fmi2Status {
            fmi2Status_fmi2Error
        }
        #[no_mangle]
        pub extern "C" fn fmi2NewDiscreteStates(
            _c: fmi2Component,
            _fmi2eventInfo: *mut fmi2EventInfo,
        ) -> fmi2Status {
            fmi2Status_fmi2Error
        }
        #[no_mangle]
        pub extern "C" fn fmi2EnterContinuousTimeMode(_c: fmi2Component) -> fmi2Status {
            fmi2Status_fmi2Error
        }
        #[no_mangle]
        pub extern "C" fn fmi2CompletedIntegratorStep(
            _c: fmi2Component,
            _noSetFMUStatePriorToCurrentPoint: fmi2Boolean,
            _enterEventMode: *mut fmi2Boolean,
            _terminateSimulation: *mut fmi2Boolean,
        ) -> fmi2Status {
            fmi2Status_fmi2Error
        }
        #[no_mangle]
        pub extern "C" fn fmi2SetTime(_c: fmi2Component, _time: fmi2Real) -> fmi2Status {
            fmi2Status_fmi2Error
        }
        #[no_mangle]
        pub extern "C" fn fmi2SetContinuousStates(
            _c: fmi2Component,
            _x: *const fmi2Real,
            _nx: usize,
        ) -> fmi2Status {
            fmi2Status_fmi2Error
        }
        #[no_mangle]
        pub extern "C" fn fmi2GetDerivatives(
            _c: fmi2Component,
            _derivatives: *mut fmi2Real,
            _nx: usize,
        ) -> fmi2Status {
            fmi2Status_fmi2Error
        }
        #[no_mangle]
        pub extern "C" fn fmi2GetEventIndicators(
            _c: fmi2Component,
            _eventIndicators: *mut fmi2Real,
            _ni: usize,
        ) -> fmi2Status {
            fmi2Status_fmi2Error
        }
        #[no_mangle]
        pub extern "C" fn fmi2GetContinuousStates(
            _c: fmi2Component,
            _x: *mut fmi2Real,
            _nx: usize,
        ) -> fmi2Status {
            fmi2Status_fmi2Error
        }
        #[no_mangle]
        pub extern "C" fn fmi2GetNominalsOfContinuousStates(
            _c: fmi2Component,
            _x_nominal: *mut fmi2Real,
            _nx: usize,
        ) -> fmi2Status {
            fmi2Status_fmi2Error
        }
        #[no_mangle]
        pub extern "C" fn fmi2SetRealInputDerivatives(
            _c: fmi2Component,
            _vr: *const fmi2ValueReference,
            _nvr: usize,
            _order: *const fmi2Integer,
            _value: *const fmi2Real,
        ) -> fmi2Status {
            fmi2Status_fmi2Error
        }
        #[no_mangle]
        pub extern "C" fn fmi2GetRealOutputDerivatives(
            _c: fmi2Component,
            _vr: *const fmi2ValueReference,
            _nvr: usize,
            _order: *const fmi2Integer,
            _value: *mut fmi2Real,
        ) -> fmi2Status {
            fmi2Status_fmi2Error
        }

        #[no_mangle]
        pub extern "C" fn fmi2DoStep(
            c: fmi2Component,
            currentCommunicationPoint: fmi2Real,
            communicationStepSize: fmi2Real,
            _noSetFMUStatePriorToCurrentPoint: fmi2Boolean,
        ) -> fmi2Status {
            assert!(
                std::ptr::null() != c as *mut c_void,
                "fmi2GetReal: Null pointer passed"
            );

            // println!("fmi2DoStep: currentCommunicationPoint = {}, communicationStepSize = {}", currentCommunicationPoint, communicationStepSize);
            let model: &mut #model_name = #model_name::from_c_ptr(c as *mut c_void).expect("fmi2DoStep: Failed to get model from c_ptr");
            model.do_step(currentCommunicationPoint, communicationStepSize).map(|_| fmi2Status_fmi2OK).unwrap_or(fmi2Status_fmi2Error)
        }

        #[no_mangle]
        pub extern "C" fn fmi2CancelStep(_c: fmi2Component) -> fmi2Status {
            fmi2Status_fmi2Error
        }

        #[no_mangle]
        pub extern "C" fn fmi2GetStatus(
            _c: fmi2Component,
            _s: fmi2StatusKind,
            _value: *mut fmi2Status,
        ) -> fmi2Status {
            fmi2Status_fmi2Error
        }

        #[no_mangle]
        pub extern "C" fn fmi2GetRealStatus(
            _c: fmi2Component,
            _s: fmi2StatusKind,
            _value: *mut fmi2Real,
        ) -> fmi2Status {
            fmi2Status_fmi2Error
        }

        #[no_mangle]
        pub extern "C" fn fmi2GetIntegerStatus(
            _c: fmi2Component,
            _s: fmi2StatusKind,
            _value: *mut fmi2Integer,
        ) -> fmi2Status {
            fmi2Status_fmi2Error
        }

        #[no_mangle]
        pub extern "C" fn fmi2GetBooleanStatus(
            _c: fmi2Component,
            _s: fmi2StatusKind,
            _value: *mut fmi2Boolean,
        ) -> fmi2Status {
            fmi2Status_fmi2Error
        }

        #[no_mangle]
        pub extern "C" fn fmi2GetStringStatus(
            _c: fmi2Component,
            _s: fmi2StatusKind,
            _value: *mut fmi2String,
        ) -> fmi2Status {
            fmi2Status_fmi2Error
        }

        #[no_mangle]
        pub fn get_model_description() -> String {
            #model_name::to_model_description_xml()
        }

        #[no_mangle]
        pub fn get_model_name() -> &'static str {
            #model_name::model_name()
        }
        });
    }

    output.into()
}
