use fmi2_sys::*;
use std::ffi::{CString, CStr};
use std::os::raw::{c_char, c_void};

const VERSION: &str = "1.0";
const GUID: &str = "{21d9f232-b090-4c79-933f-33da939b5934}";

const FMI2TRUE: fmi2Boolean = fmi2True as fmi2Boolean;
const FMI2FALSE: fmi2Boolean = fmi2False as fmi2Boolean;

#[repr(C)]
enum ModelState {
    Instantiated,
    Initialized,
    Terminated,
    Error,
}

#[repr(C)]
struct SolverStruct {
}


#[repr(C)]
struct ModelInstance {
    instanceName: fmi2String,
    GUID: fmi2String,
    fmuType: fmi2Type,
    loggingOn: fmi2Boolean,
    synced: fmi2Boolean,
    jsynced: fmi2Boolean,
    state: ModelState,
	functions: fmi2CallbackFunctions,

    tolerance: fmi2Real,
    startTime: fmi2Real,
    stopTime: fmi2Real,
    stopTimeDefined: fmi2Boolean,

    initializeMode: fmi2Boolean,
}


pub extern "cdecl" fn fmi2GetVersion() -> *const c_char {
    // Create a c-string from a rust string and report it
    let version = CString::new(VERSION).unwrap();
    version.into_raw() 
}

pub extern "cdecl" fn fmi2GetTypesPlatform() -> *const c_char {
    // Create a c-string from a rust string and report it
    let version = CString::new("default").unwrap();
    version.into_raw() 
}

pub extern "cdecl" fn fmi2SetDebugLogging(
    c: *mut c_void, loggingOn: fmi2Boolean, nCategories: usize, categories: *const fmi2String) -> fmi2Status {
    let x: &mut ModelInstance = unsafe { &mut *(c as *mut ModelInstance) };

    
    // https://stackoverflow.com/questions/26117197/create-interface-to-c-function-pointers-in-rust
    // Create a c-string from a rust string and report it

    match x.functions.logger {
        Some(f) => {
            // TODO(cw): Create a wrapper around calling this function (and other callback functions)
            let category = CString::new("log").unwrap();
            let message = CString::new("fmi2SetDebugLogging: loggingOn = %d").unwrap();    
            unsafe {
                f(c, x.instanceName, fmi2Status_fmi2OK, category.into_raw(), message.into_raw(), loggingOn);
            }
        },
        None => {
            ();
        },
    }

    match x.state {
        ModelState::Error => {
            fmi2Status_fmi2Error
        },
        _ => {
            x.loggingOn = loggingOn;
            fmi2Status_fmi2OK
        },
    }
}

pub extern "cdecl" fn fmi2Instantiate(
    instanceName: fmi2String, fmuType: fmi2Type, fmuGUID: fmi2String, fmuResourceLocation: fmi2String,
    functions: fmi2CallbackFunctions, visible: fmi2Boolean, loggingOn: fmi2Boolean) -> *mut c_void {
    let mut x = Box::new(ModelInstance {
        instanceName: instanceName,
        GUID: fmuGUID,
        fmuType: fmuType,
        loggingOn: loggingOn,
        synced: FMI2FALSE,
        jsynced: FMI2FALSE,
        state: ModelState::Instantiated,
        functions: functions,

        tolerance: 0.0,
        startTime: 0.0,
        stopTime: 0.0,
        stopTimeDefined: FMI2FALSE,
        initializeMode: FMI2FALSE,
    });

    // https://stackoverflow.com/questions/26117197/create-interface-to-c-function-pointers-in-rust
    // Create a c-string from a rust string and report it
    let category = CString::new("log").unwrap();
    let message = CString::new("fmi2Instantiate: instanceName = %s").unwrap();    
    unsafe {
        x.functions.logger.unwrap()(x.as_mut() as *mut ModelInstance as *mut c_void, instanceName, fmi2Status_fmi2OK, category.into_raw(), message.into_raw(), instanceName);
    }

    Box::into_raw(x) as *mut c_void
}


// void fmi2FreeInstance(fmi2Component c) {
pub extern "cdecl" fn fmi2FreeInstance(c: fmi2Component) -> () {
    let mut x:Box<ModelInstance> = unsafe { Box::from_raw(c as *mut ModelInstance) };

    // https://stackoverflow.com/questions/26117197/create-interface-to-c-function-pointers-in-rust
    // Create a c-string from a rust string and report it
    let category = CString::new("log").unwrap();
    let message = CString::new("fmi2FreeInstance: instanceName = %s").unwrap();    
    unsafe {
        x.functions.logger.unwrap()(x.as_mut() as *mut ModelInstance as *mut c_void, x.instanceName, fmi2Status_fmi2OK, category.into_raw(), message.into_raw(), x.instanceName);
    }

    unsafe {
        x.functions.freeMemory.unwrap()(c);
    }
}