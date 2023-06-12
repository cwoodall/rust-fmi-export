use fmi2_sys::*;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};

extern crate num;
#[macro_use]
extern crate num_derive;

pub const VERSION: &str = "2.0";
pub const GUID: &str = "{21d9f232-b090-4c79-933f-33da939b5934}";

const FMI2TRUE: fmi2Boolean = fmi2True as fmi2Boolean;
const FMI2FALSE: fmi2Boolean = fmi2False as fmi2Boolean;

pub mod test {
    pub const VERSION: &str = "1.0";
}

#[repr(C)]
#[derive(Debug)]
pub enum ModelState {
    Instantiated,
    Initialized,
    Terminated,
    Error,
}

#[repr(C)]
#[derive(Debug)]
#[derive(FromPrimitive)]
enum ValueReferences {
    TimeElapsed = 0,
    Output = 1,
    Frequency = 2,
    Gain = 3
}

#[repr(C)]
#[derive(Debug)]
pub struct SolverStruct {}

#[repr(C)]
#[derive(Debug)]
pub struct ModelInstance {
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

    output: fmi2Real,
    time_elapsed: fmi2Real,
    frequency: fmi2Real,
    gain: fmi2Real,
}

#[no_mangle]
pub extern "C" fn fmi2GetVersion() -> *const c_char {
    // Create a c-string from a rust string and report it
    let version = CString::new(VERSION).unwrap();
    version.into_raw()
}

#[no_mangle]
pub extern "C" fn fmi2GetTypesPlatform() -> *const c_char {
    // Create a c-string from a rust string and report it
    let version = CString::new("default").unwrap();
    version.into_raw()
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

    
    let x: &mut ModelInstance = unsafe { &mut *(c as *mut ModelInstance) };

    // https://stackoverflow.com/questions/26117197/create-interface-to-c-function-pointers-in-rust
    // Create a c-string from a rust string and report it

    match x.functions.logger {
        Some(f) => {
            // TODO(cw): Create a wrapper around calling this function (and other callback functions)
            let category = CString::new("log").unwrap();
            let message = CString::new("fmi2SetDebugLogging: loggingOn = %d").unwrap();
            unsafe {
                f(
                    c,
                    x.instanceName,
                    fmi2Status_fmi2OK,
                    category.into_raw(),
                    message.into_raw(),
                    loggingOn,
                );
            }
        }
        None => {
            ();
        }
    }

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
) -> *mut ModelInstance {
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
    assert!(guid.to_str().unwrap() == GUID, "fmi2Instantiate: Invalid GUID");

    let mut x: Box<ModelInstance> = Box::new(ModelInstance {
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

        output: 0.0,
        time_elapsed: 0.0,
        frequency: 100.0,
        gain: 1.0
    });

    if x.loggingOn == FMI2TRUE {
        // // https://stackoverflow.com/questions/26117197/create-interface-to-c-function-pointers-in-rust
        // // Create a c-string from a rust string and report it
        let category = CString::new("log").unwrap();
        let message = CString::new("fmi2Instantiate: instanceName = %s").unwrap();
        unsafe {
            x.functions.logger.unwrap()(
                x.as_mut() as *mut ModelInstance as *mut c_void,
                instanceName,
                fmi2Status_fmi2OK,
                category.into_raw(),
                message.into_raw(),
                instanceName,
            );
        }
    }

    println!("{:?}", x);
    Box::into_raw(x) as *mut ModelInstance
}

#[no_mangle]
pub extern "C" fn fmi2FreeInstance(c: fmi2Component) -> () {
    assert!(
        std::ptr::null() != c as *mut c_void,
        "fmi2FreeInstance: Null pointer passed"
    );

    let x: &mut ModelInstance = unsafe { &mut *(c as *mut ModelInstance) };
    println!("{:?}", x);

    // https://stackoverflow.com/questions/26117197/create-interface-to-c-function-pointers-in-rust
    // Create a c-string from a rust string and report it
    if x.loggingOn == FMI2TRUE {
        let category = CString::new("log").unwrap();
        let message = CString::new("fmi2FreeInstance: instanceName = %s").unwrap();
        unsafe {
            x.functions.logger.unwrap()(
                c,
                x.instanceName,
                fmi2Status_fmi2OK,
                category.into_raw(),
                message.into_raw(),
                x.instanceName,
            );
        }
    }

    match x.functions.freeMemory {
        None => {
            ();
        }
        Some(f) => unsafe {
            f(c);
        },
    }
}

#[no_mangle]
pub extern "C" fn fmi2SetupExperiment(
    c: fmi2Component,
    _toleranceDefined: fmi2Boolean,
    _tolerance: fmi2Real,
    startTime: fmi2Real,
    stopTimeDefined: fmi2Boolean,
    stopTime: fmi2Real,
) -> fmi2Status {
    assert!(
        std::ptr::null() != c as *mut c_void,
        "fmi2FreeInstance: Null pointer passed"
    );

    let x: &mut ModelInstance = unsafe { &mut *(c as *mut ModelInstance) };
    println!("{:?}", x);

    x.startTime = startTime;
    x.stopTime = if stopTimeDefined == FMI2TRUE {
        stopTime
    } else {
        0.0
    };

    x.stopTimeDefined = stopTimeDefined;

    fmi2Status_fmi2OK
}

#[no_mangle]
pub extern "C" fn fmi2EnterInitializationMode(c: fmi2Component) -> fmi2Status {
    // Validate that c is not a nullptr
    if c != 0 as *mut c_void {
        let x: &mut ModelInstance = unsafe { &mut *(c as *mut ModelInstance) };

        x.initializeMode = FMI2TRUE;
        x.jsynced = FMI2FALSE;
        x.synced = FMI2FALSE;
        x.state = ModelState::Initialized;

        fmi2Status_fmi2OK
    } else {
        fmi2Status_fmi2Error
    }
}

#[no_mangle]
pub extern "C" fn fmi2ExitInitializationMode(c: fmi2Component) -> fmi2Status {
    // Validate that c is not a nullptr
    if c != 0 as *mut c_void {
        let x: &mut ModelInstance = unsafe { &mut *(c as *mut ModelInstance) };

        x.initializeMode = FMI2FALSE;
        x.jsynced = FMI2FALSE;
        fmi2Status_fmi2OK
    } else {
        fmi2Status_fmi2Error
    }
}

#[no_mangle]
pub extern "C" fn fmi2Terminate(c: fmi2Component) -> fmi2Status {
    let x: &mut ModelInstance = unsafe { &mut *(c as *mut ModelInstance) };

    x.state = ModelState::Terminated;
    fmi2Status_fmi2OK
}

#[no_mangle]
pub extern "C" fn fmi2Reset(c: fmi2Component) -> fmi2Status {
    if c != 0 as *mut c_void {
        let x: &mut ModelInstance = unsafe { &mut *(c as *mut ModelInstance) };

        x.state = ModelState::Instantiated;
        x.jsynced = FMI2FALSE;
        x.synced = FMI2FALSE;
        fmi2Status_fmi2OK
    } else {
        fmi2Status_fmi2Error
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

    let x: &mut ModelInstance = unsafe { &mut *(c as *mut ModelInstance) };

    print!("fmi2GetReal: ");
    println!("{:?}", x);
    println!("{:?}, {:?}, {:?}", vr, nvr, value);


    if nvr > 0 {
        let value_slice: &mut [f64] = unsafe { std::slice::from_raw_parts_mut(value, nvr) };
        let reference_slice: &[fmi2ValueReference] = unsafe { std::slice::from_raw_parts(vr, nvr) };
    

        for i in 0..nvr {
            let value_reference = num::FromPrimitive::from_u32(reference_slice[i]);
            match value_reference {
                Some(ValueReferences::TimeElapsed) => {
                    value_slice[i] = x.time_elapsed;
                },
                Some(ValueReferences::Output) => {
                    value_slice[i] = x.output;
                },
                Some(ValueReferences::Frequency) => {
                    value_slice[i] = x.frequency;
                },
                _ => {
                    println!("fmi2GetReal: Unknown value reference: {}", reference_slice[i]);
                    return fmi2Status_fmi2Error;
                }
            }
        }
        fmi2Status_fmi2OK
    } else {
        fmi2Status_fmi2Error
    }
}

#[no_mangle]
pub extern "C" fn fmi2GetInteger(
    _c: fmi2Component,
    _vr: *const fmi2ValueReference,
    _nvr: usize,
    _value: *mut fmi2Integer,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2GetBoolean(
    _c: fmi2Component,
    _vr: *const fmi2ValueReference,
    _nvr: usize,
    _value: *mut fmi2Boolean,
) -> fmi2Status {
    fmi2Status_fmi2Error
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

    let x: &mut ModelInstance = unsafe { &mut *(c as *mut ModelInstance) };

    print!("fmi2SetReal: ");
    println!("{:?}", x);
    println!("{:?}, {:?}, {:?}", vr, nvr, value);


    if nvr > 0 {
        let value_slice: &[f64] = unsafe { std::slice::from_raw_parts(value, nvr) };
        let reference_slice: &[fmi2ValueReference] = unsafe { std::slice::from_raw_parts(vr, nvr) };
    

        for i in 0..nvr {
            let value_reference = num::FromPrimitive::from_u32(reference_slice[i]);
            match value_reference {
                Some(ValueReferences::Frequency) => {
                    x.frequency = value_slice[i];
                },
                Some(ValueReferences::Gain) => {
                    x.gain = value_slice[i];
                },
                _ => {
                    println!("fmi2GetReal: Unknown value reference: {}", reference_slice[i]);
                    return fmi2Status_fmi2Error;
                }
            }
        }
        fmi2Status_fmi2OK
    } else {
        fmi2Status_fmi2Error
    }
}

#[no_mangle]
pub extern "C" fn fmi2SetInteger(
    _c: fmi2Component,
    _vr: *const fmi2ValueReference,
    _nvr: usize,
    _value: *const fmi2Integer,
) -> fmi2Status {
    fmi2Status_fmi2Error
}
#[no_mangle]
pub extern "C" fn fmi2SetBoolean(
    _c: fmi2Component,
    _vr: *const fmi2ValueReference,
    _nvr: usize,
    _value: *const fmi2Boolean,
) -> fmi2Status {
    fmi2Status_fmi2Error
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
    _currentCommunicationPoint: fmi2Real,
    communicationStepSize: fmi2Real,
    _noSetFMUStatePriorToCurrentPoint: fmi2Boolean,
) -> fmi2Status {
    assert!(
        std::ptr::null() != c as *mut c_void,
        "fmi2GetReal: Null pointer passed"
    );


    let x: &mut ModelInstance = unsafe { &mut *(c as *mut ModelInstance) };

    x.time_elapsed = x.time_elapsed + communicationStepSize;
    x.output = x.gain*(x.time_elapsed * x.frequency).sin();

    fmi2Status_fmi2OK
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

