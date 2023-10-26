#[macro_use]
pub extern crate fmi2_derive;

#[derive(Debug)]
pub enum FMIErrors { InvalidValueReference, Error }

use std::os::raw::{c_char, c_void};

pub mod derive {
    pub use fmi2_derive::*;
}

pub trait FmiModelStructDerive {
    fn get_real_by_value_reference(self: &Self, value_reference: u64) -> Option<f64>;
    fn get_bool_by_value_reference(self: &Self, value_reference: u64) -> Option<bool>;
    fn get_integer_by_value_reference(self: &Self, value_reference: u64) -> Option<i64>;

    fn set_real_by_value_reference(self: &mut Self, value_reference: u64, value: f64) -> Result<(),FMIErrors> ;
    fn set_integer_by_value_reference(self: &mut Self, value_reference: u64, value: i64) -> Result<(),FMIErrors> ;
    fn set_bool_by_value_reference(self: &mut Self, value_reference: u64, value: bool) -> Result<(),FMIErrors> ;
    
    fn guid() -> &'static str;
    fn description() -> &'static str;
    fn model_name() -> &'static str;
    fn to_model_description_xml() -> String;
}

pub trait Steppable 
where Self: Sized {
    fn do_step(&mut self, time: f64, step_size: f64) -> Result<(), FMIErrors>;
}

pub trait Instantiatable 
where Self: Sized + Default {
    fn instantiate() -> Result<Self, FMIErrors> {
        Ok(Self::default())
    }

    fn from_c_ptr<'a>(c_ptr: *mut c_void) -> Result<&'a mut Self, FMIErrors> {
        if c_ptr == std::ptr::null_mut() {
            return Err(FMIErrors::InvalidValueReference);
        } else {
            let x: &mut Self = unsafe { &mut *(c_ptr as *mut Self) };
            Ok(x)
        }
    }
}
