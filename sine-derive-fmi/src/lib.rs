extern crate num;
#[macro_use]
extern crate serde;

use serde::{Serialize, Deserialize};

#[macro_use]
extern crate num_derive;

#[macro_use]
extern crate fmi2;

#[macro_use]
extern crate fmi2_derive;

use fmi2_sys::*;
use std::any::TypeId;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};

use fmi2::derive::*;
use fmi2::{FMIErrors, FmiModelStructDerive};
use fmi2::{Instantiatable, Steppable};
// const fmi2True as fmi2Boolean: fmi2Boolean = fmi2True as fmi2Boolean;
// const FMI2FALSE: fmi2Boolean = fmi2False as fmi2Boolean;

use handlebars::Handlebars;
#[repr(C)]
#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub enum ModelState {
    Instantiated,
    Initialized,
    Terminated,
    Error,
}

#[repr(C)]
#[derive(Serialize, Deserialize)]
#[derive(FmiModelStructDerive, Debug)]
#[fmi_model(
    description = "How is it going?",
    guid = "{21d9f232-b090-4c79-933f-33da939b5934}",
    export = true
)]
pub struct SineModel {
    #[fmi_variable(id = 0, causality = "parameter", starting_value = 20.0, description = "Frequency in Hz", unit = "Hz")]
    frequency: f64,

    #[fmi_variable(causality = "input", description = "gain", starting_value = 1.0, unit = "V")]
    gain: f64,

    #[fmi_variable(causality = "output", description = "output", unit = "V")]
    output: f64,

    #[fmi_variable(causality = "independent", description = "elapsed time", unit = "s")]
    elapsed_time: f64,

    state: ModelState,

    loggingOn: fmi2Boolean,
}

impl Default for SineModel {
    fn default() -> SineModel {
        SineModel {
            frequency: 0.01,
            gain: 1.0,
            output: 0.0,
            elapsed_time: 0.0,
            state: ModelState::Instantiated,
            loggingOn: fmi2False as fmi2Boolean,
        }
    }
}

impl fmi2::Steppable for SineModel {
    fn do_step(&mut self, time: f64, step_size: f64) -> Result<(), FMIErrors> {
        // println!("SineModel::step: time = {}, step_size = {}", time, step_size);
        self.elapsed_time += step_size;
        self.output = (self.gain * (2.0 * std::f64::consts::PI * self.frequency * self.elapsed_time as f64)).sin();
        Ok(())
    }
}

impl fmi2::Instantiatable for SineModel {}

